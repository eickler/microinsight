use dashmap::DashMap;
use std::sync::{Arc, Mutex};

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct Key {
    pub timestamp: u64,
    pub environment: String,
    pub pod: String,
    pub container: String,
}

#[derive(Default, Clone, Debug)]
pub struct Metrics {
    pub cpu_usage_total: Option<f64>,
    pub cpu_usage: Option<f64>,
    pub cpu_limit: Option<f64>,
    pub memory_usage: Option<f64>,
    pub memory_limit: Option<f64>,
}

pub struct MetricsBuffer {
    interval: u64,
    max_delay: usize,
    buffer: DashMap<Key, Arc<Mutex<Metrics>>>,
}

impl MetricsBuffer {
    pub fn new(interval: u64, max_delay: usize) -> Self {
        MetricsBuffer {
            interval,
            max_delay,
            buffer: DashMap::new(),
        }
    }

    fn truncate_timestamp(&self, timestamp: u64) -> u64 {
        (timestamp / self.interval) * self.interval
    }

    pub fn insert(
        &self,
        name: &str,
        environment: &str,
        pod: &str,
        container: &str,
        timestamp: u64,
        value: f64,
    ) {
        let truncated_timestamp = self.truncate_timestamp(timestamp);
        let key = Key {
            timestamp: truncated_timestamp,
            environment: environment.to_string(),
            pod: pod.to_string(),
            container: container.to_string(),
        };

        // Prometheus remote write protocol specifies that metrics have to arrive in timestamp order
        // for their database to work -- fingers crossed!
        let mut previous_cpu_usage_total = Option::None;
        if name == "cpu_usage_total" {
            let previous_key = Key {
                timestamp: truncated_timestamp - self.interval,
                environment: key.environment.clone(),
                pod: key.pod.clone(),
                container: key.container.clone(),
            };

            if let Some(previous_entry) = self.buffer.get(&previous_key) {
                let previous_metrics = previous_entry.lock().unwrap();
                previous_cpu_usage_total = previous_metrics.cpu_usage_total;
            }
        }

        let entry = self
            .buffer
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Mutex::new(Metrics::default())));

        let mut metrics = entry.lock().unwrap();
        match name {
            "cpu_usage_total" => {
                metrics.cpu_usage_total = Some(value);
                if let Some(previous_value) = previous_cpu_usage_total {
                    if value >= previous_value {
                        metrics.cpu_usage = Some(value - previous_value);
                    }
                }
            }
            "cpu_limit" => metrics.cpu_limit = Some(value),
            "memory_usage" => metrics.memory_usage = Some(value),
            "memory_limit" => metrics.memory_limit = Some(value),
            _ => {}
        }
    }

    pub fn flush(&self) -> Vec<(Key, Metrics)> {
        let mut flushed = Vec::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let threshold = self.truncate_timestamp(now) - self.interval * self.max_delay as u64;

        self.buffer.retain(|key, value| {
            if key.timestamp < threshold {
                let metrics = value.lock().unwrap().clone();
                flushed.push((key.clone(), metrics));
                false
            } else {
                true
            }
        });

        flushed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_key(timestamp: u64) -> Key {
        Key {
            timestamp,
            environment: "env1".to_string(),
            pod: "pod1".to_string(),
            container: "container1".to_string(),
        }
    }

    #[test]
    fn test_insert_cpu_limit() {
        let buffer = MetricsBuffer::new(60, 5);
        let timestamp = 120;
        let value = 100.0;

        buffer.insert("cpu_limit", "env1", "pod1", "container1", timestamp, value);

        let key = create_key(buffer.truncate_timestamp(timestamp));
        let entry = buffer.buffer.get(&key).unwrap();
        let metrics = entry.lock().unwrap();

        assert_eq!(metrics.cpu_limit, Some(value));
    }

    #[test]
    fn test_insert_cpu_usage_no_previous() {
        let buffer = MetricsBuffer::new(60, 5);
        let timestamp = 120;
        let value = 100.0;

        buffer.insert(
            "cpu_usage_total",
            "env1",
            "pod1",
            "container1",
            timestamp,
            value,
        );

        let key = create_key(buffer.truncate_timestamp(timestamp));
        let entry = buffer.buffer.get(&key).unwrap();
        let metrics = entry.lock().unwrap();

        assert_eq!(metrics.cpu_usage, None);
        assert_eq!(metrics.cpu_usage_total, Some(value));
    }

    #[test]
    fn test_insert_cpu_usage_with_previous() {
        let buffer = MetricsBuffer::new(60, 5);
        let first_timestamp = 120;
        let first_value = 100.0;
        let second_timestamp = 180;
        let second_value = 150.0;

        buffer.insert(
            "cpu_usage_total",
            "env1",
            "pod1",
            "container1",
            first_timestamp,
            first_value,
        );
        buffer.insert(
            "cpu_usage_total",
            "env1",
            "pod1",
            "container1",
            second_timestamp,
            second_value,
        );

        let first_key = create_key(buffer.truncate_timestamp(first_timestamp));
        let second_key = create_key(buffer.truncate_timestamp(second_timestamp));

        let first_entry = buffer.buffer.get(&first_key).unwrap();
        let first_metrics = first_entry.lock().unwrap();
        assert_eq!(first_metrics.cpu_usage, None);
        assert_eq!(first_metrics.cpu_usage_total, Some(first_value));

        let second_entry = buffer.buffer.get(&second_key).unwrap();
        let second_metrics = second_entry.lock().unwrap();
        assert_eq!(second_metrics.cpu_usage, Some(second_value - first_value));
        assert_eq!(second_metrics.cpu_usage_total, Some(second_value));
    }

    #[test]
    fn test_insert_cpu_usage_with_previous_no_cpu_usage() {
        let buffer = MetricsBuffer::new(60, 5);
        let first_timestamp = 120;
        let first_value = 100.0;
        let second_timestamp = 180;
        let second_value = 150.0;

        buffer.insert(
            "memory_usage",
            "env1",
            "pod1",
            "container1",
            first_timestamp,
            first_value,
        );
        buffer.insert(
            "cpu_usage_total",
            "env1",
            "pod1",
            "container1",
            second_timestamp,
            second_value,
        );

        let first_key = create_key(buffer.truncate_timestamp(first_timestamp));
        let second_key = create_key(buffer.truncate_timestamp(second_timestamp));

        let first_entry = buffer.buffer.get(&first_key).unwrap();
        let first_metrics = first_entry.lock().unwrap();
        assert_eq!(first_metrics.memory_usage, Some(first_value));
        assert_eq!(first_metrics.cpu_usage, None);
        assert_eq!(first_metrics.cpu_usage_total, None);

        let second_entry = buffer.buffer.get(&second_key).unwrap();
        let second_metrics = second_entry.lock().unwrap();
        assert_eq!(second_metrics.cpu_usage, None);
        assert_eq!(second_metrics.cpu_usage_total, Some(second_value));
    }

    #[test]
    fn test_insert_cpu_usage_with_wrapping() {
        let buffer = MetricsBuffer::new(60, 5);
        let first_timestamp = 120;
        let first_value = 100.0;
        let second_timestamp = 180;
        let second_value = 50.0;

        buffer.insert(
            "cpu_usage_total",
            "env1",
            "pod1",
            "container1",
            first_timestamp,
            first_value,
        );
        buffer.insert(
            "cpu_usage_total",
            "env1",
            "pod1",
            "container1",
            second_timestamp,
            second_value,
        );

        let first_key = create_key(buffer.truncate_timestamp(first_timestamp));
        let second_key = create_key(buffer.truncate_timestamp(second_timestamp));

        let first_entry = buffer.buffer.get(&first_key).unwrap();
        let first_metrics = first_entry.lock().unwrap();
        assert_eq!(first_metrics.cpu_usage, None);
        assert_eq!(first_metrics.cpu_usage_total, Some(first_value));

        let second_entry = buffer.buffer.get(&second_key).unwrap();
        let second_metrics = second_entry.lock().unwrap();
        assert_eq!(second_metrics.cpu_usage, None);
        assert_eq!(second_metrics.cpu_usage_total, Some(second_value));
    }

    #[test]
    fn test_insert_memory_usage() {
        let buffer = MetricsBuffer::new(60, 5);
        let timestamp = 120;
        let value = 200.0;

        buffer.insert(
            "memory_usage",
            "env1",
            "pod1",
            "container1",
            timestamp,
            value,
        );

        let key = create_key(buffer.truncate_timestamp(timestamp));
        let entry = buffer.buffer.get(&key).unwrap();
        let metrics = entry.lock().unwrap();

        assert_eq!(metrics.memory_usage, Some(value));
    }

    #[test]
    fn test_flush_removes_old_entries() {
        let buffer = MetricsBuffer::new(60, 5);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let old_timestamp = now - 360; // Older than max_delay
        let recent_timestamp = now - 120; // Within max_delay

        buffer.insert(
            "cpu_usage_total",
            "env1",
            "pod1",
            "container1",
            old_timestamp,
            100.0,
        );
        buffer.insert(
            "cpu_usage_total",
            "env1",
            "pod1",
            "container1",
            recent_timestamp,
            200.0,
        );

        let flushed = buffer.flush();

        assert_eq!(flushed.len(), 1);
        assert_eq!(
            flushed[0].0.timestamp,
            buffer.truncate_timestamp(old_timestamp)
        );
        assert_eq!(buffer.buffer.len(), 1);
        assert!(buffer.buffer.contains_key(&Key {
            timestamp: buffer.truncate_timestamp(recent_timestamp),
            environment: "env1".to_string(),
            pod: "pod1".to_string(),
            container: "container1".to_string(),
        }));
    }

    #[test]
    fn test_flush_empty_buffer() {
        let buffer = MetricsBuffer::new(60, 5);
        let flushed = buffer.flush();
        assert!(flushed.is_empty());
    }
}
