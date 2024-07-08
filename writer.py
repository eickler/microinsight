import logging
import os
from datetime import datetime
# Connection pool has by default up to 100 connections in parallel and does retries.
import pymysqlpool

# The size of the timestamp buckets
INTERVAL = os.getenv('INTERVAL', 5)

def get_env_or_throw(name):
    value = os.getenv(name)
    if value is None:
        raise ValueError(f'{name} not set')
    return value

class Writer:
    def __init__(self):
        self.pool = pymysqlpool.ConnectionPool(
            host=get_env_or_throw('DB_HOST'),
            user=get_env_or_throw('DB_USER'),
            password=get_env_or_throw('DB_PASS'),
            database=get_env_or_throw('DB_NAME')
        )
        self.create_table_if_needed()

    def create_table_if_needed(self):
        with self.pool.get_connection() as connection, connection.cursor() as cursor:
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS micrometrics (
                    time TIMESTAMP,
                    environment VARCHAR(255),
                    pod VARCHAR(255),
                    container VARCHAR(255),
                    cpu_usage_total FLOAT,
                    cpu_limit FLOAT,
                    memory_usage FLOAT,
                    memory_limit FLOAT,
                    PRIMARY KEY (time, environment, pod, container)
                )
            """)
            connection.commit()

    def map(self, labels):
        environment = pod = container = col_name = None
        for label in labels:
            if label.name == 'container_label_io_kubernetes_pod_name' or label.name == 'pod':
                pod = label.value
            elif label.name == 'container_label_io_kubernetes_container_name' or label.name == 'container':
                container = label.value
            elif label.name == 'cluster' or label.name == 'cumulocity_environment':
                environment = label.value
            elif label.name == 'resource':
                resource = label.value
            elif label.name == '__name__':
                dp_name = label.value

        if dp_name == 'container_cpu_usage_seconds_total':
            col_name = 'cpu_usage_total'
        elif dp_name == 'container_memory_working_set_bytes':
            col_name = 'memory_usage'
        elif dp_name == 'kube_pod_container_resource_limits':
            if resource == 'cpu':
                col_name = 'cpu_limit'
            elif resource == 'memory':
                col_name = 'memory_limit'
        return environment, pod, container, col_name

    # If straight single insertion is too slow, we can also batch the insertions from a buffer.
    def insert(self, write_request):
        with self.pool.get_connection() as connection, connection.cursor() as cursor:
            for ts in write_request.timeseries:
                (environment, pod, container, name) = self.map(ts.labels)
                if environment is None or pod is None or container is None or name is None:
                    continue
                query = f"""
                    INSERT INTO micrometrics (time, environment, pod, container, {name})
                    VALUES (%s, %s, %s, %s, %s)
                    ON DUPLICATE KEY UPDATE
                    {name} = VALUES({name})
                """.format(name=name)
                for sample in ts.samples:
                    timestamp_trunc_secs = int(sample.timestamp / 1000.0 / INTERVAL) * INTERVAL
                    timestamp_datetime = datetime.fromtimestamp(timestamp_trunc_secs)
                    logging.debug(f'Inserting {timestamp_datetime} {environment} {pod} {container} {name} {sample.value}')
                    cursor.execute(query, (timestamp_datetime, environment, pod, container, sample.value))
            connection.commit()
