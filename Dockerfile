# Stage 1: Build the TypeScript SDK (produces pulse.min.js)
FROM node:24.15.0-slim AS sdk-builder
WORKDIR /sdk
COPY sdk/package.json sdk/package-lock.json sdk/tsup.config.ts sdk/tsconfig.json ./
RUN npm ci
COPY sdk/src/ src/
RUN npm run build

# Stage 2: Build the Rust server
FROM rust:1.95.0-slim-trixie AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libpq-dev build-essential cmake && \
    rm -rf /var/lib/apt/lists/*

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY crates/pulse-server/Cargo.toml crates/pulse-server/
COPY crates/pulse-common/Cargo.toml crates/pulse-common/
RUN mkdir -p crates/pulse-server/src crates/pulse-common/src crates/pulse-server/static && \
    echo "fn main() {}" > crates/pulse-server/src/main.rs && \
    echo "" > crates/pulse-common/src/lib.rs && \
    echo "" > crates/pulse-server/static/pulse.min.js && \
    cargo build --release -p pulse-server && \
    rm -rf crates/*/src

# Copy SDK build output into Rust static dir
COPY --from=sdk-builder /sdk/dist/pulse.min.global.js crates/pulse-server/static/pulse.min.js

# Build actual source
COPY crates/ crates/
COPY migrations/ migrations/
# Overwrite the static script with the SDK-built version
COPY --from=sdk-builder /sdk/dist/pulse.min.global.js crates/pulse-server/static/pulse.min.js
RUN touch crates/pulse-common/src/lib.rs crates/pulse-server/src/main.rs && cargo build --release -p pulse-server

# Stage 3: Runtime
FROM debian:trixie-slim
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libpq5 wget && \
    rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 1001 appgroup && \
    useradd --uid 1001 --gid appgroup --shell /bin/bash appuser

COPY --from=builder /app/target/release/pulse-server .
COPY --from=builder /app/migrations/ ./migrations/

RUN mkdir -p /app/data && chown -R appuser:appgroup /app

USER appuser

EXPOSE 8090

HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8090/health || exit 1

CMD ["./pulse-server"]
