# Build stage
FROM rust:1.83-slim as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install Dioxus CLI
RUN cargo install dioxus-cli

WORKDIR /app

# Copy project files
COPY Cargo.toml Cargo.lock ./
COPY Dioxus.toml ./
COPY src ./src
COPY assets ./assets
COPY public ./public
COPY tailwind.css ./

# Build the project
RUN dx build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy built files from builder
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/public ./public
COPY --from=builder /app/assets ./assets

# Expose port
EXPOSE 8080

CMD ["./dist/birdhouse-rs"]
