# Get started with a build env with Rust nightly,
# using bullseye because bookworm refused to build it 
FROM rustlang/rust:nightly-bullseye as chef

# Install cargo-binstall, which makes it easier to install other
# cargo extensions like cargo-leptos
RUN wget --progress=dot:giga https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz && \
    tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz && \
    cp cargo-binstall /usr/local/cargo/bin && \
    # Install cargo-leptos and chef
    cargo binstall cargo-leptos -y && \
    cargo binstall cargo-chef -y && \
    # Add the WASM target
    rustup target add wasm32-unknown-unknown && \
    # Make an /app dir, which everything will eventually live in
    mkdir -p /app

WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --all-features --recipe-path recipe.json
# Build application
COPY . .

# Build the app
RUN cargo leptos build --release -vv

FROM debian:bookworm-slim as runner
# Copy the server binary to the /app directory
COPY --from=builder /app/target/release/apis /app/
# /target/site contains our JS/WASM/CSS, etc.
COPY --from=builder /app/target/site /app/site
# Copy Cargo.toml if it’s needed at runtime
COPY --from=builder /app/Cargo.toml /app/
WORKDIR /app
# Install dependencies pinned to a certain version and delete lists
RUN apt-get update && \
    apt-get install --no-install-recommends libpq5 -y && \
    rm -rf /var/lib/apt/lists/*

# Set any required env variables and
ENV RUST_LOG="info"
ENV APP_ENVIRONMENT="production"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT="site"
EXPOSE 8080

# Run the server
ENTRYPOINT ["/app/apis"]