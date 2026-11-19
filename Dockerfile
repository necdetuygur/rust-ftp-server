FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 1000 ftpuser \
    && useradd --uid 1000 --gid ftpuser --create-home --home-dir /app --shell /usr/sbin/nologin ftpuser

WORKDIR /app
COPY --from=builder /app/target/release/rust-ftp-server /usr/local/bin/rust-ftp-server
COPY config.json /app/config.json
RUN chmod 0644 /app/config.json \
    && mkdir -p /app/data/media /app/data/public \
    && chown -R 1000:1000 /app/data

USER 1000:1000
EXPOSE 2121/tcp
EXPOSE 50000-50100/tcp
ENTRYPOINT ["/usr/local/bin/rust-ftp-server"]
