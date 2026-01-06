# rust builder

FROM rust:bookworm AS rust_builder

ARG GIT_COMMIT

ENV GIT_COMMIT=$GIT_COMMIT

WORKDIR /app

COPY rust-toolchain.toml ./
RUN rustup toolchain install --profile minimal $(grep "channel" rust-toolchain.toml | cut -d'"' -f2)

COPY . .
RUN cargo build --release

# runtime image

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=rust_builder /app/target/release/api-main .
RUN apt-get update && \
    apt-get install -y ca-certificates curl gnupg lsb-release && \
    install -m 0755 -d /etc/apt/keyrings && \
    curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/debian $(lsb_release -cs) stable" \
      > /etc/apt/sources.list.d/docker.list && \
    apt-get update && \
    apt-get install -y docker-ce-cli && \
    rm -rf /var/lib/apt/lists/*

# commands

CMD ["./api-main"]
