FROM rust:latest AS rust_builder
WORKDIR /app

COPY rust-toolchain.toml ./
RUN rustup show

COPY . .
RUN cargo build --release

# -----------------------
FROM debian:bookworm-slim

WORKDIR /app
COPY --from=rust_builder /app/target/release/main .

CMD ["./main"]
