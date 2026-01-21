# ---------- Base build stage ----------
FROM rust:1-trixie AS chef
RUN cargo install cargo-chef
WORKDIR /app

# ---------- Planner stage ----------
FROM chef AS planner

COPY Cargo.toml Cargo.lock ./
COPY Dioxus.toml ./
COPY src ./src
COPY encryption ./encryption

RUN cargo chef prepare --recipe-path recipe.json

# ---------- Build stage ----------
FROM chef AS builder

# Install build deps
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy path dependencies (REQUIRED for cargo-chef)
COPY encryption ./encryption

# Copy dependency recipe
COPY --from=planner /app/recipe.json recipe.json

# Build deps only (cached unless Cargo.toml changes)
RUN cargo chef cook --release --recipe-path recipe.json

# ---- Install dx (cached) ----
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
  https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

RUN cargo binstall dioxus-cli --root /.cargo -y --force
ENV PATH="/.cargo/bin:$PATH"

# ---- Copy app source (invalidates only when code changes) ----
COPY Cargo.toml Cargo.lock ./
COPY Dioxus.toml ./
COPY src ./src
COPY encryption ./encryption

# ---- Copy assets (only affects dx bundle layer) ----
COPY assets ./assets
COPY public ./public

# ---- Build web bundle ----
RUN dx bundle --release --platform web

# ---- Build server ----
RUN cargo build --release --features server

# ---------- Runtime stage ----------
FROM debian:trixie-slim AS runtime
WORKDIR /usr/local/app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy server binary
COPY --from=builder /app/target/release/birdhouse-rs ./server

# Copy web output
COPY --from=builder /app/target/dx/birdhouse-rs/release/web/public ./public

# Copy runtime data
COPY --from=builder /app/data ./data

ENV PORT=8080
ENV IP=0.0.0.0
ENV RUST_BACKTRACE=1

EXPOSE 8080

ENTRYPOINT ["./server"]