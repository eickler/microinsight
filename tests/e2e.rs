use microinsight::metrics_buffer::MetricsBuffer;
use microinsight::owner_buffer::OwnerBuffer;
use microinsight::prometheus::{Label, Sample, TimeSeries, WriteRequest};
use microinsight::{Server, buffer_manager::BufferManager, database::Database};
use mysql::prelude::*;
use prost::Message;
use reqwest::Client;
use snap::raw::Encoder;
use std::time::{Duration, SystemTime};
use testcontainers::ImageExt;
use testcontainers_modules::{mariadb, testcontainers::runners::AsyncRunner};

#[tokio::test]
async fn test_receive_data_e2e() {
    let mysql_instance = mariadb::Mariadb::default()
        .with_env_var("MARIADB_ALLOW_EMPTY_ROOT_PASSWORD", "0")
        .start()
        .await
        .unwrap();
    let host = mysql_instance.get_host().await.unwrap();
    let port = mysql_instance.get_host_port_ipv4(3306).await.unwrap();
    let db_url = format!("mysql://root:test@{}:{}/test", host, port);
    let database = Database::new(&db_url, 5000);
    database.create_tables();
    let metrics_buffer = MetricsBuffer::new(60000, 5);
    let owner_buffer = OwnerBuffer::new(300, SystemTime::UNIX_EPOCH);
    let buffer_manager = BufferManager::new(metrics_buffer, owner_buffer);

    let server = Server::new(buffer_manager, database);
    let server_handle = tokio::spawn(server.run().await.expect("Failed to start server"));
    tokio::time::sleep(Duration::from_secs(5)).await;

    let write_request = WriteRequest {
        timeseries: vec![TimeSeries {
            labels: vec![
                Label {
                    name: "cluster".to_string(),
                    value: "prod".to_string(),
                },
                Label {
                    name: "pod".to_string(),
                    value: "pod-1".to_string(),
                },
                Label {
                    name: "container".to_string(),
                    value: "container-1".to_string(),
                },
                Label {
                    name: "__name__".to_string(),
                    value: "container_memory_working_set_bytes".to_string(),
                },
            ],
            samples: vec![Sample {
                value: 0.5,
                timestamp: 1234567890,
            }],
            exemplars: vec![],
            histograms: vec![],
        }],
        metadata: vec![],
    };

    let mut buf = Vec::new();
    write_request
        .encode(&mut buf)
        .expect("Failed to encode WriteRequest");
    let compressed_payload = Encoder::new()
        .compress_vec(&buf)
        .expect("Failed to compress payload");

    let client = Client::new();
    let response = client
        .post("http://127.0.0.1:80/receive")
        .body(compressed_payload)
        .header("Content-Encoding", "snappy")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 204);

    let url = format!("mysql://root:test@{}/test", &db_url);
    let opts = mysql::Opts::from_url(&url).expect("Invalid database URL");
    let pool = mysql::Pool::new(opts).expect("Failed to create database pool");
    let mut conn = pool.get_conn().unwrap();
    let result: Option<(String,)> = conn
        .query_first("SELECT * FROM micrometrics LIMIT 1")
        .unwrap();

    assert!(result.is_some());

    server_handle.abort();
}
