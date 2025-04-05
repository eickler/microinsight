use crate::metrics_buffer::{Key, Metrics};
use mysql::prelude::*;
use mysql::*;
use std::sync::Mutex;

pub struct Database {
    pool: Mutex<Pool>,
    chunk_size: usize,
}

impl Database {
    pub fn new(url: &str, chunk_size: usize) -> Self {
        let pool = Pool::new(url).expect("Failed to create database pool");
        Database {
            pool: Mutex::new(pool),
            chunk_size,
        }
    }

    pub fn create_tables(&self) {
        let mut conn = self
            .pool
            .lock()
            .unwrap()
            .get_conn()
            .expect("Failed to get connection");
        conn.query_drop(
            r"CREATE TABLE IF NOT EXISTS micrometrics (
                time TIMESTAMP,
                environment VARCHAR(255),
                pod VARCHAR(255),
                container VARCHAR(255),
                cpu_usage FLOAT,
                cpu_limit FLOAT,
                memory_usage FLOAT,
                memory_limit FLOAT,
                PRIMARY KEY (time, environment, pod, container)
            )",
        )
        .expect("Failed to create micrometrics table");

        conn.query_drop(
            r"CREATE TABLE IF NOT EXISTS microowner (
                environment VARCHAR(255),
                pod VARCHAR(255),
                owner VARCHAR(255),
                PRIMARY KEY (environment, pod)
            )",
        )
        .expect("Failed to create microowner table");
    }

    pub fn insert_metrics(&self, metrics: Vec<(Key, Metrics)>) {
        let mut conn = self
            .pool
            .lock()
            .unwrap()
            .get_conn()
            .expect("Failed to get connection");
        let query = r"INSERT INTO micrometrics
            (time, environment, pod, container, cpu_usage, cpu_limit, memory_usage, memory_limit)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
            cpu_usage = VALUES(cpu_usage),
            cpu_limit = VALUES(cpu_limit),
            memory_usage = VALUES(memory_usage),
            memory_limit = VALUES(memory_limit)";

        let insert_values: Vec<_> = metrics
            .into_iter()
            .filter_map(|(key, metrics)| {
                chrono::DateTime::from_timestamp_millis(key.timestamp as i64).map(|timestamp| {
                    (
                        timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                        key.environment,
                        key.pod,
                        key.container,
                        metrics.cpu_usage,
                        metrics.cpu_limit,
                        metrics.memory_usage,
                        metrics.memory_limit,
                    )
                })
            })
            .collect();

        for chunk in insert_values.chunks(self.chunk_size) {
            if let Err(e) = conn.exec_batch(query, chunk) {
                eprintln!("Error inserting metrics: {}", e);
            }
        }
    }

    pub fn insert_owners(&self, owners: Vec<(String, String, String)>) {
        let mut conn = self
            .pool
            .lock()
            .unwrap()
            .get_conn()
            .expect("Failed to get connection");
        let query = r"INSERT IGNORE INTO microowner (environment, pod, owner)
                      VALUES (?, ?, ?)";

        if let Err(e) = conn.exec_batch(query, owners) {
            eprintln!("Error inserting owners: {}", e);
        }
    }
}
