import logging
import os
# Connection pool has by default up to 100 connections in parallel and does retries.
import pymysqlpool

# The interval in seconds to which the timestamps are truncated, so that the measurements can be matched.
INTERVAL = 15

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
            password=get_env_or_throw('DB_PASSWORD'),
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
        for label in labels:
            if label.name == 'container_label_io_kubernetes_pod_name' or label.name == 'cluster':
                pod = label.value
            elif label.name == 'container_label_io_kubernetes_pod_name' or label.name == 'pod':
                container = label.value
            elif label.name == 'cumulocity_environment' or label.name == 'cluster':
                environment = label.value
            elif label.name == '__name__':
                dp_name = label.value
        if dp_name == 'container_cpu_usage_seconds_total':
            col_name = 'cpu_usage_total'
        elif dp_name == 'container_cpu_limit':
            col_name = 'cpu_limit'
        elif dp_name == 'container_memory_usage_bytes':
            col_name = 'memory_usage'
        elif dp_name == 'container_memory_limit_bytes':
            col_name = 'memory_limit'
        return environment, pod, container, col_name

    # If straight single insertion is too slow, we can also batch the insertions from a buffer.
    def insert(self, write_request):
        with self.pool.get_connection() as connection, connection.cursor() as cursor:
            for ts in write_request.timeseries:
                (environment, pod, container, name) = self.map(ts.labels)
                if name is None:
                    continue
                query = f"""
                    INSERT INTO micrometrics (time, environment, pod, container, {name})
                    VALUES (%s, %s, %s)
                    ON DUPLICATE KEY UPDATE
                    {name} = VALUES({name})
                """.format(name=name)
                for sample in ts.samples:
                    # Truncate the timestamp to the nearest lower INTERVAL second interval.
                    sample.timestamp = int(sample.timestamp / INTERVAL) * INTERVAL
                    logging.debug(f'Inserting {sample.timestamp} {environment} {pod} {container} {name} {sample.value}')
                    cursor.execute(query, (environment, pod, container, sample.timestamp, sample.value))
            connection.commit()
