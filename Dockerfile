FROM rust:slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    protobuf-compiler-grpc \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . /app
RUN cargo test --release
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
LABEL org.opencontainers.image.title="microinsight" \
  org.opencontainers.image.description="Collecting microservice metrics from Prometheus for easier analysis." \
  org.opencontainers.image.source="https://github.com/eickler/microinsight"
COPY --from=builder /app/target/release/microinsight /
EXPOSE 80
CMD ["./microinsight"]
