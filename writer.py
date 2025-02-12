import logging
import os
from datetime import datetime
# Connection pool has by default up to 100 connections in parallel and does retries.
import pymysqlpool
from batch_buffer import BatchBuffer

# The size of the timestamp buckets
INTERVAL = int(os.getenv('INTERVAL', 60))
# The number of timestamp buckets to keep in memory
MAX_DELAY = int(os.getenv('MAX_DELAY', 5))
# Maximum number of rows to insert in one go
CHUNK_SIZE = int(os.getenv('CHUNK_SIZE', 5000))

LABEL_TO_COLUMN = {
    'container_label_io_kubernetes_pod_name': 'pod',
    'pod': 'pod',
    'container_label_io_kubernetes_container_name': 'container',
    'container': 'container',
    'cluster': 'environment',
    'cumulocity_environment': 'environment',
    'resource': 'resource',
    'label_owner': 'owner',
    '__name__': 'dp_name'
}

NAME_TO_COLUMN = {
    'container_cpu_usage_seconds_total': 'cpu_usage',
    'container_memory_working_set_bytes': 'memory_usage',
    'kube_pod_labels': 'owner'
}

POD_PREFIX_BLACKLIST = ["daemonset-", "deployment-", "kube-", "node-", "ebs-", "efs-"];

def skip(r):
    return r['container'] == "POD" or not r['pod'] or any(r['pod'].startswith(prefix) for prefix in POD_PREFIX_BLACKLIST)

def get_env_or_throw(name):
    value = os.getenv(name)
    if value is None:
        raise ValueError(f'{name} not set')
    return value

def map(labels):
    result = { 'name': None, 'environment': None, 'pod': None, 'container': None, 'owner': None }
    for label in labels:
        if label.name in LABEL_TO_COLUMN:
            result[LABEL_TO_COLUMN[label.name]] = label.value

    if result['dp_name'] in NAME_TO_COLUMN:
        result['name'] = NAME_TO_COLUMN[result['dp_name']]
    elif result['dp_name'] == 'kube_pod_container_resource_limits':
        if result['resource'] == 'cpu':
            result['name']  = 'cpu_limit'
        elif result['resource'] == 'memory':
            result['name'] = 'memory_limit'

    return result

def batch_to_array(timestamp, batch):
    batch_values = []
    for key, metrics in batch.items():
        environment, pod, container = key
        # Skip pod-level metrics, or containers without any limit set (because it doesn't make sense to calculate utilization without a limit).
        if container is None or (metrics['cpu_limit'] is None and metrics['memory_limit'] is None):
            continue
        batch_values.append((
            timestamp, environment, pod, container,
            metrics['cpu_usage'], metrics['cpu_limit'],
            metrics['memory_usage'], metrics['memory_limit']
        ))
    return batch_values


# Digest the Prometheus write requests, post process them and write them to the database in batches.
# This takes late data into account using BatchBuffer.
# It also writes batches to the database in one go as a batch write.
class Writer:
    def __init__(self):
        self.pool = pymysqlpool.ConnectionPool(
            host=get_env_or_throw('DB_HOST'),
            user=get_env_or_throw('DB_USER'),
            password=get_env_or_throw('DB_PASS'),
            database=get_env_or_throw('DB_NAME')
        )
        self.batch_buffer = None
        self.create_table_if_needed()

    def create_table_if_needed(self):
        with self.pool.get_connection() as connection, connection.cursor() as cursor:
            # Idiotically, MySQL can have keys only up to 3K in size, so I need to cut the strings.
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS micrometrics (
                    time TIMESTAMP,
                    environment VARCHAR(255),
                    pod VARCHAR(255),
                    container VARCHAR(255),
                    cpu_usage FLOAT,
                    cpu_limit FLOAT,
                    memory_usage FLOAT,
                    memory_limit FLOAT,
                    PRIMARY KEY (time, environment, pod, container)
                )
            """)
            cursor.execute("""
                CREATE TABLE IF NOT EXISTS microowner (
                    environment VARCHAR(255),
                    pod VARCHAR(255),
                    owner VARCHAR(255),
                    PRIMARY KEY (environment, pod)
                )
            """)
            connection.commit()

    def insert_metrics(self, r, samples):
        flush_batch, timestamp = self.batch_buffer.insert(r, samples)
        if flush_batch:
            self.write_batch_to_db(flush_batch, timestamp)

    def write_batch_to_db(self, batch, timestamp):
        with self.pool.get_connection() as connection, connection.cursor() as cursor:
            timestamp_datetime = datetime.fromtimestamp(timestamp / 1000)  # Convert milliseconds to seconds
            insert_values = batch_to_array(timestamp_datetime, batch)
            query = """
                INSERT INTO micrometrics (time, environment, pod, container, cpu_usage, cpu_limit, memory_usage, memory_limit)
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
                ON DUPLICATE KEY UPDATE
                cpu_usage = VALUES(cpu_usage),
                cpu_limit = VALUES(cpu_limit),
                memory_usage = VALUES(memory_usage),
                memory_limit = VALUES(memory_limit)
            """
            for i in range(0, len(insert_values), CHUNK_SIZE):
                chunk = insert_values[i:i + CHUNK_SIZE]
                logging.debug(f'Inserting batch at {timestamp_datetime} with {len(chunk)} entries')
                cursor.executemany(query, chunk)
            connection.commit()

    def insert_owner(self, r):
        if r['environment'] is None or r['pod'] is None or r['owner'] is None:
            return

        query = """
            INSERT IGNORE INTO microowner (environment, pod, owner)
            VALUES (%s, %s, %s)
        """
        logging.debug(f'Inserting owner {r["environment"]} {r["pod"]} {r["owner"]}')
        with self.pool.get_connection() as connection, connection.cursor() as cursor:
            cursor.execute(query, (r['environment'], r['pod'], r['owner']))
            connection.commit()

    def insert(self, write_request):
        if self.batch_buffer is None:
            watermark = min(sample.timestamp for ts in write_request.timeseries for sample in ts.samples)
            self.batch_buffer = BatchBuffer(INTERVAL, MAX_DELAY, watermark)

        logging.debug(f'Received {len(write_request.timeseries)} timeseries, buffer has {len(self.batch_buffer.batches)} batches so far')

        for ts in write_request.timeseries:
            r = map(ts.labels)
            if skip(r):
                continue

            if r['name'] == "owner":
                self.insert_owner(r)
            else:
                self.insert_metrics(r, ts.samples)
