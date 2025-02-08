import threading

# BatchBuffer keeps data for max_delay intervals to capture late data.
# `insert` buckets samples into the interval batches and returns the oldest batch if it is time to flush it, minimizing locking.
class BatchBuffer:
    def __init__(self, interval, max_delay, watermark):
        self.interval = interval * 1000 # timestamps are in milliseconds
        self.max_delay = max_delay
        self.watermark = self._truncate_timestamp(watermark)
        self.batches = []
        # Lock to synchronize access to the watermark and the batches, since we may get concurrent requests.
        self.lock = threading.Lock()

    def _truncate_timestamp(self, timestamp):
        return int(timestamp / self.interval) * self.interval

    def _get_slot_index(self, timestamp):
        index = (timestamp - self.watermark) // self.interval
        while len(self.batches) < index + 1:
            self.batches.append({})
        return index

    def _insert_samples(self, r, samples):
        for sample in samples:
            if sample.timestamp < self.watermark:
                continue

            timestamp_trunc_secs = self._truncate_timestamp(sample.timestamp)
            slot_index = self._get_slot_index(timestamp_trunc_secs)
            key = (r['environment'], r['pod'], r['container'])
            if key not in self.batches[slot_index]:
                self.batches[slot_index][key] = {
                    'cpu_usage_total': None,
                    'cpu_usage': None,
                    'cpu_limit': None,
                    'memory_usage': None,
                    'memory_limit': None
                }

            if r['name'] == 'cpu_usage':
                self.batches[slot_index][key]['cpu_usage_total'] = sample.value
                # CPU usage of the interval can only be calculated if there is a previous value and that value did not wrap.
                previous_value = self.batches[slot_index-1][key]['cpu_usage_total'] if slot_index > 0 and key in self.batches[slot_index-1] else None
                if previous_value is not None and sample.value >= previous_value:
                  sample.value = sample.value - previous_value
                else:
                  continue

            self.batches[slot_index][key][r['name']] = sample.value

    def _flush_candidate(self):
        if len(self.batches) > self.max_delay:
            oldest_batch = self.batches.pop(0)
            oldest_watermark = self.watermark
            self.watermark += self.interval
            return oldest_batch, oldest_watermark

        return None, None

    def insert(self, r, samples):
        with self.lock:
            self._insert_samples(r, samples)
            return self._flush_candidate()