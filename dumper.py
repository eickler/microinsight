import threading
import traceback
import sys
import time
import logging

def get_thread_dump():
    """Gets a thread dump."""
    thread_dumps = {}
    current_threads = {thread.ident: thread for thread in threading.enumerate()}
    for thread_id, stack_frame in sys._current_frames().items():
        thread_name = current_threads.get(thread_id, threading.Thread()).name
        thread_dumps[thread_name] = "".join(traceback.format_stack(stack_frame))
    return thread_dumps

def log_thread_dumps_periodically(interval=60):
    """Logs thread dumps periodically."""
    logging.debug(f"=== Dumping threads ===")
    while True:
        thread_dumps = get_thread_dump()
        for thread_name, stack_trace in thread_dumps.items():
            logging.debug(f"{thread_name}\n{stack_trace}")
        time.sleep(interval)

# Example usage (start in a separate thread)
thread_dump_thread = threading.Thread(target=log_thread_dumps_periodically)
thread_dump_thread.daemon = True  # allow program to exit even if this thread is running
thread_dump_thread.start()
