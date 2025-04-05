#![cfg(feature = "e2e-tests")]

use microinsight::prometheus::{Label, Sample, TimeSeries, WriteRequest};
use prost::Message;
use reqwest::Client;
use snap::raw::Encoder;
use std::time::Duration;
use testcontainers_modules::{mariadb, testcontainers::runners::SyncRunner};

#[tokio::test]
async fn test_receive_data_e2e() {
    let mariadb_instance = mariadb::Mariadb::default().start().unwrap();
    let mariadb_url = format!(
        "{}:{}",
        mariadb_instance.get_host().unwrap(),
        mariadb_instance.get_host_port_ipv4(3306).unwrap(),
    );

    std::env::set_var("DB_HOST", &mariadb_url);
    std::env::set_var("DB_USER", "root");
    std::env::set_var("DB_PASS", "test");
    std::env::set_var("DB_NAME", "test");

    let app_handle = tokio::spawn(async {
        crate::main().await.unwrap();
    });
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

    let pool = mysql::Pool::new(format!("mysql://root:test@{}/test", db_host));
    let mut conn = pool.get_conn().unwrap();
    let result: Option<(String,)> = conn
        .query_first("SELECT * FROM micrometrics LIMIT 1")
        .unwrap();

    assert!(result.is_some());

    app_handle.abort();
}
