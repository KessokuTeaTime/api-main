# --- Rust builder ---
FROM rust:1.82 AS rust_builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM scratch
COPY --from=rust_builder /app/target/release/api .

CMD ["./api"]
