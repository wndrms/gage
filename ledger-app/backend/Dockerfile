# ── Stage 1: chef base (cargo-chef 설치 + 네이티브 빌드 도구) ────────────────
FROM rust:1.82-bookworm AS chef
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
WORKDIR /app

# ── Stage 2: recipe 생성 ───────────────────────────────────────────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 3: 의존성 빌드 캐시 ────────────────────────────────────────────────
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release

# ── Stage 4: 최소 런타임 이미지 ──────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl && \
    update-ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN useradd --uid 10001 --no-create-home --shell /usr/sbin/nologin ledger

WORKDIR /app
COPY --from=builder /app/target/release/ledger-backend /app/ledger-backend
COPY --from=builder /app/migrations /app/migrations

RUN mkdir -p /app/data/imports && chown -R ledger:ledger /app/data

USER ledger

EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=5s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["/app/ledger-backend"]
