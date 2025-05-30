FROM rust:slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    protobuf-compiler-grpc \
    libprotobuf-dev \
    zlib1g \
    libssl-dev musl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY . /app
RUN cargo test --release --lib  # Run only unit tests
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
LABEL org.opencontainers.image.title="microinsight" \
  org.opencontainers.image.description="Collecting microservice metrics from Prometheus for easier analysis." \
  org.opencontainers.image.source="https://github.com/eickler/microinsight"
COPY --from=builder /app/target/release/microinsight /
COPY --from=builder /lib/*/libz.so.1 /lib/
EXPOSE 80
CMD ["./microinsight"]
