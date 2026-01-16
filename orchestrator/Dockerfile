# CAGE Orchestrator Container Image
# Multi-stage build for minimal production image

FROM rust:1.75-slim-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src ./src

# Build actual binary
RUN cargo build --release --locked

# ============================================
# Runtime Stage
# ============================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    podman \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create cage user
RUN useradd -r -s /bin/false cage && \
    mkdir -p /var/lib/cage /var/log/cage && \
    chown -R cage:cage /var/lib/cage /var/log/cage

# Copy binary from builder
COPY --from=builder /build/target/release/cage-orchestrator /usr/local/bin/

# Copy default config
COPY ../config/cage.yaml /etc/cage/config.yaml

# Set working directory
WORKDIR /opt/cage

# Expose API port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run as root (required for Podman container management)
USER root

# Start orchestrator
CMD ["/usr/local/bin/cage-orchestrator"]
