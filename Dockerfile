# ---------- Base build stage ----------
FROM rust:1-trixie AS chef
RUN cargo install cargo-chef
WORKDIR /app

# ---------- Planner stage ----------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---------- Build stage ----------
FROM chef AS builder

# Install build deps
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy encryption first (if used by build scripts)
COPY ./encryption ./encryption

# Cook deps
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy full source
COPY . .

# Install dx
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
  https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli --root /.cargo -y --force
ENV PATH="/.cargo/bin:$PATH"

# Build web assets
RUN dx bundle --release --platform web

# Build server
RUN cargo build --release --features server

# ---------- Runtime stage ----------
FROM debian:trixie-slim AS runtime
WORKDIR /usr/local/app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy artifacts
COPY --from=builder /app/target/release/birdhouse-rs ./server
COPY --from=builder /app/target/dx/birdhouse-rs/release/web/public ./public

ENV PORT=8080
ENV IP=0.0.0.0
ENV RUST_BACKTRACE=1

EXPOSE 8080

ENTRYPOINT ["./server"]