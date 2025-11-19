FROM rust:latest AS rust_builder
WORKDIR /app

COPY rust-toolchain.toml ./
RUN rustup show

COPY . .
RUN cargo build --release
RUN echo "=== target dir ===" && ls -R /app/target

# -----------------------
FROM debian:bookworm-slim

WORKDIR /app
COPY --from=rust_builder /app/target/release/api-main .

CMD ["./api"]
