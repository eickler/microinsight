import unittest
from batch_buffer import BatchBuffer

def create_ts(timestamp, value):
    return type('TestSample', (object,), {'samples': [type('Sample', (object,), {'timestamp': timestamp, 'value': value})()]})()

def create_r(metric):
    return {
        'environment': 'env1',
        'pod': 'pod1',
        'container': 'container1',
        'name': metric
    }

def create_key(r):
    return (r['environment'], r['pod'], r['container'])

class TestBatchBuffer(unittest.TestCase):
    def setUp(self):
        self.max_delay = 5
        self.watermark = 0
        self.buffer = BatchBuffer(60, self.max_delay, self.watermark)

    def test_get_slot_index(self):
        timestamp = 125000
        expected_index = 2  # by interval * 1000 ms, truncated
        expected_batches = 3
        self.assertEqual(len(self.buffer.batches), 0)
        self.assertEqual(self.buffer._get_slot_index(timestamp), expected_index)
        self.assertEqual(len(self.buffer.batches), expected_batches)

    def test_flush_candidate(self):
        oldest_batch, oldest_watermark = self.buffer._flush_candidate()
        self.assertIsNone(oldest_batch)
        self.assertEqual(self.buffer.watermark, self.watermark)

        self.buffer._get_slot_index((self.max_delay - 1)* self.buffer.interval)
        oldest_batch, oldest_watermark = self.buffer._flush_candidate()
        self.assertIsNone(oldest_batch)
        self.assertEqual(self.buffer.watermark, self.watermark)

        self.buffer._get_slot_index(self.max_delay * self.buffer.interval)
        oldest_batch, oldest_watermark = self.buffer._flush_candidate()
        self.assertIsNotNone(oldest_batch)
        self.assertEqual(oldest_watermark, self.watermark)
        self.assertEqual(self.buffer.watermark, self.buffer.interval)

    def test_insert_limit(self):
        timestamp = 120000
        value = 100
        r = create_r('cpu_limit')
        ts = create_ts(timestamp, value)
        self.buffer._insert_samples(r, ts)

        slot_index = self.buffer._get_slot_index(timestamp)
        key = create_key(r)
        self.assertIn(key, self.buffer.batches[slot_index])
        self.assertEqual(self.buffer.batches[slot_index][key]['cpu_limit'], value)

    def test_insert_usage(self):
        first_timestamp = 0
        first_value = 100
        r = create_r('cpu_usage')
        key = create_key(r)
        ts = create_ts(first_timestamp, first_value)
        self.buffer._insert_samples(r, ts)

        # Check that value with no predecessor is None.
        slot_index = self.buffer._get_slot_index(first_timestamp)
        self.assertIn(key, self.buffer.batches[slot_index])
        self.assertIsNone(self.buffer.batches[slot_index][key]['cpu_usage'])
        self.assertEqual(self.buffer.batches[slot_index][key]['cpu_usage_total'], first_value)

        # Check if difference is calculated correctly.
        second_timestamp = 60100
        second_value = 110
        ts = create_ts(second_timestamp, second_value)
        self.buffer._insert_samples(r, ts)

        slot_index = self.buffer._get_slot_index(second_timestamp)
        self.assertIn(key, self.buffer.batches[slot_index])
        self.assertEqual(self.buffer.batches[slot_index][key]['cpu_usage'], second_value - first_value)
        self.assertEqual(self.buffer.batches[slot_index][key]['cpu_usage_total'], second_value)

        # Check if wrapping is handled
        third_timestamp = 121100
        third_value = 10
        ts = create_ts(third_timestamp, third_value)
        self.buffer._insert_samples(r, ts)

        slot_index = self.buffer._get_slot_index(third_timestamp)
        self.assertIsNone(self.buffer.batches[slot_index][key]['cpu_usage'])
        self.assertEqual(self.buffer.batches[slot_index][key]['cpu_usage_total'], third_value)


if __name__ == '__main__':
    unittest.main()
