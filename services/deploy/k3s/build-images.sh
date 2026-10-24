#!/usr/bin/env bash
# @cypher schema=1 source_sha256=4b28b89ebde4b1b1a960d1e83873ae6ce7c45d582d6b2655c52ea9aa1685ea21
# MERGE (self:File {path:"services/deploy/k3s/build-images.sh"})
# MERGE (main:Symbol {id:"services/deploy/k3s/build-images.sh::main",kind:"script"})
# MERGE (services:Symbol {id:"services/deploy/k3s/build-images.sh::ALL_SERVICES",kind:"array"})
# MERGE (self)-[:DEFINES]->(main)
# MERGE (self)-[:DEFINES]->(services)
# MERGE (main)-[:READS]->(services)
# MERGE (main)-[:CREATES]->(images:ExternalService {name:"dokipon/<service>:0.1.0"})
# @endcypher
# build-images.sh — turn the VM-local `cargo build --release` output into k3s
# images and import them into k3s containerd (namespace k8s.io).
#
# Why import instead of push: the images are built from a private, unpushed
# worktree. k3s manifests use `imagePullPolicy: Never`, so the images only ever
# need to exist in this node's containerd. Re-run this script after every
# `cargo build --release`.
#
# Usage:  bash build-images.sh            # build + import all 9
#         bash build-images.sh commerce-bff   # one service
set -euo pipefail

SRC_ROOT="${SRC_ROOT:-/home/ubuntu/dokipon/services}"
RELEASE_DIR="${SRC_ROOT}/target/release"
CTR="${CTR:-sudo /usr/local/bin/k3s ctr -n k8s.io images}"

ALL_SERVICES=(
  commerce-bff
  commerce-identity
  commerce-catalog
  commerce-cart
  commerce-inventory
  commerce-order
  commerce-payment-adapter
  commerce-admin
  commerce-content
)

SERVICES=("$@")
if [ "${#SERVICES[@]}" -eq 0 ]; then
  SERVICES=("${ALL_SERVICES[@]}")
fi

WORK="$(mktemp -d -p /home/ubuntu)"
trap 'rm -rf "$WORK"' EXIT

for svc in "${SERVICES[@]}"; do
  bin="${RELEASE_DIR}/${svc}"
  if [[ ! -x "$bin" ]]; then
    echo "  x missing ${bin} — run: cd ${SRC_ROOT} && cargo build --release" >&2
    exit 1
  fi

  # Minimal runtime: ubuntu base + the binary + its shared libraries. Copying the
  # resolved libs (rather than shipping a full distro) keeps each image ~30 MB.
  libdir="${WORK}/lib/${svc}"
  mkdir -p "${WORK}/root/bin" "${libdir}/usr/lib/x86_64-linux-gnu" "${libdir}/lib/x86_64-linux-gnu"
  cp "$bin" "${WORK}/root/bin/${svc}"
  for lib in $(ldd "$bin" | awk '/=>/ {print $3} /ld-linux/ {print $1}' | grep -v '^$'); do
    cp -n "$lib" "${libdir}${lib}" 2>/dev/null || true
  done
  # loader itself
  loader=$(ldd "$bin" | awk '/ld-linux/ {print $1}')
  cp -n "$loader" "${libdir}${loader}" 2>/dev/null || true

  tar -czf "${WORK}/root.tar.gz" -C "${WORK}/root" .
  tar -czf "${WORK}/libs.tar.gz" -C "${libdir}" .

  cat > "${WORK}/Dockerfile" <<EOF
FROM ubuntu:26.04
COPY libs.tar.gz /tmp/libs.tar.gz
RUN mkdir -p /nix && tar xzf /tmp/libs.tar.gz -C / && rm -f /tmp/libs.tar.gz
COPY root.tar.gz /tmp/root.tar.gz
RUN tar xzf /tmp/root.tar.gz -C / && rm -f /tmp/root.tar.gz
WORKDIR /
ENTRYPOINT ["/bin/${svc}"]
EOF

  # Image builder selection.
  #
  # nerdctl 2.x is NOT usable here: even with buildkitd running as root on
  # /run/buildkit/buildkitd.sock, `nerdctl --address <sock>` still resolves its
  # containerd endpoint from the rootless path and dies with
  #   rootless containerd not running?
  #     stat /run/user/1000/containerd-rootless: no such file or directory
  # so docker (apt package `docker.io`, which talks to its own daemon) is tried
  # first and nerdctl is only a fallback for a host that genuinely has a working
  # rootless containerd.
  IMAGE="dokipon/${svc}:0.1.0"
  if command -v docker >/dev/null 2>&1; then
    docker build -t "$IMAGE" "$WORK"
    docker save "$IMAGE" | sudo /usr/local/bin/k3s ctr -n k8s.io images import -
  elif command -v nerdctl >/dev/null 2>&1; then
    nerdctl build -t "$IMAGE" "$WORK"
    nerdctl save "$IMAGE" | sudo /usr/local/bin/k3s ctr -n k8s.io images import -
  else
    echo "  x no usable image builder on this VM (need docker, or nerdctl with a" >&2
    echo "    running rootless containerd)" >&2
    exit 1
  fi

  echo "  ok ${IMAGE}"
done

echo
echo "=== images in k3s containerd ==="
sudo /usr/local/bin/k3s ctr -n k8s.io images ls -q | grep dokipon || echo "(none)"
