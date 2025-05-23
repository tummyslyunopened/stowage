FROM rust:latest AS builder
WORKDIR /usr/src/stowage
RUN USER=root cargo init --bin .
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
COPY src/ ./src/
RUN touch src/main.rs && \
    cargo build --release && \
    cp target/release/stowage .
FROM debian:bullseye
RUN apt-get update && \
    apt-get install -y --no-install-recommends openssl ca-certificates && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /usr/src/stowage/stowage .
RUN mkdir -p /app/media
ENV RUST_LOG=info
ENV MEDIA_PATH=/app/media
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080
CMD ["./stowage"]
