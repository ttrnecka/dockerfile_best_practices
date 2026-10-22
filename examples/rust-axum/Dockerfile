# syntax=docker/dockerfile:1.7

FROM lukemathwalker/cargo-chef:0.1.78-rust-1.99.1-trixie AS chef
WORKDIR /app

FROM chef AS planner
COPY . ./
RUN --mount=type=secret,id=cargo-ca,target=/run/secrets/cargo-ca,required=false \
    if [ -s /run/secrets/cargo-ca ]; then \
        CARGO_HTTP_CAINFO=/run/secrets/cargo-ca cargo chef prepare --bin rust-api --recipe-path recipe.json; \
    else \
        cargo chef prepare --bin rust-api --recipe-path recipe.json; \
    fi

FROM chef AS builder
COPY --from=planner /app/recipe.json ./recipe.json
RUN --mount=type=secret,id=cargo-ca,target=/run/secrets/cargo-ca,required=false \
    if [ -s /run/secrets/cargo-ca ]; then \
        CARGO_HTTP_CAINFO=/run/secrets/cargo-ca cargo chef cook --release --locked --bin rust-api --recipe-path recipe.json; \
    else \
        cargo chef cook --release --locked --bin rust-api --recipe-path recipe.json; \
    fi
COPY . ./
RUN --mount=type=secret,id=cargo-ca,target=/run/secrets/cargo-ca,required=false \
    if [ -s /run/secrets/cargo-ca ]; then \
        CARGO_HTTP_CAINFO=/run/secrets/cargo-ca cargo build --release --locked --bin rust-api; \
    else \
        cargo build --release --locked --bin rust-api; \
    fi

FROM debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rust-api /usr/local/bin/rust-api

ENV PORT=8080
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/rust-api"]