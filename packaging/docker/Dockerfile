# syntax=docker/dockerfile:1.7

# Build the SPA first: rust-embed bakes web/dist into the server binary.
FROM node:24-bookworm-slim AS web
WORKDIR /src
RUN npm install -g corepack@0.36.0 && corepack enable
ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY web/package.json web/
COPY apps/desktop/package.json apps/desktop/
RUN pnpm install --frozen-lockfile
COPY web ./web
RUN pnpm -C web build

FROM rust:1.99.1-bookworm AS server
WORKDIR /src
RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake perl pkg-config \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml clippy.toml ./
COPY crates ./crates
COPY apps ./apps
COPY data ./data
COPY --from=web /src/web/dist ./web/dist
RUN cargo build --release --locked -p yaoplm-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 1000 oplmdash \
    && useradd --uid 1000 --gid 1000 --home-dir /data --shell /usr/sbin/nologin oplmdash \
    && mkdir -p /data \
    && chown oplmdash:oplmdash /data

COPY --from=server /src/target/release/yaoplm-server /usr/local/bin/yaoplm-server

LABEL org.opencontainers.image.title="oplmdash" \
      org.opencontainers.image.description="Web-first manager for Open PS2 Loader libraries" \
      org.opencontainers.image.source="https://github.com/edcm95/yayaoplm" \
      org.opencontainers.image.licenses="GPL-3.0-or-later"

ENV YAOPLM_BIND=0.0.0.0:7878 \
    YAOPLM_DATA_DIR=/data \
    RUST_LOG=info

USER oplmdash
WORKDIR /data
VOLUME ["/data"]
EXPOSE 7878

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -fsS http://127.0.0.1:7878/health || exit 1

ENTRYPOINT ["/usr/local/bin/yaoplm-server"]
CMD ["--root", "/opl"]
