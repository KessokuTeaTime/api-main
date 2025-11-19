FROM rust:latest AS rust_builder
WORKDIR /app

COPY rust-toolchain.toml ./
RUN rustup show

COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

COPY . .
RUN cargo build --release

# -----------------------
FROM debian:bookworm-slim

WORKDIR /app
COPY --from=rust_builder /app/target/release/api-main .

CMD ["./api"]
