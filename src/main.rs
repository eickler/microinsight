use std::time::SystemTime;

use microinsight::{
    Server, buffer_manager::BufferManager, database::Database, metrics_buffer::MetricsBuffer,
    owner_buffer::OwnerBuffer,
};

fn init_logging() {
    let log_level = std::env::var("LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string())
        .to_lowercase();

    env_logger::builder()
        .filter_level(log_level.parse().unwrap_or(log::LevelFilter::Info))
        .init();
}

fn init_db() -> Database {
    let db_host = std::env::var("DB_HOST").expect("DB_HOST must be set");
    let db_user = std::env::var("DB_USER").expect("DB_USER must be set");
    let db_pass = std::env::var("DB_PASS").expect("DB_PASS must be set");
    let db_name = std::env::var("DB_NAME").expect("DB_NAME must be set");
    let chunk_size = std::env::var("CHUNK_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);

    let db_url = format!("mysql://{}:{}@{}/{}", db_user, db_pass, db_host, db_name);
    let database = Database::new(&db_url, chunk_size);
    database.create_tables();
    database
}

fn init_buffers() -> BufferManager {
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
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    init_logging();
    let database = init_db();
    let buffer_manager = init_buffers();

    let server = Server::new(buffer_manager, database);
    server.run().await?.await?;
    Ok(())
}
