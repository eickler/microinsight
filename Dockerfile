FROM python:alpine
EXPOSE 80
COPY *.py gogoproto/gogo_pb2.py /
RUN pip install flask waitress

ENTRYPOINT ["python"]
CMD ["-u", "microinsight.py"]
