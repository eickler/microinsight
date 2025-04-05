use microinsight::prometheus::Label;
use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Default, Debug, PartialEq)]
pub struct MappedLabels {
    pub name: Option<String>,
    pub environment: Option<String>,
    pub pod: Option<String>,
    pub container: Option<String>,
    pub owner: Option<String>,
}

static LABEL_TO_COLUMN: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    [
        ("container_label_io_kubernetes_pod_name", "pod"),
        ("pod", "pod"),
        ("container_label_io_kubernetes_container_name", "container"),
        ("container", "container"),
        ("cluster", "environment"),
        ("cumulocity_environment", "environment"),
        ("resource", "resource"),
        ("label_owner", "owner"),
        ("__name__", "dp_name"),
    ]
    .iter()
    .cloned()
    .collect()
});

static NAME_TO_COLUMN: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    [
        ("container_cpu_usage_seconds_total", "cpu_usage_total"),
        ("container_memory_working_set_bytes", "memory_usage"),
        ("kube_pod_labels", "owner"),
    ]
    .iter()
    .cloned()
    .collect()
});

static POD_PREFIX_BLACKLIST: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "daemonset-",
        "deployment-",
        "kube-",
        "node-",
        "ebs-",
        "efs-",
    ]
});

pub fn map(labels: &[Label]) -> Option<MappedLabels> {
    let mut result = MappedLabels::default();

    for label in labels {
        if let Some(&mapped_key) = LABEL_TO_COLUMN.get(label.name.as_str()) {
            match mapped_key {
                "pod" => result.pod = Some(label.value.clone()),
                "container" => result.container = Some(label.value.clone()),
                "environment" => result.environment = Some(label.value.clone()),
                "owner" => result.owner = Some(label.value.clone()),
                "dp_name" => result.name = Some(label.value.clone()),
                _ => {}
            }
        }
    }

    if let Some(dp_name) = &result.name {
        if let Some(&mapped_name) = NAME_TO_COLUMN.get(dp_name.as_str()) {
            result.name = Some(mapped_name.to_string());
        } else if dp_name == "kube_pod_container_resource_limits" {
            if let Some(resource) = labels.iter().find(|l| l.name == "resource") {
                if resource.value == "cpu" {
                    result.name = Some("cpu_limit".to_string());
                } else if resource.value == "memory" {
                    result.name = Some("memory_limit".to_string());
                }
            }
        }
    }

    if result.container.as_deref() == Some("POD")
        || result.pod.is_none()
        || result
            .pod
            .as_ref()
            .map(|pod| {
                POD_PREFIX_BLACKLIST
                    .iter()
                    .any(|prefix| pod.starts_with(prefix))
            })
            .unwrap_or(false)
    {
        return None;
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_straight() {
        let labels = vec![
            Label {
                name: "cluster".to_string(),
                value: "test_prod".to_string(),
            },
            Label {
                name: "__name__".to_string(),
                value: "container_cpu_usage_seconds_total".to_string(),
            },
            Label {
                name: "pod".to_string(),
                value: "test_pod".to_string(),
            },
        ];

        let result = map(&labels);

        assert_eq!(
            result,
            Some(MappedLabels {
                environment: Some("test_prod".to_string()),
                name: Some("cpu_usage_total".to_string()),
                pod: Some("test_pod".to_string()),
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_map_ksm_cpu() {
        let labels = vec![
            Label {
                name: "resource".to_string(),
                value: "cpu".to_string(),
            },
            Label {
                name: "__name__".to_string(),
                value: "kube_pod_container_resource_limits".to_string(),
            },
            Label {
                name: "pod".to_string(),
                value: "test_pod".to_string(),
            },
        ];

        let result = map(&labels);

        assert_eq!(
            result,
            Some(MappedLabels {
                name: Some("cpu_limit".to_string()),
                pod: Some("test_pod".to_string()),
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_map_ksm_memory() {
        let labels = vec![
            Label {
                name: "resource".to_string(),
                value: "memory".to_string(),
            },
            Label {
                name: "__name__".to_string(),
                value: "kube_pod_container_resource_limits".to_string(),
            },
            Label {
                name: "pod".to_string(),
                value: "test_pod".to_string(),
            },
        ];

        let result = map(&labels);

        assert_eq!(
            result,
            Some(MappedLabels {
                name: Some("memory_limit".to_string()),
                pod: Some("test_pod".to_string()),
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_map_skip_pod() {
        let labels = vec![
            Label {
                name: "container".to_string(),
                value: "POD".to_string(),
            },
            Label {
                name: "pod".to_string(),
                value: "test_pod".to_string(),
            },
        ];

        let result = map(&labels);

        assert_eq!(result, None);
    }

    #[test]
    fn test_map_skip_blacklisted_pod() {
        let labels = vec![
            Label {
                name: "container".to_string(),
                value: "test_container".to_string(),
            },
            Label {
                name: "pod".to_string(),
                value: "daemonset-test".to_string(),
            },
        ];

        let result = map(&labels);

        assert_eq!(result, None);
    }

    #[test]
    fn test_map_no_match() {
        let labels = vec![Label {
            name: "unknown_label".to_string(),
            value: "unknown_value".to_string(),
        }];

        let result = map(&labels);

        assert_eq!(result, None);
    }

    #[test]
    fn test_map_owner() {
        let labels = vec![
            Label {
                name: "__name__".to_string(),
                value: "kube_pod_labels".to_string(),
            },
            Label {
                name: "label_owner".to_string(),
                value: "a-team".to_string(),
            },
            Label {
                name: "pod".to_string(),
                value: "test_pod".to_string(),
            },
        ];

        let result = map(&labels);

        assert_eq!(
            result,
            Some(MappedLabels {
                name: Some("owner".to_string()),
                owner: Some("a-team".to_string()),
                pod: Some("test_pod".to_string()),
                ..Default::default()
            })
        );
    }
}
