FROM rust:1.82 AS rust_builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

COPY . .
RUN cargo build --release

# -----------------------
FROM debian:bookworm-slim

WORKDIR /app
COPY --from=rust_builder /app/target/release/api-main .

CMD ["./api"]
