# Multi-stage non-root Dockerfile for TableTalk Backend
# Stage 1: Build binary
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /usr/src/tabletalk

# Install system build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy cargo manifest and lockfile
COPY Cargo.lock ./Cargo.lock
COPY backend/Cargo.toml ./backend/Cargo.toml

# Copy backend source and migrations
COPY backend/src ./backend/src
COPY backend/migrations ./backend/migrations

# Build production binary in release mode
RUN cargo build --release --manifest-path backend/Cargo.toml --bin mcp_tabletalk_backend

# Stage 2: Minimal non-root runtime image
FROM debian:bookworm-slim AS runtime

# Install minimal runtime dependencies (curl for healthcheck and CA certificates)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root group and user
RUN groupadd -g 10001 tabletalk && \
    useradd -u 10001 -g tabletalk -s /bin/sh -m tabletalk

WORKDIR /app

# Copy binary from builder
COPY --from=builder /usr/src/tabletalk/target/release/mcp_tabletalk_backend /app/mcp_tabletalk_backend

# Prepare database directory with non-root ownership
RUN mkdir -p /app/data && chown -R tabletalk:tabletalk /app

USER tabletalk:tabletalk

ENV PORT=8080 \
    DATABASE_URL="sqlite:///app/data/tabletalk.db" \
    RUST_LOG="info" \
    SEED_ON_START="true"

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["/app/mcp_tabletalk_backend"]
