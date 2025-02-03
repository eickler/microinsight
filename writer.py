import logging
import os
from datetime import datetime
# Connection pool has by default up to 100 connections in parallel and does retries.
import pymysqlpool

# The size of the timestamp buckets
INTERVAL = int(os.getenv('INTERVAL', 15))

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
    'container_cpu_usage_seconds_total': 'cpu_usage_total',
    'container_memory_working_set_bytes': 'memory_usage',
    'kube_pod_labels': 'owner'
}

POD_PREFIX_BLACKLIST = ["daemonset-", "deployment-", "kube-", "node-", "ebs-", "efs-"];

def skip(r):
    return r['container'] == "POD" or any(r['pod'].startswith(prefix) for prefix in POD_PREFIX_BLACKLIST)

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
            # Idiotically, MySQL can have keys only up to 3K in size, so I need to cut the strings.
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

    def insert_metrics(self, cursor, r, ts):
        if r['environment'] is None or r['pod'] is None or r['container'] is None or r['name'] is None:
            return

        query = f"""
            INSERT INTO micrometrics (time, environment, pod, container, {r['name']})
            VALUES (%s, %s, %s, %s, %s)
            ON DUPLICATE KEY UPDATE
            {r['name']} = VALUES({r['name']})
        """
        logging.debug(f'Inserting {len(ts.samples)} samples')
        for sample in ts.samples:
            timestamp_trunc_secs = int(sample.timestamp / 1000.0 / INTERVAL) * INTERVAL
            timestamp_datetime = datetime.fromtimestamp(timestamp_trunc_secs)
            logging.debug(f'Inserting {timestamp_datetime} {r['environment']} {r['pod']} {r['container']} {r['name']} {sample.value}')
            cursor.execute(query, (timestamp_datetime, r['environment'], r['pod'], r['container'], sample.value))

    def insert_owner(self, cursor, r, ts):
        if r['environment'] is None or r['pod'] is None or r['owner'] is None:
            return

        query = f"""
            INSERT IGNORE INTO microowner (environment, pod, owner)
            VALUES (%s, %s, %s)
        """
        logging.debug(f'Inserting owner {r['environment']} {r['pod']} {r['owner']}')
        cursor.execute(query, (r['environment'], r['pod'], r['owner']))

    def insert(self, write_request):
        with self.pool.get_connection() as connection, connection.cursor() as cursor:
            logging.debug(f'Inserting {len(write_request.timeseries)} timeseries')
            for ts in write_request.timeseries:
                r = self.map(ts.labels)
                if skip(r):
                    continue

                if r['name'] == "kube_pod_labels":
                    self.insert_owner(cursor, r, ts)
                else:
                    self.insert_metrics(cursor, r, ts)
            connection.commit()
