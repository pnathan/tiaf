FROM rust:1.70.0-slim-bullseye as builder
RUN apt-get update && apt-get install -y  pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src/tiaf
copy .cargo/ .cargo/
copy vendor vendor/
COPY Cargo.lock Cargo.lock
COPY Cargo.toml Cargo.toml
copy rust rust/
RUN cargo build --offline
RUN cp ./target/debug/tiaf-server /tiaf-server
FROM debian:bullseye-slim

COPY --from=builder /tiaf-server /tiaf-server
CMD ["/tiaf-server"]