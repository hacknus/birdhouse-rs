FROM rust:1 AS builder
WORKDIR /app

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli --root /.cargo -y --force
ENV PATH="/.cargo/bin:$PATH"

COPY . .

# Build web bundle WITHOUT server feature, then build server binary separately
RUN dx bundle --release --platform web --no-default-features --features web
RUN cargo build --release --features server

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/app

COPY --from=builder /app/target/release/birdhouse-rs ./server
COPY --from=builder /app/target/dx/birdhouse-rs/release/web/public/ ./public/

ENV PORT=8080
ENV IP=0.0.0.0

EXPOSE 8080

ENTRYPOINT ["./server"]