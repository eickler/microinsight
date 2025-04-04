use dashmap::DashMap;
use std::sync::Arc;

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

pub struct Buffer {
    interval: u64,
    max_delay: usize,
    buffer: DashMap<Key, Arc<std::sync::Mutex<Metrics>>>,
}

impl Buffer {
    pub fn new(interval: u64, max_delay: usize) -> Self {
        Buffer {
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

        // Deadlock safety:
        // We always first lock the current value and then the previous value.
        // If the previous value is already locked, this is either locked due to
        // a writing a different property, or due to the same property. If it's
        // due to the same property, it will eventually unlock when the start of
        // the buffer is reached.

        let entry = self
            .buffer
            .entry(key.clone())
            .or_insert_with(|| Arc::new(std::sync::Mutex::new(Metrics::default())));

        let mut metrics = entry.lock().unwrap();
        if name == "cpu_usage" {
            let previous_key = Key {
                timestamp: truncated_timestamp - self.interval,
                environment: key.environment.clone(),
                pod: key.pod.clone(),
                container: key.container.clone(),
            };

            if let Some(previous_entry) = self.buffer.get(&previous_key) {
                let previous_metrics = previous_entry.lock().unwrap();
                if let Some(prev_value) = previous_metrics.cpu_usage_total {
                    if value >= prev_value {
                        metrics.cpu_usage = Some(value - prev_value);
                    }
                }
            }
            metrics.cpu_usage_total = Some(value);
        } else {
            match name {
                "cpu_limit" => metrics.cpu_limit = Some(value),
                "memory_usage" => metrics.memory_usage = Some(value),
                "memory_limit" => metrics.memory_limit = Some(value),
                _ => {}
            }
        }
    }

    pub fn flush(&self) -> Vec<(Key, Metrics)> {
        let mut flushed = Vec::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let threshold = self.truncate_timestamp(now) - self.interval * self.max_delay as u64;

        self.buffer.retain(|key, _| {
            if key.timestamp < threshold {
                if let Some((key, value)) = self.buffer.remove(key) {
                    flushed.push((key, Arc::try_unwrap(value).unwrap().into_inner().unwrap()));
                }
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
        let buffer = Buffer::new(60, 5);
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
        let buffer = Buffer::new(60, 5);
        let timestamp = 120;
        let value = 100.0;

        buffer.insert("cpu_usage", "env1", "pod1", "container1", timestamp, value);

        let key = create_key(buffer.truncate_timestamp(timestamp));
        let entry = buffer.buffer.get(&key).unwrap();
        let metrics = entry.lock().unwrap();

        assert_eq!(metrics.cpu_usage, None);
        assert_eq!(metrics.cpu_usage_total, Some(value));
    }

    #[test]
    fn test_insert_cpu_usage_with_previous() {
        let buffer = Buffer::new(60, 5);
        let first_timestamp = 120;
        let first_value = 100.0;
        let second_timestamp = 180;
        let second_value = 150.0;

        buffer.insert(
            "cpu_usage",
            "env1",
            "pod1",
            "container1",
            first_timestamp,
            first_value,
        );
        buffer.insert(
            "cpu_usage",
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
    fn test_insert_cpu_usage_with_wrapping() {
        let buffer = Buffer::new(60, 5);
        let first_timestamp = 120;
        let first_value = 100.0;
        let second_timestamp = 180;
        let second_value = 50.0; // Simulate wrapping

        buffer.insert(
            "cpu_usage",
            "env1",
            "pod1",
            "container1",
            first_timestamp,
            first_value,
        );
        buffer.insert(
            "cpu_usage",
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
        assert_eq!(second_metrics.cpu_usage, None); // Wrapping detected
        assert_eq!(second_metrics.cpu_usage_total, Some(second_value));
    }

    #[test]
    fn test_insert_memory_usage() {
        let buffer = Buffer::new(60, 5);
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
        let buffer = Buffer::new(60, 5);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let old_timestamp = now - 360; // Older than max_delay
        let recent_timestamp = now - 120; // Within max_delay

        buffer.insert(
            "cpu_usage",
            "env1",
            "pod1",
            "container1",
            old_timestamp,
            100.0,
        );
        buffer.insert(
            "cpu_usage",
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
        let buffer = Buffer::new(60, 5);
        let flushed = buffer.flush();
        assert!(flushed.is_empty());
    }
}
