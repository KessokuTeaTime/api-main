# --- Rust builder ---
FROM rust:1.82 AS rust_builder
WORKDIR /api
COPY . .
RUN cargo build --release

COPY --from=rust_builder /api/target/release/api .

CMD ["./api"]
