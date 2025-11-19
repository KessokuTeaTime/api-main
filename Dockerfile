# syntax=docker/dockerfile:1.5

# Rust builder

FROM rust:bookworm AS rust_builder

WORKDIR /app

COPY rust-toolchain.toml ./
RUN rustup toolchain install --profile minimal $(grep "channel" rust-toolchain.toml | cut -d'"' -f2)

COPY Cargo.toml Cargo.lock build.rs ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    mkdir src && echo "fn main(){}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release

# Runtime image

FROM debian:bookworm-slim
RUN curl -fsSL https://get.docker.com | sh

WORKDIR /app

COPY --from=rust_builder /app/target/release/main .

# Commands

CMD ["./main"]
