# ---------- Base build stage ----------
FROM rust:1 AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---------- Build stage ----------
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .

# Install `dx`
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
  https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli --root /.cargo -y --force
ENV PATH="/.cargo/bin:$PATH"

# Bundle the Dioxus fullstack app
RUN dx bundle --web --fullstack --release

# ---------- Runtime stage ----------
FROM debian:bookworm-slim AS runtime
WORKDIR /usr/local/app

FROM chef AS runtime
COPY --from=builder /app/target/dx/birdhouse-rs/release/web/ /usr/local/app

# Set environment
ENV PORT=8080
ENV IP=0.0.0.0

EXPOSE 8080

# Start the server
ENTRYPOINT ["/usr/local/app/server"]