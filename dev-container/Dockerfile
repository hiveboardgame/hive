FROM debian:bookworm-slim AS base

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# Build dependencies for the application itself.
RUN apt-get update && \
    apt-get install --no-install-recommends -y \
      ca-certificates curl pkg-config \
      libssl-dev libpq-dev build-essential && \
    rm -rf /var/lib/apt/lists/*

# Share rustup without installing a compiler in the base image.
RUN bash -o pipefail -c ' \
    curl --fail --location --silent --show-error \
      --proto "=https" --tlsv1.2 https://sh.rustup.rs | \
      sh -s -- -y --no-modify-path --default-toolchain none --profile minimal'

# Build CLI tools independently of the application's pinned toolchain.
FROM base AS cli-tools
RUN rustup default stable

RUN bash -o pipefail -c ' \
    curl --fail --location --silent --show-error \
      --proto "=https" --tlsv1.2 https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | \
      bash'

RUN cargo binstall -y cargo-leptos

RUN cargo install diesel_cli --version '~2.3.0' --locked --no-default-features --features postgres

ARG TEST_MODE_DOCKER=false
RUN if [ "$TEST_MODE_DOCKER" = "true" ]; then cargo binstall leptosfmt; fi

# Keep the CLI compiler and build artifacts out of the development image.
FROM base AS development
WORKDIR /app
COPY rust-toolchain.toml ./
RUN rustup show

# Copy executables only; keep toolchain installation cached when CLI tools change.
COPY --from=cli-tools /usr/local/cargo/bin/ /usr/local/cargo/bin/
