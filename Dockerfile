FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/rust-ftp-server /usr/local/bin/rust-ftp-server
COPY config.json /app/config.json
EXPOSE 2121/tcp
EXPOSE 50000-50100/tcp
ENTRYPOINT ["/usr/local/bin/rust-ftp-server"]
