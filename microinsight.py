import logging
import os
from flask import Flask, request, jsonify
import snappy
from remote_pb2 import WriteRequest
from writer import Writer

app = Flask(__name__)
writer = Writer()

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
    logging.basicConfig(level=level)
    from waitress import serve
    serve(app, host="0.0.0.0", port=80)
