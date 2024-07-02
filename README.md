# microinsight

A small hack to collect Prometheus microservice actuals and limits into MySQL for easier analysis.

```
helm repo add eickler-charts https://eickler.github.io/charts/
helm repo update
helm install \
  --set db.host=mqtt://emqx-listeners:1883 \
  --set db.user=mysql \
  --set db.pass=mysql \
  --set db.name=mysql \
  microinsight eickler-charts/microinsight
kubectl get deployment microinsight
```

This tool contains [protobuf definitions](https://github.com/prometheus/prometheus/tree/release-2.53/prompb) from the Prometheus project, Copyright Prometheus Team, licensed under Apache 2.0 license as included here.
