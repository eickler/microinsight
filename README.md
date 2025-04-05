# microinsight

A small hack to collect Prometheus microservice actuals and limits into MySQL for easier analysis.

## Overview

It might be me, but I did not find a reproducible and reliable way to calculate the ratio of actual usage of a Kubernetes microservice versus the configured limits just with using PromQL. The results never matched a manual calculation and there was nearly no formal documentation on the semantics of PromQL. Since this ratio is commercially relevant to me, I wanted to have the results reproducible and hence wrote a little microservice to dump the relevant data into MySQL. Using MySQL, I can do calculations using the well-known semantics of SQL.

The pipeline is as follows:

* The data is provided by [cAdvisor](https://github.com/google/cadvisor) and [KSM](https://github.com/kubernetes/kube-state-metrics).
* The data is scraped by Prometheus (or Grafana agent) in regular intervals.
* Prometheus pushes the data through the [remote_write protocol](https://docs.google.com/document/d/1LPhVRSFkGNSuU1fBd81ulhsCPR4hkSZyyBj1SZ8fWOM/edit?tab=t.0) to microinsight.
* microinsight postprocesses the data and writes the result in `INTERVAL` seconds into a MySQL table `micrometrics`.
  * The table is created if necessary.
  * System containers and containers without any limits are excluded. (Please crosscheck `POD_PREFIX_BLACKLIST` in `writer.py`.)
  * Pleae see [late data handling](#late-data-handling) below.
* Query as usual through SQL.

This is an example of the output:

| time                | environment | pod            | container | cpu_usage | cpu_limit | memory_usage | memory_limit |
| ------------------- | ----------- | -------------- | --------- | --------: | --------: | -----------: | -----------: |
| 2024-07-08 10:57:00 | demo        | cadvisor-lwf24 | cadvisor  |  23.80411 |        48 |    1.47968E8 |   2.097152E9 |
| 2024-07-08 10:58:00 | demo        | cadvisor-lwf24 | cadvisor  |  24.61136 |        48 | 1.49573632E8 |   2.097152E9 |
| 2024-07-08 10:59:00 | demo        | cadvisor-lwf24 | cadvisor  |  24.86298 |        48 | 1.36855552E8 |   2.097152E9 |

cAdvisor calculates CPU usage in seconds, so `cpu_usage` reflects the CPU seconds consumed in the configured writing interval. `cpu_limit` is the maximum CPU seconds a container can consume in the interval (i.e., the actually configured limit in Kubernetes x the interval). Example: Assume an interval of one minute. In the minute following 10:57:00, the container `cadvisor` used 23.80411 CPU seconds and could have used up to 48 CPU seconds -- per second.  So the CPU utilization was around 23.80411 / 48 * 100 / 60 ~ 0.826%. The memory utilization was 100 * 1.47968E8 bytes / 2.097152E9 bytes, so a mere 7%.

## Prerequisites

What is needed?

* A Kubernetes cluster with [cAdvisor](https://github.com/google/cadvisor) and [KSM](https://github.com/kubernetes/kube-state-metrics) installed.
* Prometheus configured to scrape cadvisor and KSM.
* MySQL installed, for example using the [operator](https://dev.mysql.com/doc/mysql-operator/en/mysql-operator-installation.html).
* Helm.

## Installation

* Install microinsight using helm, with the target MySQL server and the scraping interval configured in Prometheus. The interval is optional and by default 60 seconds. It should be preferably a multiple of both `scrape_interval`s configured in Prometheus for cAdvisor and KSM.

```
helm repo add eickler-charts https://eickler.github.io/charts/
helm repo update
helm install \
  --set db.host=mycluster \
  --set db.user=mysql \
  --set db.pass=mysql \
  --set db.name=mydb
  microinsight eickler-charts/microinsight
kubectl get service microinsight
```

* The chart creates a service under which microinsight is reachable.
* Add a remote_write endpoint to Prometheus, changing the destination URL to wherever microinsight is exposed. (Or equivalently for Grafana.)

```
remote_write:
  - url: http://microinsight/receive
    write_relabel_configs:
      - source_labels: [__name__]
        regex: "kube_pod_labels|kube_pod_container_resource_limits|container_cpu_usage_seconds_total|container_memory_working_set_bytes"
        action: keep
```

## Configuration parameters

| Chart     | Env        | Default | Description                                           |
| --------- | ---------- | ------- | ----------------------------------------------------- |
| db.host   | DB_HOST    |         | MySQL/MariaDB database host                           |
| db.user   | DB_USER    |         | Database username                                     |
| db.pass   | DB_PASS    |         | Database user password                                |
| db.name   | DB_NAME    |         | Database name                                         |
| interval  | INTERVAL   | 60      | Interval in seconds for creating database entries     |
| maxdelay  | MAX_DELAY  | 5       | Number of intervals to keep in memory for late data   |
| loglevel  | LOG_LEVEL  | INFO    | Rust log level (trace, debug, info, warn, error)      |
| threads   | THREADS    | 32      | Number of threads accepting connections               |
| chunksize | CHUNK_SIZE | 5000    | Number of rows to write to the database in one insert |

Note: The latter depends on the `max_allowed_packet` size of the database. If you get an error related to packet size, reduce the chunk size.


## Fine print

### Prometheus sampling

Prometheus samples values at more or less arbitrary points in time during the configured `scrape_interval`. This makes it more difficult to correlate actuals and limits. For that reason, microinsight puts the forwarded values into buckets of size `INTERVAL`, truncating the timestamp to `INTERVAL` seconds. E.g., if the interval is 60 seconds, an actual with timestamp `2024-07-08 10:59:15.123` and a limit with timestamp  `2024-07-08 10:59:16.456` are placed into the same bucket with tiemstamp `2024-07-08 10:59:00.000`. Should another actual with timestamp `2024-07-08 10:59:19.999` arrive, it will simply overwrite the previous actual in the bucket. When the next value after the 60 second interval arrives, a new bucket begins.

### Late data handling

Data can arrive sometimes pretty late and outside of timestamp order. For that reason, microinsight keeps `MAX_DELAY` buckets in memory and only flushes the oldest bucket to the database when the `MAX_DELAY + 1` bucket begins. When data for already flushed buckets still arrives, the data is discarded and a warning is printed. If you regularly see the message, please adjust either `INTERVAL` or `MAX_DELAY`. If microinsight is terminated for some reason, the buckets in memory are lost. (Note that the in-memory state also means that microinsight currently needs to be a singleton and can only be vertically scaled.)

### CPU usage handling

Since `cpu_uages_total` is reported by cAdvisor as a cumulative total, microinsight subtracts the current bucket's total from the last bucket's total. That saves you some handstands in your SQL during reporting.

```
SELECT
  time, environment, pod,
  100 * cpu_usage / cpu_limit / 60 as cpu_utilization_percent,
  100 * memory_usage / memory_limit as memory_utilization_percent
FROM micrometrics
WHERE container = 'cadvisor'
ORDER BY time
```

Please note that if you aggregate the utilization across containers, you need to first add up the values across all containers and only then calculate utilization in a second step.

If you first calculate the utilization per container and then average across all containers, every container will get the same weight, which is what you often do not want. For example, if a container with 1MB limit has 10% utilization and container with 1000MB limit has 90% utilization, the average utilization across containers is 50%. However, the memory usage in the cluster is not (1MB + 1000 MB) * 50%, but 1MB * 10% + 1000MB * 90% = 900.1 MB.

```
SELECT
  owner,
  100 * sum(cpu_usage) / sum(cpu_limit*60) AS avg_cpu_utilization,
  100 * sum(memory_usage) / sum(memory_limit) AS avg_memory_utilization
FROM
  micrometrics mm LEFT JOIN microowner mo ON mm.environment = mo.environment AND mm.pod = mo.pod
WHERE
  time >= NOW() - INTERVAL 30 DAY
GROUP BY
  owner
```

## TBDs

* There's no authentication on the endpoint (currently done before the endpoint).
* The service cannot be horizontally scaled.

## License and copyright notice

This software is made available under Apache License, Version 2.0.

This repository contains [protobuf definitions](https://github.com/prometheus/prometheus/tree/release-2.53/prompb) from the Prometheus project, Copyright Prometheus Team, licensed under Apache License, Version 2.0, as included here.
