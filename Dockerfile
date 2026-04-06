# ── Stage 1: Frontend build ─────────────────────────────────────────────────
FROM node:20-alpine AS frontend-build
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# ── Stage 2: Rust build ──────────────────────────────────────────────────────
FROM rust:1.85-slim-bookworm AS rust-build
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY migrations/ ./migrations/
COPY --from=frontend-build /app/frontend/dist ./frontend/dist/

# Stub build.rs so cargo doesn't try to run npm again
RUN echo 'fn main() {}' > build.rs

RUN cargo build --release

# ── Stage 3: Runtime ─────────────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=rust-build /app/target/release/discoprowl ./discoprowl

EXPOSE 3079
CMD ["./discoprowl"]
