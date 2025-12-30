FROM rust:1 AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .

# Install dx using cargo-binstall
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli --root /.cargo -y --force
ENV PATH="/.cargo/bin:$PATH"

# Build the web bundle
RUN dx bundle --release

# Debug: check what was actually created
RUN ls -la /app/target/dx/birdhouse-rs/release/web/ || ls -la /app/target/dx/birdhouse-rs/release/ || ls -la /app/dist/

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    python3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the static web files
COPY --from=builder /app/target/dx/birdhouse-rs/release/web/public/ /usr/local/app/
COPY --from=builder /app/static/ /usr/local/app/static/

ENV PORT=8080

EXPOSE 8080

WORKDIR /usr/local/app

# Serve static files with Python's built-in HTTP server
CMD ["python3", "-m", "http.server", "8080"]