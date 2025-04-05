use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use log::LevelFilter;
use metrics_buffer::MetricsBuffer;
use once_cell::sync::Lazy;
use owner_buffer::OwnerBuffer;
use prost::Message;
use snap::raw::Decoder;

use database::Database;
use labels::map;
use microinsight::prometheus::WriteRequest;

mod database;
mod labels;
mod metrics_buffer;
mod owner_buffer;

static BUFFER: Lazy<MetricsBuffer> = Lazy::new(|| {
    let interval = std::env::var("INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);

    let max_delay = std::env::var("MAX_DELAY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    MetricsBuffer::new(interval * 1000, max_delay)
});

static OWNER_BUFFER: Lazy<OwnerBuffer> = Lazy::new(|| {
    let owner_flush_interval = std::env::var("OWNER_FLUSH_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    OwnerBuffer::new(owner_flush_interval)
});

static DATABASE: Lazy<Database> = Lazy::new(|| {
    let db_host = std::env::var("DB_HOST").expect("DB_HOST environment variable must be set");
    let db_user = std::env::var("DB_USER").expect("DB_USER environment variable must be set");
    let db_pass = std::env::var("DB_PASS").expect("DB_PASS environment variable must be set");
    let db_name = std::env::var("DB_NAME").expect("DB_NAME environment variable must be set");
    let chunk_size = std::env::var("CHUNK_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);

    let db_url = format!("mysql://{}:{}@{}/{}", db_user, db_pass, db_host, db_name);
    let db = Database::new(&db_url, chunk_size);
    db.create_tables();
    db
});

async fn health() -> impl Responder {
    HttpResponse::Ok().json("{ \"status\" : \"UP\" }")
}

async fn receive_data(body: web::Bytes) -> impl Responder {
    let mut decoder = Decoder::new();
    let decompressed_data = match decoder.decompress_vec(&body) {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().body("Failed to decompress data"),
    };

    let write_request = match WriteRequest::decode(&*decompressed_data) {
        Ok(req) => req,
        Err(_) => return HttpResponse::BadRequest().body("Failed to parse WriteRequest"),
    };

    let (total_samples, metrics_to_flush, owners_to_flush) = to_buffers(write_request);

    if !metrics_to_flush.is_empty() {
        DATABASE.insert_metrics(metrics_to_flush);
    }

    if !owners_to_flush.is_empty() {
        DATABASE.insert_owners(owners_to_flush);
    }

    HttpResponse::NoContent()
        .insert_header((
            "X-Prometheus-Remote-Write-Samples-Written",
            total_samples.to_string(),
        ))
        .finish()
}

fn to_buffers(
    write_request: WriteRequest,
) -> (
    usize,
    Vec<(metrics_buffer::Key, metrics_buffer::Metrics)>,
    Vec<(String, String, String)>,
) {
    let mut total_samples = 0;

    for ts in write_request.timeseries {
        total_samples += ts.samples.len();
        if let Some(labels) = map(&ts.labels) {
            if labels.name.as_deref() == Some("owner") {
                if let (Some(environment), Some(pod), Some(owner)) = (
                    labels.environment.as_deref(),
                    labels.pod.as_deref(),
                    labels.owner.as_deref(),
                ) {
                    OWNER_BUFFER.insert(environment, pod, owner);
                }
                continue;
            }

            for sample in ts.samples {
                if sample.value.is_nan() {
                    continue;
                }

                if let Some(name) = &labels.name {
                    BUFFER.insert(
                        name,
                        labels.environment.as_deref().unwrap_or(""),
                        labels.pod.as_deref().unwrap_or(""),
                        labels.container.as_deref().unwrap_or(""),
                        sample.timestamp as u64,
                        sample.value,
                    );
                }
            }
        }
    }

    let flushed_data = BUFFER.flush();
    let owners = OWNER_BUFFER.flush();
    (total_samples, flushed_data, owners)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::builder().filter_level(LevelFilter::Info).init();

    HttpServer::new(|| {
        App::new()
            .route("/health", web::get().to(health))
            .route("/receive", web::post().to(receive_data))
    })
    .bind("0.0.0.0:80")?
    .run()
    .await
}
