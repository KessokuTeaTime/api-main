# --- Rust builder ---
FROM rust:1.82 AS rust_builder
WORKDIR /api
RUN cargo build --release

FROM scratch
COPY --from=rust_builder /api/target/release/api /api

CMD ["./api"]
