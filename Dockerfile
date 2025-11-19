FROM rust:1.75 as builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY narayana-core narayana-core
COPY narayana-storage narayana-storage
COPY narayana-query narayana-query
COPY narayana-api narayana-api
COPY narayana-server narayana-server

# Build the server
WORKDIR /app/narayana-server
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/narayana-server/target/release/narayana-server /app/narayana-server

EXPOSE 8080

ENV RUST_LOG=info
ENV NARAYANA_DATA_DIR=/data

VOLUME ["/data"]

CMD ["/app/narayana-server"]

