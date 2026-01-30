# Sassy Browser - Docker Container
# Dockerfile demo

FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/sassy-browser /usr/local/bin/
COPY --from=builder /app/assets /usr/local/share/sassy-browser/assets

ENV SASSY_ASSETS_DIR=/usr/local/share/sassy-browser/assets

ENTRYPOINT ["sassy-browser"]
