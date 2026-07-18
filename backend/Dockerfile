# Build stage
FROM rust:1.80-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Run stage
FROM debian:bookworm-slim
WORKDIR /app

# Install SSL certificates and openssl libraries (needed for external database and API requests)
RUN apt-get update && apt-get install -y ca-certificates openssl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/backend /usr/local/bin/backend

EXPOSE 8081
CMD ["/usr/local/bin/backend"]
