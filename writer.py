import logging
import os
import threading
from datetime import datetime
import pymysql
import pymysqlpool
from batch_buffer import BatchBuffer

# The size of the timestamp buckets
INTERVAL = int(os.getenv('INTERVAL', 60))
# The number of timestamp buckets to keep in memory
MAX_DELAY = int(os.getenv('MAX_DELAY', 5))
# Maximum number of rows to insert in one go
CHUNK_SIZE = int(os.getenv('CHUNK_SIZE', 5000))
# Time when the microservice owners are flushed to the database
OWNER_FLUSH_INTERVAL = int(os.getenv('OWNER_FLUSH_INTERVAL', 300))  # 5 minutes
# Maximum database connection pool size
POOL_SIZE = int(os.getenv('POOL_SIZE', int(os.getenv('THREADS', 32))))
# Too old data is discarded, default is one day
TOO_OLD_DATA = int(os.getenv('TOO_OLD_DATA', 86400))

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

POD_PREFIX_BLACKLIST = ["daemonset-", "deployment-", "kube-", "node-", "ebs-", "efs-"]

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
            database=get_env_or_throw('DB_NAME'),
            maxsize=POOL_SIZE
        )
        self.batch_buffer = None
        self.batch_buffer_lock = threading.Lock()
        self.owner_buffer = []
        self.owner_buffer_lock = threading.Lock()
        self.last_owner_flush = datetime.now()
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

    def _flush_owner_buffer(self, buffer):
        query = """
            INSERT IGNORE INTO microowner (environment, pod, owner)
            VALUES (%s, %s, %s)
        """
        try:
            with self.pool.get_connection() as connection, connection.cursor() as cursor:
                cursor.executemany(query, buffer)
                connection.commit()
            logging.debug(f'Flushed {len(buffer)} owner entries to the database')
            self.last_owner_flush = datetime.now()
        except pymysql.err.OperationalError as e:
            logging.warning(f'Error inserting owners: {e}')
            logging.debug("Exception details", exc_info=True)

    def _insert_metrics(self, r, samples):
        with self.batch_buffer_lock:
            return self.batch_buffer.insert(r, samples)

    def insert_metrics(self, r, samples):
        flush_batch, timestamp = self._insert_metrics(r, samples)
        if flush_batch:
            try:
                self.write_batch_to_db(flush_batch, timestamp)
            except pymysql.err.OperationalError as e:
                logging.warning(f'Error inserting metrics: {e}')
                logging.debug("Exception details", exc_info=True)

    def write_batch_to_db(self, batch, timestamp):
        try:
            with self.pool.get_connection() as connection, connection.cursor() as cursor:
                timestamp_datetime = datetime.fromtimestamp(timestamp / 1000)  # Convert milliseconds to seconds
                insert_values = batch_to_array(timestamp_datetime, batch)
                if len(insert_values) == 0:
                    return
                query = """
                    INSERT INTO micrometrics (time, environment, pod, container, cpu_usage, cpu_limit, memory_usage, memory_limit)
                    VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
                    ON DUPLICATE KEY UPDATE
                    cpu_usage = VALUES(cpu_usage),
                    cpu_limit = VALUES(cpu_limit),
                    memory_usage = VALUES(memory_usage),
                    memory_limit = VALUES(memory_limit)
                """
                logging.debug(f'{len(insert_values)} of {len(batch)} entries at {timestamp_datetime} have limits, inserting.')
                for i in range(0, len(insert_values), CHUNK_SIZE):
                    chunk = insert_values[i:i + CHUNK_SIZE]
                    cursor.executemany(query, chunk)
                connection.commit()
        except pymysql.err.OperationalError as e:
            logging.warning(f'Error inserting batch: {e}')
            logging.debug("Exception details", exc_info=True)

    def _insert_owner(self, r):
        with self.owner_buffer_lock:
            self.owner_buffer.append((r['environment'], r['pod'], r['owner']))

            if (datetime.now() - self.last_owner_flush).total_seconds() > OWNER_FLUSH_INTERVAL:
                buffer = self.owner_buffer
                self.owner_buffer = []
                return buffer
            return None

    def insert_owner(self, r):
        if r['environment'] is None or r['pod'] is None or r['owner'] is None:
            return

        buffer = self._insert_owner(r)
        if buffer:
            self._flush_owner_buffer(buffer)

    def insert(self, write_request):
        with self.batch_buffer_lock:
            if self.batch_buffer is None and len(write_request.timeseries) > 0:
                watermark = min(sample.timestamp for ts in write_request.timeseries for sample in ts.samples)
                too_old = datetime.now().timestamp() - TOO_OLD_DATA
                if watermark < too_old:
                    watermark = too_old
                self.batch_buffer = BatchBuffer(INTERVAL, MAX_DELAY, watermark)

        total = 0
        for ts in write_request.timeseries:
            total += len(ts.samples)
            r = map(ts.labels)
            if skip(r):
                continue

            if r['name'] == "owner":
                self.insert_owner(r)
            elif len(ts.samples) > 0 and r['container'] is not None:
                logging.debug(f'Received {len(write_request.timeseries)} metrics timeseries, buffer has {len(self.batch_buffer.batches)} batches so far')
                self.insert_metrics(r, ts.samples)
        return total
