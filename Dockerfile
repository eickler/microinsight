FROM python:alpine as builder
RUN apk add --no-cache gcc musl-dev python3-dev libffi-dev openssl-dev cargo
COPY requirements.txt .
RUN pip install --prefix="/install" -r requirements.txt

FROM python:alpine
COPY --from=builder /install /usr/local
COPY *.py /
COPY gogoproto/ /gogoproto/
EXPOSE 80
ENTRYPOINT ["python"]
CMD ["-u", "microinsight.py"]
