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
        .with_env_var("MARIADB_ROOT_PASSWORD", "test")
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
                    name: "resource".to_string(),
                    value: "memory".to_string(),
                },
                Label {
                    name: "__name__".to_string(),
                    value: "kube_pod_container_resource_limits".to_string(),
                },
            ],
            samples: vec![Sample {
                value: 0.5,
                timestamp: 60000, // Funny enough, Jan 1, 1970 0:00:00 UTC is not a valid timestamp in MySQL
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

    let opts = mysql::Opts::from_url(&db_url).expect("Invalid database URL");
    let pool = mysql::Pool::new(opts).expect("Failed to create database pool");
    let mut conn = pool.get_conn().unwrap();
    let result: Option<(String, String, String, Option<f32>, Option<f32>, Option<f32>, Option<f32>)> = conn
        .query_first("SELECT environment, pod, container, cpu_usage, cpu_limit, memory_usage, memory_limit FROM micrometrics LIMIT 1")
        .unwrap();

    assert!(result.is_some());
    let (environment, pod, container, _cpu_usage, _cpu_limit, _memory_usage, memory_limit) =
        result.unwrap();
    assert_eq!(environment, "prod");
    assert_eq!(pod, "pod-1");
    assert_eq!(container, "container-1");
    assert_eq!(memory_limit, Some(0.5));

    server_handle.abort();
}
