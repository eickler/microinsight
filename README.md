# microinsight

A small hack to collect Prometheus microservice actuals and limits into MySQL for easier analysis.

## Overview

It might be me, but I did not find a reproducible and reliable way to calculate the ratio of actual usage of a Kubernetes microservice versus the configured limits just with using PromQL. The results never matched a manual calculation and there was nearly no formal documentation on the semantics of PromQL. Since this ratio is commercially relevant to me, I wanted to have the results reproducible and hence wrote a little microservice to dump the relevant data into MySQL. Using MySQL, I can do calculations using the well-known semantics of SQL.

The pipeline is as follows:

* The data is provided by [cAdvisor](https://github.com/google/cadvisor) and [KSM](https://github.com/kubernetes/kube-state-metrics).
* The data is scraped by Prometheus in regular intervals.
* Prometheus pushes the data through remote_write to microinsight.
* microinsight writes this into a MySQL table ``micrometrics`` (creating it if necessary).
* Query as usual through SQL.

| time                | environment | pod            | container | cpu_usage_total | cpu_limit | memory_usage | memory_limit |
| ------------------- | ----------- | -------------- | --------- | --------------: | --------: | -----------: | -----------: |
| 2024-07-08 10:59:15 | demo        | cadvisor-lwf24 | cadvisor  |        53.80411 |       0.8 |    1.47968E8 |   2.097152E9 |
| 2024-07-08 10:59:30 | demo        | cadvisor-lwf24 | cadvisor  |        54.61136 |       0.8 | 1.49573632E8 |   2.097152E9 |
| 2024-07-08 10:59:45 | demo        | cadvisor-lwf24 | cadvisor  |        54.86298 |       0.8 | 1.36855552E8 |   2.097152E9 |


## Prerequisites

* Kubernetes cluster with [cAdvisor](https://github.com/google/cadvisor) and [KSM](https://github.com/kubernetes/kube-state-metrics) installed.
* Prometheus configured to scrape cadvisor and KSM.
* MySQL installed, for example using the [operator](https://dev.mysql.com/doc/mysql-operator/en/mysql-operator-installation.html).
* Helm.

## Installation

* Install microinsight using helm, with the target MySQL server and the scraping interval configured in Prometheus.

```
helm repo add eickler-charts https://eickler.github.io/charts/
helm repo update
helm install \
  --set db.host=mycluster \
  --set db.user=mysql \
  --set db.pass=mysql \
  --set db.name=mydb \
  --set interval=15 \
  microinsight eickler-charts/microinsight
kubectl get service microinsight
```

* Add a remote_write endpoint to Prometheus, changing the URL as required.

```
remote_write:
  - url: http://microinsight/receive
    write_relabel_configs:
      - source_labels: [__name__]
        regex: "kube_pod_labels|kube_pod_container_resource_limits|container_cpu_usage_seconds_total|container_memory_working_set_bytes"
        action: keep
```

## Fine print

Prometheus samples values at more or less arbitrary points in time. This makes it more difficult to correlate actuals and limits. For that reason, microinsight puts the forwarded values into buckets of size ``INTERVAL`` (truncates to ``INTERVAL`` seconds). E.g., if the interval is five seconds, an actual with timestamp ``2024-07-08 10:59:15:123`` and a limit with timestamp  ``2024-07-08 10:59:16:456`` are placed into the same row. Should another actual with timestamp ``2024-07-08 10:59:19:999`` arrives, it will simply overwrite the previous actual in the row.

``INTERVAL`` should be larger than the larger of the ``scrape_interval`` setting for cAdvisor and kube-state-metrics, otherwise you end up with gaps in the reporting.

``cpu_usage_total`` is a cumulative counter of the consumed CPU in seconds. So to find out what was actually consumed since the last measurement, you would need to subtract the last measured value. For example,

```
SELECT
  time, environment, pod,
  100 * (cpu_usage_total - LAG(cpu_usage_total, 1) OVER (PARTITION BY environment, pod ORDER BY time)) / <INTERVAL> / cpu_limit as usage_percent
FROM micrometrics
WHERE container = 'cadvisor' AND cpu_usage_total IS NOT NULL
ORDER BY time
```

Note that I assume in the examples that the usage counter does not wrap or reset meanwhile. Not sure if that happens in practice. Memory usage is always a sample, which makes it easier to add it to the calculation:

```
SELECT
  time, environment, pod,
  (memory_usage / memory_limit) * (cpu_usage_total - LAG(cpu_usage_total, 1) OVER (PARTITION BY environment, pod ORDER BY time)) / 15 / cpu_limit
FROM micrometrics
WHERE
  time >= CURRENT_TIMESTAMP - INTERVAL 30 DAY AND
  container = 'cadvisor' AND cpu_usage_total IS NOT NULL
ORDER BY time
```



## TBDs

* Internal: Check if there is a difference between cost for RAM and CPUs.
* Filtering irrelevant microservices.
* Can there be milicore values transferred by kube-state-metrics or is it always core fractions?

## Copyright notice

This tool contains [protobuf definitions](https://github.com/prometheus/prometheus/tree/release-2.53/prompb) from the Prometheus project, Copyright Prometheus Team, licensed under Apache 2.0 license as included here.
