# ---------- Base build stage ----------
FROM rust:1-trixie AS chef
RUN cargo install cargo-chef
WORKDIR /app

# ---------- Planner stage ----------
FROM chef AS planner

# Copy only dependency manifests
COPY Cargo.toml Cargo.lock ./
COPY encryption/Cargo.toml encryption/Cargo.lock ./encryption/

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

# Copy dependency recipe
COPY --from=planner /app/recipe.json recipe.json

# Build deps only (cached unless Cargo.toml changes)
RUN cargo chef cook --release --recipe-path recipe.json

# ---- Install dx (cached) ----
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
  https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

RUN cargo binstall dioxus-cli --root /.cargo -y --force
ENV PATH="/.cargo/bin:$PATH"

# ---- Copy web assets (only affects dx bundle) ----
COPY assets ./assets
COPY public ./