FROM rust:1 AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --features server --recipe-path recipe.json
COPY . .

# Install dx using cargo-binstall
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli --root /.cargo -y --force
ENV PATH="/.cargo/bin:$PATH"

# Build with server features
RUN dx bundle --release --features server

# Debug output
RUN find /app/target -name "birdhouse-rs" -type f 2>/dev/null || echo "Server binary not found"
RUN ls -la /app/target/dx/birdhouse-rs/release/web/ || echo "Web directory not found"

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the server binary and web assets
COPY --from=builder /app/target/dx/birdhouse-rs/release/web/server /usr/local/app/server
COPY --from=builder /app/target/dx/birdhouse-rs/release/web/public/ /usr/local/app/public/
COPY --from=builder /app/assets/ /usr/local/app/assets/
COPY --from=builder /app/public/ /usr/local/app/public/

ENV PORT=8080
ENV IP=0.0.0.0

EXPOSE 8080

WORKDIR /usr/local/app
ENTRYPOINT [ "/usr/local/app/server" ]