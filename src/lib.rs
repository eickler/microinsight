use actix_web::{App, HttpResponse, HttpServer, Responder, middleware::Logger, web};
use actix_web_prometheus::PrometheusMetricsBuilder;
use buffer_manager::BufferManager;
use database::Database;
use prometheus::WriteRequest;
use prost::Message;
use snap::raw::Decoder;
use sysinfo::System;

pub mod prometheus {
    include!(concat!(env!("OUT_DIR"), "/prometheus.rs"));
}

pub mod buffer_manager;
pub mod database;
pub mod labels;
pub mod metrics_buffer;
pub mod owner_buffer;

static MAX_PAYLOAD_SIZE: usize = 4 * 1024 * 1024;

pub struct Server {
    buffer_manager: BufferManager,
    database: Database,
}

impl Server {
    pub fn new(buffer_manager: BufferManager, database: Database) -> Self {
        Self {
            buffer_manager,
            database,
        }
    }

    pub async fn run(self) -> std::io::Result<actix_web::dev::Server> {
        let server_data = web::Data::new(self);

        let prometheus = PrometheusMetricsBuilder::new("api")
            .endpoint("/metrics")
            .build()
            .unwrap();

        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::PayloadConfig::new(MAX_PAYLOAD_SIZE))
                .app_data(server_data.clone())
                .wrap(Logger::default())
                .wrap(prometheus.clone())
                .route("/health", web::get().to(health))
                .route("/receive", web::post().to(receive_data))
        })
        .bind("0.0.0.0:80")?
        .run();

        Ok(server)
    }
}

async fn health() -> impl Responder {
    let mut system = System::new_all();
    system.refresh_all();

    let memory_used = system.used_memory();
    let memory_total = system.total_memory();
    let cpu_usage = system.global_cpu_usage();

    HttpResponse::Ok().body(format!(
        r#"{{
            "status": "UP",
            "memory_used": {},
            "memory_total": {},
            "cpu_usage": "{:.2}%"
        }}"#,
        memory_used, memory_total, cpu_usage
    ))
}

async fn receive_data(server: web::Data<Server>, body: web::Bytes) -> impl Responder {
    let mut decoder = Decoder::new();
    let decompressed_data = match decoder.decompress_vec(&body) {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().body("Failed to decompress data"),
    };

    let write_request = match WriteRequest::decode(&*decompressed_data) {
        Ok(req) => req,
        Err(_) => return HttpResponse::BadRequest().body("Failed to parse WriteRequest"),
    };

    let (process_samples, metrics_to_flush, owners_to_flush) =
        server.buffer_manager.process_write_request(write_request);

    if !metrics_to_flush.is_empty() {
        server.database.insert_metrics(metrics_to_flush);
    }

    if !owners_to_flush.is_empty() {
        server.database.insert_owners(owners_to_flush);
    }

    HttpResponse::NoContent()
        .insert_header((
            "X-Prometheus-Remote-Write-Samples-Written",
            process_samples.to_string(),
        ))
        .finish()
}
