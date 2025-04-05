use std::time::SystemTime;

use actix_web::middleware::Logger;
use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use actix_web_prometheus::PrometheusMetricsBuilder;
use log::LevelFilter;
use once_cell::sync::Lazy;
use prost::Message;
use snap::raw::Decoder;
use sysinfo::System;

use buffer_manager::BufferManager;
use database::Database;
use metrics_buffer::MetricsBuffer;
use microinsight::prometheus::WriteRequest;
use owner_buffer::OwnerBuffer;

mod buffer_manager;
mod database;
mod labels;
mod metrics_buffer;
mod owner_buffer;

static MAX_PAYLOAD_SIZE: usize = 4 * 1024 * 1024;

static BUFFER_MANAGER: Lazy<BufferManager> = Lazy::new(|| {
    let metrics_interval = std::env::var("INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);

    let metrics_max_delay = std::env::var("MAX_DELAY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

    let owner_flush_interval = std::env::var("OWNER_FLUSH_INTERVAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);

    let metrics_buffer = MetricsBuffer::new(metrics_interval * 1000, metrics_max_delay);
    let owner_buffer = OwnerBuffer::new(owner_flush_interval, SystemTime::now());

    BufferManager::new(metrics_buffer, owner_buffer)
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

    let (process_samples, metrics_to_flush, owners_to_flush) =
        BUFFER_MANAGER.process_write_request(write_request);

    if !metrics_to_flush.is_empty() {
        DATABASE.insert_metrics(metrics_to_flush);
    }

    if !owners_to_flush.is_empty() {
        DATABASE.insert_owners(owners_to_flush);
    }

    HttpResponse::NoContent()
        .insert_header((
            "X-Prometheus-Remote-Write-Samples-Written",
            process_samples.to_string(),
        ))
        .finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let log_level = std::env::var("LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string())
        .to_lowercase();

    env_logger::builder()
        .filter_level(log_level.parse().unwrap_or(LevelFilter::Info))
        .init();

    let prometheus = PrometheusMetricsBuilder::new("api")
        .endpoint("/metrics")
        .build()
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(prometheus.clone())
            .route("/health", web::get().to(health))
            .route("/receive", web::post().to(receive_data))
            .app_data(web::PayloadConfig::new(MAX_PAYLOAD_SIZE))
    })
    .bind("0.0.0.0:80")?
    .run()
    .await
}
