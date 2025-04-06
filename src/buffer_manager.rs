use crate::labels::map;
use crate::metrics_buffer::{Key as MetricsKey, Metrics, MetricsBuffer};
use crate::owner_buffer::OwnerBuffer;
use crate::prometheus::WriteRequest;
use log::debug;

pub struct BufferManager {
    metrics_buffer: MetricsBuffer,
    owner_buffer: OwnerBuffer,
}

impl BufferManager {
    pub fn new(metrics_buffer: MetricsBuffer, owner_buffer: OwnerBuffer) -> Self {
        Self {
            metrics_buffer,
            owner_buffer,
        }
    }

    pub fn process_write_request(
        &self,
        write_request: WriteRequest,
    ) -> (
        usize,
        Vec<(MetricsKey, Metrics)>,
        Vec<(String, String, String)>,
    ) {
        let mut total_samples = 0;

        debug!(
            "Starting to process write request with {} timeseries",
            write_request.timeseries.len()
        );

        for ts in write_request.timeseries {
            if log::log_enabled!(log::Level::Debug) {
                debug!(
                    "Starting to process {} samples (name={:?}, container={:?}, owner={:?}",
                    ts.samples.len(),
                    ts.labels
                        .iter()
                        .find(|label| label.name == "__name__")
                        .map(|label| &label.value),
                    ts.labels
                        .iter()
                        .find(|label| label.name == "container_label_io_kubernetes_container_name")
                        .map(|label| &label.value),
                    ts.labels
                        .iter()
                        .find(|label| label.name == "label_owner")
                        .map(|label| &label.value),
                );
            }

            total_samples += ts.samples.len();
            if let Some(labels) = map(&ts.labels) {
                let environment = match labels.environment.as_deref() {
                    Some(env) => env,
                    None => continue,
                };
                let pod = match labels.pod.as_deref() {
                    Some(p) => p,
                    None => continue,
                };
                let name = match labels.name.as_deref() {
                    Some(n) => n,
                    None => continue,
                };

                if name == "owner" {
                    if let Some(owner) = labels.owner.as_deref() {
                        self.owner_buffer.insert(environment, pod, owner);
                    }
                    continue;
                }

                let container = match labels.container.as_deref() {
                    Some(c) => c,
                    None => continue,
                };

                debug!(
                    "Processing {} samples for processed labels: {:?}",
                    ts.samples.len(),
                    labels
                );

                for sample in ts.samples {
                    if sample.value.is_nan() {
                        continue;
                    }

                    self.metrics_buffer.insert(
                        name,
                        environment,
                        pod,
                        container,
                        sample.timestamp as u64,
                        sample.value,
                    );
                }
            }
        }

        let flushed_metrics = self.metrics_buffer.flush();
        let flushed_owners = self.owner_buffer.flush();
        (total_samples, flushed_metrics, flushed_owners)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics_buffer::MetricsBuffer;
    use crate::owner_buffer::OwnerBuffer;
    use crate::prometheus::{Label, Sample, TimeSeries, WriteRequest};
    use std::time::SystemTime;

    #[test]
    fn test_process_write_request_with_valid_metrics() {
        let metrics_buffer = MetricsBuffer::new(60000, 5);
        let owner_buffer = OwnerBuffer::new(300, SystemTime::UNIX_EPOCH);
        let buffer_manager = BufferManager::new(metrics_buffer, owner_buffer);

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

        let (total_samples, flushed_metrics, flushed_owners) =
            buffer_manager.process_write_request(write_request);

        assert_eq!(total_samples, 1);
        assert_eq!(flushed_metrics.len(), 1);
        assert!(flushed_owners.is_empty());
    }

    #[test]
    fn test_process_write_request_with_owner_label() {
        let metrics_buffer = MetricsBuffer::new(60000, 5);
        let owner_buffer = OwnerBuffer::new(300, SystemTime::UNIX_EPOCH);
        let buffer_manager = BufferManager::new(metrics_buffer, owner_buffer);

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
                        name: "__name__".to_string(),
                        value: "kube_pod_labels".to_string(),
                    },
                    Label {
                        name: "label_owner".to_string(),
                        value: "team-a".to_string(),
                    },
                ],
                samples: vec![],
                exemplars: vec![],
                histograms: vec![],
            }],
            metadata: vec![],
        };

        // Process the write request.
        let (total_samples, flushed_metrics, flushed_owners) =
            buffer_manager.process_write_request(write_request);

        // Verify the results.
        assert_eq!(total_samples, 0);
        assert!(flushed_metrics.is_empty());
        assert_eq!(
            flushed_owners[0],
            (
                "prod".to_string(),
                "pod-1".to_string(),
                "team-a".to_string()
            )
        );
    }

    #[test]
    fn test_process_write_request_with_missing_fields() {
        let metrics_buffer = MetricsBuffer::new(60000, 5);
        let owner_buffer = OwnerBuffer::new(300, SystemTime::UNIX_EPOCH);
        let buffer_manager = BufferManager::new(metrics_buffer, owner_buffer);

        let write_request = WriteRequest {
            timeseries: vec![TimeSeries {
                labels: vec![
                    Label {
                        name: "cluster".to_string(),
                        value: "prod".to_string(),
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

        let (total_samples, flushed_metrics, flushed_owners) =
            buffer_manager.process_write_request(write_request);

        assert_eq!(total_samples, 1);
        assert!(flushed_metrics.is_empty());
        assert!(flushed_owners.is_empty());
    }

    #[test]
    fn test_process_write_request_with_nan_sample() {
        let metrics_buffer = MetricsBuffer::new(60000, 5);
        let owner_buffer = OwnerBuffer::new(300, SystemTime::UNIX_EPOCH);
        let buffer_manager = BufferManager::new(metrics_buffer, owner_buffer);

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
                    value: f64::NAN,
                    timestamp: 1234567890,
                }],
                exemplars: vec![],
                histograms: vec![],
            }],
            metadata: vec![],
        };

        let (total_samples, flushed_metrics, flushed_owners) =
            buffer_manager.process_write_request(write_request);

        assert_eq!(total_samples, 1);
        assert!(flushed_metrics.is_empty());
        assert!(flushed_owners.is_empty());
    }
}
