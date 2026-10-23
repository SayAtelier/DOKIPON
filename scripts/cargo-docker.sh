#!/usr/bin/env bash
# Run cargo inside the cached rust:1.98.1 container, against this repo.
#
# The host has no Rust toolchain (and the network here is too flaky to install
# one), but the Docker image `rust:1.98.1-slim-bookworm` is already cached
# locally, and 1.98.1 is exactly what rust-toolchain.toml pins. So the local
# DoD (cargo check / cargo test) runs in the container.
#
# Usage:
#   scripts/cargo-docker.sh check --tests
#   scripts/cargo-docker.sh test --lib
#   scripts/cargo-docker.sh test --test '*' -- --test-threads=1
#
# The workspace is the `services/` directory; the root Cargo.toml is a separate
# package (the Vercel function) and is built with a different invocation --
# see scripts/cargo-docker.sh --function.

set -euo pipefail

REPO="${REPO:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
IMAGE="${CARGO_IMAGE:-rust:1.98.1-slim-bookworm}"
VOL="${CARGO_VOLUME:-dokipon-cargo-home}"

# Keep the cargo registry and the build directory on a volume so a second run
# does not re-download the registry or recompile every dependency.
docker volume create "$VOL" >/dev/null

WORKDIR=/work
if [ "${1:-}" = "--function" ]; then
  shift
  WORKDIR=/function
  exec docker run --rm \
    -v "$REPO:/function" \
    -v "$VOL:/cargo" \
    -e CARGO_HOME=/cargo \
    -e CARGO_TERM_COLOR=never \
    -w "$WORKDIR" \
    "$IMAGE" cargo "$@"
fi

exec docker run --rm \
  -v "$REPO:/work" \
  -v "$VOL:/cargo" \
  -e CARGO_HOME=/cargo \
  -e CARGO_TERM_COLOR=never \
  -w "$WORKDIR/services" \
  "$IMAGE" cargo "$@"
