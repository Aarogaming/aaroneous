# Builder stage
FROM rust:1.70-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/aaroneous

# Copy the source code
COPY . .

# Build the core application (release mode)
# We enable standard features but can be overridden by docker-compose args
RUN cargo build --release --bin aaroneous

# Production stage
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create necessary directories
RUN mkdir -p /app/data/models /app/data/dna_bank /app/data/logs

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/aaroneous/target/release/aaroneous /usr/local/bin/aaroneous

# Set up environment variables
ENV RUST_LOG="info"
ENV AARONEOUS_WORKSPACE="/app"

# Expose standard ports (API, Metrics, Admin)
EXPOSE 8001 8002 8003 8766

# Run the federation by default
CMD ["aaroneous", "start"]
