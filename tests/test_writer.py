import os
import unittest
from unittest.mock import patch, MagicMock
from batch_buffer import BatchBuffer
from writer import INTERVAL, MAX_DELAY, Writer, map, skip
from writer import batch_to_array
from datetime import datetime

class TestWriter(unittest.TestCase):
    @patch('writer.pymysqlpool.ConnectionPool')
    @patch('writer.BatchBuffer')
    def setUp(self, MockBatchBuffer, MockConnectionPool):
        os.environ['DB_HOST'] = 'localhost'
        os.environ['DB_USER'] = 'user'
        os.environ['DB_PASS'] = 'pass'
        os.environ['DB_NAME'] = 'db'
        self.writer = Writer()
        self.writer.pool = MockConnectionPool()
        self.writer.batch_buffer = BatchBuffer(INTERVAL, MAX_DELAY, 0)

    def test_skip(self):
        r = {'container': 'POD', 'pod': 'test_pod'}
        self.assertTrue(skip(r))

        r = {'container': 'test_container', 'pod': 'daemonset-test'}
        self.assertTrue(skip(r))

        r = {'container': 'test_container', 'pod': 'test_pod'}
        self.assertFalse(skip(r))

    def test_map_straight(self):
        labels = [
            type('Label', (object,), {'name': 'cluster', 'value': 'test_prod'}),
            type('Label', (object,), {'name': '__name__', 'value': 'container_cpu_usage_seconds_total'})
        ]
        result = map(labels)
        self.assertIsNone(result['pod'])
        self.assertEqual(result['environment'], 'test_prod')
        self.assertEqual(result['name'], 'cpu_usage')

    def test_map_ksm(self):
        labels = [
            type('Label', (object,), {'name': 'resource', 'value': 'cpu'}),
            type('Label', (object,), {'name': '__name__', 'value': 'kube_pod_container_resource_limits'})
        ]
        result = map(labels)
        self.assertEqual(result['name'], 'cpu_limit')

        labels = [
            type('Label', (object,), {'name': 'resource', 'value': 'memory'}),
            type('Label', (object,), {'name': '__name__', 'value': 'kube_pod_container_resource_limits'})
        ]
        result = map(labels)
        self.assertEqual(result['name'], 'memory_limit')

    def test_batch_to_array(self):
        timestamp = datetime(2023, 10, 1, 12, 0, 0)
        batch = {
            ('env1', 'pod1', 'container1'): {
                'cpu_usage': 1.0,
                'cpu_limit': 2.0,
                'memory_usage': 3.0,
                'memory_limit': 4.0
            },
            ('env2', 'pod2', 'container2'): {
                'cpu_usage': 5.0,
                'cpu_limit': 6.0,
                'memory_usage': 7.0,
                'memory_limit': 8.0
            }
        }
        result = batch_to_array(timestamp, batch)
        expected = [
            (timestamp, 'env1', 'pod1', 'container1', 1.0, 2.0, 3.0, 4.0),
            (timestamp, 'env2', 'pod2', 'container2', 5.0, 6.0, 7.0, 8.0)
        ]
        self.assertEqual(result, expected)

    @patch('writer.Writer.write_batch_to_db')
    def test_insert_metrics(self, mock_write_batch_to_db):
        # Write into the first batch does not trigger a write to the database.
        r = {'environment': 'env', 'pod': 'pod', 'container': 'container', 'name': 'cpu_limit'}
        samples = [type('Sample', (object,), {'timestamp': 1000, 'value': 1.0})()]
        self.writer.insert_metrics(r, samples)
        mock_write_batch_to_db.assert_not_called()

        # Write into a batch larger than MAX_DELAY triggers a write to the database.
        samples = [type('Sample', (object,), {'timestamp': INTERVAL * (MAX_DELAY + 1) * 1000, 'value': 2.0})()]
        self.writer.insert_metrics(r, samples)
        mock_write_batch_to_db.assert_called_once()

        # Check that actually the first batch was written.
        (batch, timestamp), _ = mock_write_batch_to_db.call_args
        self.assertEqual(timestamp, 0)
        self.assertEqual(batch[('env', 'pod', 'container')]['cpu_limit'], 1.0)

    @patch('writer.Writer.insert_owner')
    @patch('writer.Writer.insert_metrics')
    def test_insert(self, mock_insert_metrics, mock_insert_owner):
        labels = [
            type('Label', (object,), {'name': 'pod', 'value': 'heinz'}),
            type('Label', (object,), {'name': '__name__', 'value': 'kube_pod_labels'})
        ]
        write_request = MagicMock()
        write_request.timeseries = [type('WriteRequest', (object,), {'labels': labels})]
        self.writer.insert(write_request)
        mock_insert_owner.assert_called_once()
        mock_insert_metrics.assert_not_called()

if __name__ == '__main__':
    unittest.main()
