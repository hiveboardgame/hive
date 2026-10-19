FROM rustlang/rust:nightly-bookworm

# Base tools and runtime deps (libpq for diesel_cli)
RUN apt-get update && \
    apt-get install --no-install-recommends -y \
      ca-certificates \
      wget \
      curl \
      pkg-config \
      build-essential \
      libssl-dev \
      libpq5 \
      libpq-dev && \
    rm -rf /var/lib/apt/lists/*

RUN wget --progress=dot:giga https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz && \
    tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz && \
    mv cargo-binstall /usr/local/cargo/bin && \
    rm -f cargo-binstall-x86_64-unknown-linux-musl.tgz

RUN cargo binstall -y \
      cargo-leptos && \
    cargo install diesel_cli --no-default-features --features postgres -j $(nproc)

RUN rustup target add wasm32-unknown-unknown

WORKDIR /app
