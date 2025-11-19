# syntax=docker/dockerfile:1.5

# Docker CLI

FROM docker:27-cli AS docker_cli

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

COPY --from=docker_cli /usr/local/bin/docker /usr/local/bin/docker
COPY --from=docker_cli /usr/libexec/docker/ /usr/libexec/docker/
COPY --from=docker_cli /usr/local/lib/docker/cli-plugins /usr/local/lib/docker/cli-plugins

WORKDIR /app

COPY --from=rust_builder /app/target/release/main .

CMD ["./main"]
