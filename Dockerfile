# --- Rust builder ---
FROM rust:1.82 as rust_builder
WORKDIR /app
COPY . .
RUN cargo build --release

# --- Final image ---
FROM debian:bookworm-slim
WORKDIR /app

COPY --from=rust_builder /app/target/release/api .

CMD ["./api"]
