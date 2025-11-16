# Build stage
FROM rust:1.81-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the application
RUN cargo build --release --bin pmp-gateway-api

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/pmp-gateway-api /usr/local/bin/pmp-gateway-api

# Copy configuration files
COPY config.yaml ./config.yaml

# Expose port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV CONFIG_PATH=/app/config.yaml

# Run the binary
CMD ["pmp-gateway-api"]
