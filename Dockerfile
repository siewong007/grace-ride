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
# rustls' crypto backend (aws-lc-sys, pulled in by reqwest and sentry) compiles AWS-LC
# with CMake. The rust image ships a C toolchain but not cmake.
RUN apt-get update \
  && apt-get install -y --no-install-recommends cmake \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /src
# backend/Cargo.toml sets `license-file = "../LICENSE"`; place LICENSE at /LICENSE.
COPY LICENSE /LICENSE
# Cache dependencies separately from application code. The toolchain pin comes first
# so both builds below use the same rustc.
COPY backend/Cargo.toml backend/Cargo.lock backend/rust-toolchain.toml ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
  && cargo build --release --locked \
  && rm -rf src target/release/.fingerprint/grace-ride* target/release/deps/*grace_ride* target/release/grace-ride*
COPY backend/ ./
# COPY keeps the checkout's mtimes, which are older than the stub build above, so
# cargo can treat the stub as fresh and ship `fn main() {}` (it did: the container
# exited silently and every deploy rolled back). Touch forces a real rebuild, and
# the size check fails the build if the stub binary ever slips through again.
RUN touch src/main.rs src/lib.rs \
  && cargo build --release --locked \
  && size=$(stat -c%s target/release/grace-ride) \
  && if [ "$size" -lt 5000000 ]; then echo "grace-ride binary is only $size bytes: stub build shipped" >&2; exit 1; fi

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
