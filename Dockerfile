# Get started with a build env with Rust nightly,
# using bullseye because bookworm refused to build it 
FROM rustlang/rust:nightly-bookworm AS builder

# Install cargo-binstall, which makes it easier to install other
# cargo extensions like cargo-leptos
RUN wget --progress=dot:giga https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz && \
    tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz && \
    cp cargo-binstall /usr/local/cargo/bin && \
    # Install cargo-leptos
    cargo binstall cargo-leptos -y && \
    # Add the WASM target
    rustup target add wasm32-unknown-unknown && \
    # Make an /app dir, which everything will eventually live in
    mkdir -p /app

WORKDIR /app

# Copy the application source code
COPY . .

# Build the app
RUN LEPTOS_TAILWIND_VERSION=v3.4.1 LEPTOS_HASH_FILES=true cargo leptos build -r -P -vv

FROM debian:bookworm-slim AS runner
# Copy the server binary to the /app directory
COPY --from=builder /app/target/ /app/
COPY --from=builder /app/.cargo/target/release/apis /app/
COPY --from=builder /app/.cargo/target/release/hash.txt /app/
# /target/site contains our JS/WASM/CSS, etc.
COPY --from=builder /app/target/site /app/site
# Copy Cargo.toml if it’s needed at runtime
COPY --from=builder /app/Cargo.toml /app/
WORKDIR /app
# Install dependencies pinned to a certain version and delete lists
RUN apt-get update && \
    apt-get install --no-install-recommends libpq5 -y && \
    rm -rf /var/lib/apt/lists/*

# Set any required env variables
ENV LEPTOS_HASH_FILES=true
ENV RUST_LOG="info"
ENV APP_ENVIRONMENT="production"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT="site"
EXPOSE 8080

# Run the server
ENTRYPOINT ["/app/apis"]
