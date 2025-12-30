FROM rust:1 AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Only cook dependencies without features to avoid WASM/server conflicts
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

# Install dx using cargo-binstall
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli --root /.cargo -y --force
ENV PATH="/.cargo/bin:$PATH"

# dx bundle handles building both WASM (client) and server binary separately
RUN dx bundle --release --features server

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/app

# Copy the server binary and web assets
COPY --from=builder /app/target/dx/birdhouse-rs/release/web/server ./server
COPY --from=builder /app/target/dx/birdhouse-rs/release/web/public/ ./public/
COPY --from=builder /app/assets/ ./assets/
COPY --from=builder /app/public/ ./public/

ENV PORT=8080
ENV IP=0.0.0.0

EXPOSE 8080

ENTRYPOINT ["./server"]