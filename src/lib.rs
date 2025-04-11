use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use http_body_util::Empty;
use http_body_util::Full;
use http_body_util::{BodyExt, combinators::BoxBody};
use hyper::body::Body;
use hyper::body::Bytes;
use hyper::header::HeaderName;
use hyper::header::HeaderValue;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};

use hyper_util::rt::TokioIo;
use log::debug;
use log::error;
use log::info;
use prost::Message;
use snap::raw::Decoder;

use buffer_manager::BufferManager;
use database::Database;
use prometheus::WriteRequest;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::pin;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub mod prometheus {
    include!(concat!(env!("OUT_DIR"), "/prometheus.rs"));
}

pub mod buffer_manager;
pub mod database;
pub mod labels;
pub mod metrics_buffer;
pub mod owner_buffer;

static MAX_PAYLOAD_SIZE: u64 = 4 * 1024 * 1024;
static ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
static TIMEOUT: Duration = Duration::from_secs(10);

pub struct Server {
    buffer_manager: BufferManager,
    database: Database,
    shutdown_signal: Arc<Notify>,
}

impl Server {
    pub fn new(buffer_manager: BufferManager, database: Database) -> Self {
        Self {
            buffer_manager,
            database,
            shutdown_signal: Arc::new(Notify::new()),
        }
    }

    pub async fn run(
        self: Arc<Self>,
    ) -> Result<JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(ADDRESS).await?;

        Ok(tokio::task::spawn({
            let self_clone = self.clone();
            async move {
                info!("Accepting connections on {}", ADDRESS);
                if let Err(e) = self_clone.accept_loop(listener).await {
                    error!("Error in accept loop: {}", e);
                }
            }
        }))
    }

    pub fn shutdown(&self) {
        self.shutdown_signal.notify_one();
    }

    async fn accept_loop(
        self: Arc<Self>,
        listener: TcpListener,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            let self_clone = self.clone();
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((tcp, remote_address)) => {
                            self_clone.accept_single(tcp, remote_address).await
                        },
                        Err(e) => error!("Error accepting connection: {}", e),
                    }
                }
                _ = self_clone.shutdown_signal.notified() => {
                    info!("Shutdown signal received.");
                    break;
                }
            }
        }
        info!("Accept loop finished.");
        Ok(())
    }

    async fn accept_single(self: Arc<Self>, tcp: TcpStream, remote_address: SocketAddr) {
        debug!("Accepted connection from {:?}", remote_address);
        let io = TokioIo::new(tcp);
        tokio::task::spawn({
            let self_clone = self.clone();

            async move {
                let conn = http1::Builder::new()
                    .serve_connection(io, service_fn(|req| self_clone.receive_data(req)));
                pin!(conn);

                tokio::select! {
                    res = conn.as_mut() => {
                        match res {
                            Ok(()) => debug!("Connection served successfully"),
                            Err(e) => error!("Error serving connection: {:?}", e),
                        };
                    }
                    _ = tokio::time::sleep(TIMEOUT) => {
                        debug!("Timeout reached, shutting down connection");
                        conn.as_mut().graceful_shutdown();
                    }
                }
            }
        });
    }

    async fn receive_data(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        let upper = req.body().size_hint().upper().unwrap_or(u64::MAX);
        if upper > MAX_PAYLOAD_SIZE as u64 {
            let mut resp = Response::new(full("Body too big"));
            *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
            return Ok(resp);
        }

        let body_bytes = req.collect().await?.to_bytes();
        let mut decoder = Decoder::new();
        let decompressed_data = match decoder.decompress_vec(&body_bytes) {
            Ok(data) => data,
            Err(_) => {
                let mut resp = Response::new(full("Failed to decompress data"));
                *resp.status_mut() = hyper::StatusCode::BAD_REQUEST;
                return Ok(resp);
            }
        };

        let write_request = match WriteRequest::decode(&*decompressed_data) {
            Ok(req) => req,
            Err(_) => {
                let mut resp = Response::new(full("Failed to parse WriteRequest"));
                *resp.status_mut() = hyper::StatusCode::BAD_REQUEST;
                return Ok(resp);
            }
        };

        let (processed_samples, metrics_to_flush, owners_to_flush) =
            self.buffer_manager.process_write_request(write_request);

        if !metrics_to_flush.is_empty() {
            self.database.insert_metrics(metrics_to_flush);
        }

        if !owners_to_flush.is_empty() {
            self.database.insert_owners(owners_to_flush);
        }

        let mut resp = Response::new(empty());
        *resp.status_mut() = hyper::StatusCode::NO_CONTENT;
        if let Ok(header_value) = HeaderValue::from_str(&processed_samples.to_string()) {
            resp.headers_mut().insert(
                HeaderName::from_static("X-Prometheus-Remote-Write-Status"),
                header_value,
            );
        }
        return Ok(resp);
    }
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}
