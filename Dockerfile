

FROM rust:slim as builder
RUN rustup update stable && rustup default stable && rustup self update && rustup component add cargo
RUN cargo --version

WORKDIR /app
COPY . .



# Install build dependencies for musl and OpenSSL
RUN apt-get update && \
    apt-get install -y musl-tools musl-dev pkg-config libssl-dev perl make

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
ENV DB_PATH=${DB_PATH:-/db/stowage.db}
ENV HOST=0.0.0.0
ENV PORT=8080

EXPOSE 8080
CMD ["./stowage"]
