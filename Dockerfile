# --- Rust builder ---
FROM rust:1.82 AS rust_builder
WORKDIR /app
RUN cargo build --release

FROM scratch
COPY --from=rust_builder /target/release/api /api

CMD ["./api"]
