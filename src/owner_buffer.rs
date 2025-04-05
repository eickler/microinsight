use dashmap::DashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct OwnerKey {
    pub environment: String,
    pub pod: String,
}

#[derive(Clone, Debug)]
pub struct OwnerValue {
    pub owner: String,
}

pub struct OwnerBuffer {
    buffer: DashMap<OwnerKey, OwnerValue>,
    last_flush: Arc<Mutex<SystemTime>>,
    flush_interval: Duration,
}

impl OwnerBuffer {
    pub fn new(flush_interval_secs: u64, last_flush: SystemTime) -> Self {
        OwnerBuffer {
            buffer: DashMap::new(),
            last_flush: Arc::new(Mutex::new(last_flush)),
            flush_interval: Duration::from_secs(flush_interval_secs),
        }
    }

    pub fn insert(&self, environment: &str, pod: &str, owner: &str) {
        let key = OwnerKey {
            environment: environment.to_string(),
            pod: pod.to_string(),
        };
        let value = OwnerValue {
            owner: owner.to_string(),
        };
        self.buffer.insert(key, value);
    }

    pub fn flush(&self) -> Vec<(String, String, String)> {
        let mut flushed = Vec::new();
        let now = SystemTime::now();

        let mut last_flush = self.last_flush.lock().unwrap();
        if now.duration_since(*last_flush).unwrap_or_default() >= self.flush_interval {
            *last_flush = now;

            self.buffer.retain(|key, value| {
                flushed.push((
                    key.environment.clone(),
                    key.pod.clone(),
                    value.owner.clone(),
                ));
                false
            });
        }

        flushed
    }
}
