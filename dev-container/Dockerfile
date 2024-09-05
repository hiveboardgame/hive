FROM rustlang/rust:nightly-bullseye AS chef

WORKDIR /app

RUN rustup target add wasm32-unknown-unknown && \
    cargo install cargo-leptos && \
    cargo install diesel_cli --no-default-features --features postgres && \
    cargo install leptosfmt

COPY . /app
