# ./docker/Dockerfile
# Multi-stage: static Svelte UI + release Rust binary for portable desktop and headless server sweeps.
#
# Build:
#   docker build -f docker/Dockerfile -t genomics-caddy:latest .
# Run worker (80-core server example):
#   docker compose -f docker/docker-compose.yml up genomics-worker

FROM node:22-bookworm-slim AS frontend
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY svelte.config.js vite.config.ts tsconfig.json ./
COPY src ./src
COPY static ./static
RUN npm run build

FROM rust:1.85-bookworm AS rust-builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
    && rm -rf /var/lib/apt/lists/*
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock ./src-tauri/
COPY src-tauri/build.rs src-tauri/tauri.conf.json ./src-tauri/
COPY src-tauri/icons ./src-tauri/icons
COPY src-tauri/src ./src-tauri/src
WORKDIR /app/src-tauri
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /opt/genomics-caddy
COPY --from=rust-builder /app/src-tauri/target/release/DNA-Tools /usr/local/bin/genomics-caddy
COPY --from=frontend /app/build ./ui
ENV GENOMICS_DATA_DIR=/data
VOLUME ["/data"]
ENTRYPOINT ["/usr/local/bin/genomics-caddy"]
CMD ["--headless-sweep", "--sample-id=1"]

FROM nginx:1.27-alpine AS ui
COPY --from=frontend /app/build /usr/share/nginx/html
COPY docker/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
