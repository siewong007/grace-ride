# syntax=docker/dockerfile:1.7
# 恩典同行 Grace Ride — single image: Rust API + built React app.

# ---------- Frontend ----------
FROM node:22-alpine AS web
WORKDIR /web
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY frontend/ ./
RUN npm run build

# ---------- Backend ----------
FROM rust:1-bookworm AS api
WORKDIR /src
# Cache dependencies separately from application code.
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
 && cargo build --release --locked && rm -rf src
COPY backend/ ./
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

# ---------- Runtime ----------
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates libssl3 tzdata \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --home /app --shell /usr/sbin/nologin grace \
 && mkdir -p /data/uploads && chown -R grace /data
WORKDIR /app
COPY --from=api /src/target/release/grace-ride /app/grace-ride
COPY --from=web /web/dist /app/web
ENV TZ=Asia/Kuala_Lumpur \
    BIND=0.0.0.0:8080 \
    DATABASE_URL=sqlite:///data/grace_ride.db \
    UPLOAD_DIR=/data/uploads \
    STATIC_DIR=/app/web \
    DEV_MODE=false \
    TRUST_PROXY=true \
    LOG_FORMAT=json
USER grace
VOLUME ["/data"]
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 CMD ["/app/grace-ride", "healthcheck"]
ENTRYPOINT ["/app/grace-ride"]
