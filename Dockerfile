FROM rust:latest AS builder
WORKDIR /usr/src/stowage
RUN USER=root cargo init --bin .
COPY . . 
RUN rustup target add x86_64-unknown-linux-musl \
    && cargo test --release \
    && cargo build --release --target x86_64-unknown-linux-musl
FROM debian:bullseye
RUN apt-get update && \
    apt-get install -y --no-install-recommends openssl ca-certificates && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /usr/src/stowage/target/x86_64-unknown-linux-musl/release/stowage .
RUN mkdir -p /app/media
ENV RUST_LOG=info
ENV MEDIA_PATH=${MEDIA_PATH:-/app/media}
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080
CMD ["./stowage"]
