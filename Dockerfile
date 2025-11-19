# syntax=docker/dockerfile:1.5

# Docker CLI

FROM debian:12 AS docker_cli
RUN apt-get update && \
    apt-get install -y docker.io docker-compose-plugin

# Rust builder

FROM rust:bookworm AS rust_builder

WORKDIR /app

COPY rust-toolchain.toml ./
RUN rustup toolchain install --profile minimal $(grep "channel" rust-toolchain.toml | cut -d'"' -f2)

COPY Cargo.toml Cargo.lock ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    mkdir src && echo "fn main(){}" > src/main.rs && \
    cargo build --release
RUN rm -rf src

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release

# Runtime image

FROM debian:bookworm-slim

COPY --from=docker_cli /usr/bin/docker /usr/bin/docker
COPY --from=docker_cli /usr/libexec/docker/ /usr/libexec/docker/
COPY --from=docker_cli /usr/bin/docker-compose /usr/bin/docker-compose

WORKDIR /app

COPY --from=rust_builder /app/target/release/main .

CMD ["./main"]
