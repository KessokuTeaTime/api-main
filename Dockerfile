# syntax=docker/dockerfile:1.5

# Rust builder

FROM rust:bookworm AS rust_builder

RUN cargo install cargo-chef sccache --locked
ENV RUSTC_WRAPPER=sccache \
    SCCACHE_DIR=/sccache

WORKDIR /app

COPY rust-toolchain.toml ./
RUN rustup toolchain install --profile minimal $(grep "channel" rust-toolchain.toml | cut -d'"' -f2)

COPY Cargo.toml Cargo.lock ./
RUN cargo chef prepare --recipe-path recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN echo "Listing files:" && ls -R .
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo build --release

# Runtime image

FROM debian:bookworm-slim
RUN curl -fsSL https://get.docker.com | sh

WORKDIR /app

COPY --from=rust_builder /app/target/release/main .

# Commands

CMD ["./main"]
