import logging
import os
from datetime import datetime
import threading
# Connection pool has by default up to 100 connections in parallel and does retries.
import pymysqlpool

# The size of the timestamp buckets
INTERVAL = int(os.getenv('INTERVAL', 60))
MAX_DELAY = int(os.getenv('MAX_DELAY', 5))

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

# BatchBuffer keeps data for max_delay intervals to capture late data.
# `insert` buckets samples into the interval batches and returns the oldest batch if it is time to flush it, minimizing locking.
class BatchBuffer:
    def __init__(self, interval, max_delay, watermark):
        self.interval = interval
        self.max_delay = max_delay
        self.watermark = watermark
        self.batches = []
        # Lock to synchronize access to the watermark and the batches, since we may get concurrent requests.
        self.lock = threading.Lock()

    def _truncate_timestamp(self, timestamp):
        return int(timestamp / 1000.0 / self.interval) * self.interval

    def _get_slot_index(self, timestamp):
        index = (timestamp - self.watermark) // self.interval
        while len(self.batches) <= index:
            self.batches.append({})
        return index

    def _insert_samples(self, r, ts):
        for sample in ts.samples:
            timestamp_trunc_secs = self._truncate_timestamp(sample.timestamp)
            slot_index = self._get_slot_index(timestamp_trunc_secs)
            key = (r['environment'], r['pod'], r['container'])
            if key not in self.batches[slot_index]:
                self.batches[slot_index][key] = {
                    'cpu_usage': None,
                    'cpu_limit': None,
                    'memory_usage': None,
                    'memory_limit': None
                }

            # CPU usage of the interval can only be calculated if there is a previous value and that value did not wrap.
            if r['name'] == 'cpu_usage' and slot_index > 0 and self.batches[slot_index-1][key]['cpu_usage'] is not None and r['value'] >= self.batches[slot_index-1][key]['cpu_usage'] is None:
                r['value'] -= self.batches[slot_index-1][key]['cpu_usage']

            self.batches[slot_index][key][r['name']] = sample.value

    def _flush_candidate(self):
        if len(self.batches) >= self.max_delay:
            oldest_batch = self.batches.pop(0)
            oldest_watermark = self.watermark
            self.watermark += self.interval
            return oldest_batch, oldest_watermark

        return None, None

    def insert(self, r, ts):
        with self.lock:
            self._insert_samples(r, ts)
            return self._flush_candidate()

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
        self.create_table_if_needed()
        self.batch_buffer = BatchBuffer(INTERVAL, MAX_DELAY)

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

    def map(self, labels):
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

    def insert_metrics(self, r, ts):
        flush_batch, timestamp = self.batch_buffer.insert(r, ts)
        if flush_batch:
            self.write_batch_to_db(flush_batch, timestamp)

    def write_batch_to_db(self, batch, timestamp):
        with self.pool.get_connection() as connection, connection.cursor() as cursor:
            timestamp_datetime = datetime.fromtimestamp(timestamp)
            insert_values = []
            for key, metrics in batch.items():
                environment, pod, container = key
                insert_values.append((
                    timestamp_datetime, environment, pod, container,
                    metrics['cpu_usage'], metrics['cpu_limit'],
                    metrics['memory_usage'], metrics['memory_limit']
                ))

            query = """
                INSERT INTO micrometrics (time, environment, pod, container, cpu_usage, cpu_limit, memory_usage, memory_limit)
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
                ON DUPLICATE KEY UPDATE
                cpu_usage = VALUES(cpu_usage),
                cpu_limit = VALUES(cpu_limit),
                memory_usage = VALUES(memory_usage),
                memory_limit = VALUES(memory_limit)
            """
            logging.debug(f'Inserting batch at {timestamp_datetime} with {len(insert_values)} entries')
            cursor.executemany(query, insert_values)
            connection.commit()

    def insert_owner(self, r, ts):
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
        logging.debug(f'Received {len(write_request.timeseries)} timeseries')

        if self.batch_buffer is None:
            watermark = min(sample.timestamp for ts in write_request.timeseries for sample in ts.samples)
            self.batch_buffer = BatchBuffer(INTERVAL, MAX_DELAY, watermark)

        for ts in write_request.timeseries:
            r = self.map(ts.labels)
            if skip(r):
                continue

            if r['name'] == "kube_pod_labels":
                self.insert_owner(r, ts)
            else:
                self.insert_metrics(r, ts)
