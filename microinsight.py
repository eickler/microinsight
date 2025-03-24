import logging
import os
from flask import Flask, request, jsonify
import snappy
from remote_pb2 import WriteRequest
from writer import Writer

app = Flask(__name__)

@app.route('/health')
def health():
    return '{ "status" : "UP" }'

@app.route('/receive', methods=['POST'])
def receive_data():
    compressed_data = request.data
    decompressed_data = snappy.uncompress(compressed_data)

    write_request = WriteRequest()
    write_request.ParseFromString(decompressed_data)
    writer.insert(write_request)
    return jsonify(success=True)

if __name__ == '__main__':
    level = os.getenv('LOG_LEVEL', 'INFO')
    threads = int(os.getenv('THREADS', 32))
    logging.basicConfig(format='%(asctime)s %(levelname)-8s %(message)s', datefmt='%Y-%m-%d %H:%M:%S', level=getattr(logging, level, logging.INFO))
    import dumper
    writer = Writer()
    from waitress import serve
    serve(app, host="0.0.0.0", port=80,threads=threads,connection_limit=threads*5)
