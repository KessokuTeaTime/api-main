# --- Rust builder ---
FROM rust:1.82 AS rust_builder
COPY . .
RUN cargo build --release

FROM scratch
COPY --from=rust_builder /target/release/api /api

CMD ["./api"]
