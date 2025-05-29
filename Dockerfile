
FROM rust:1.77 as builder
RUN rustup update stable && rustup self update

WORKDIR /app
COPY . .

# Install build dependencies for musl and OpenSSL
RUN apt-get update && \
    apt-get install -y musl-tools musl-dev pkg-config libssl-dev

# Set env for static OpenSSL
ENV OPENSSL_STATIC=1
ENV OPENSSL_NO_VENDOR=0

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo test --release
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM debian:bullseye
RUN apt-get update && \
    apt-get install -y --no-install-recommends openssl ca-certificates && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/stowage .
RUN mkdir -p /app/media
ENV RUST_LOG=info
ENV MEDIA_PATH=${MEDIA_PATH:-/app/media}
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080
CMD ["./stowage"]
