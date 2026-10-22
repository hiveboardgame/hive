FROM debian:bookworm AS builder

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN apt-get update && \
    apt-get install --no-install-recommends -y \
      build-essential ca-certificates curl wget pkg-config libssl-dev libpq-dev && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY rust-toolchain.toml ./
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --profile minimal --default-toolchain none && \
    rustup toolchain install --no-self-update

# Install cargo-binstall, which makes it easier to install other
# cargo extensions like cargo-leptos
RUN wget --progress=dot:giga https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz && \
    tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz && \
    cp cargo-binstall /usr/local/cargo/bin && \
    # Install cargo-leptos
    cargo binstall cargo-leptos -y

# Copy the application source code
COPY . .

# Build the app
RUN LEPTOS_HASH_FILES=true cargo leptos build -r -P -vv

FROM debian:bookworm-slim AS runner
# Copy only the runtime artifacts from the builder stage.
COPY --from=builder /app/.cargo/target/release/apis /app/apis
COPY --from=builder /app/.cargo/target/release/hash.txt /app/hash.txt
# /target/site contains our JS/WASM/CSS, etc.
COPY --from=builder /app/target/site /app/site
# Copy Cargo.toml if it’s needed at runtime
COPY --from=builder /app/Cargo.toml /app/
WORKDIR /app
# Install dependencies pinned to a certain version and delete lists
RUN apt-get update && \
    apt-get install --no-install-recommends libpq5 ca-certificates -y && \
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
