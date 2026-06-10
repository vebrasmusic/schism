# syntax=docker/dockerfile:1
#
# Multi-stage build for the Schism engine. Build context is this directory
# (apps/engine) — the crate is self-contained (no workspace), so nothing outside
# it is needed.
#
#   docker build -t schism-engine apps/engine
#   docker run --rm schism-engine run -n 150 > out.json   # the end tree on stdout
#
# Progress is written to stderr, so redirecting stdout gives a clean JSON readout.

# ---- builder ---------------------------------------------------------------
# Pinned to match the toolchain the crate is developed against (edition 2024
# needs Rust >= 1.85). Bump this alongside the local toolchain.
FROM rust:1.95-slim-bookworm AS builder

# Build tools for the AWS SDK's TLS backend (aws-lc-sys builds C via cmake).
# Part of the throwaway S3 experiment — drop this RUN with the aws-* deps.
RUN apt-get update \
  && apt-get install -y --no-install-recommends build-essential cmake perl \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy the manifest, lockfile, and sources. --locked below pins to Cargo.lock so
# the image build resolves the exact same dependency versions as local builds.
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Cache the cargo registry and the target dir across builds (BuildKit mounts).
# The release binary is copied out to a real path inside the same step, since the
# target mount itself is not persisted into the image layer.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/build/target \
  cargo build --release --locked \
  && cp target/release/engine /usr/local/bin/engine

# ---- runtime ---------------------------------------------------------------
# Slim Debian matching the builder's glibc (bookworm). The binary reads no files
# at runtime unless you pass `--config`, so nothing else needs to be copied in.
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

# Run as an unprivileged user.
RUN useradd --system --create-home --user-group --uid 10001 schism

COPY --from=builder /usr/local/bin/engine /usr/local/bin/engine

USER schism
WORKDIR /home/schism

# `engine` is the entrypoint; default subcommand is `run`. Override args at
# `docker run` time, e.g. `docker run --rm schism-engine run -p 1000 -s 42`.
ENTRYPOINT ["engine"]
CMD ["run"]
