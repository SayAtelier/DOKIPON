#!/usr/bin/env bash
# setup-commerce-tls.sh — issue the Gateway's HTTPS certificate.
#
# The internal CA (rust-game-server/rgs-secret-ca) has to be copied into the
# commerce namespace, because a cert-manager Issuer can only sign Certificates
# in its own namespace. This is a copy of the same Secret, so the issued
# certificate chains to exactly the same trust anchor as the six RGS domain
# certificates.
#
# The CA private key never enters the repository. It is read from the live
# cluster and written back into the cluster, and the Secret is created with
# `kubectl create` rather than `apply` so kubectl does not inject a
# last-applied-configuration annotation containing the key.
#
# The Issuer and the Certificate live in
# docs/deploy/03-platform/envoy/20-commerce-tls.yaml. The Secret does not:
# applying a manifest that declares it would overwrite the populated Secret with
# an empty one, because the manifest has no data section (the key must not be
# committed).
#
# Usage (on the VM):
#   bash setup-commerce-tls.sh
#
# The internal CA is self-signed, so a browser will warn on
# https://shop.local. That is expected for a local dev cluster. For production,
# point issuerRef at a cert-manager ACME Issuer (Let's Encrypt / ZeroSSL); no
# other field needs to change.

set -euo pipefail

k3s() { sudo /usr/local/bin/k3s kubectl "$@"; }
NS=commerce
SRC_NS=rust-game-server
SRC=rgs-secret-ca
DST=commerce-ca

echo "=== 1. copy CA secret ${SRC_NS}/${SRC} -> ${NS}/${DST} ==="
# The CA secret is kubernetes.io/tls with ca.crt / tls.crt / tls.key.
ca_crt="$(k3s -n "$SRC_NS" get secret "$SRC" -o jsonpath='{.data.ca\.crt}')"
tls_crt="$(k3s -n "$SRC_NS" get secret "$SRC" -o jsonpath='{.data.tls\.crt}')"
tls_key="$(k3s -n "$SRC_NS" get secret "$SRC" -o jsonpath='{.data.tls\.key}')"

if [ -z "$ca_crt" ] || [ -z "$tls_key" ]; then
  echo "x source secret is missing ca.crt or tls.key" >&2
  exit 3
fi

# kubectl reads the files itself, so process substitution does not survive: it
# opens /dev/fd/63 in the kubectl process, which is not the shell's fd table.
tmpdir="$(mktemp -d)"
printf '%s' "$tls_crt" | base64 -d > "${tmpdir}/tls.crt"
printf '%s' "$tls_key" | base64 -d > "${tmpdir}/tls.key"
chmod 600 "${tmpdir}/tls.key"

k3s -n "$NS" delete secret "$DST" --ignore-not-found >/dev/null 2>&1 || true
k3s -n "$NS" create secret tls "$DST" \
  --cert="${tmpdir}/tls.crt" --key="${tmpdir}/tls.key" >/dev/null
rm -rf "$tmpdir"

# create secret tls writes tls.crt/tls.key; the CA needs ca.crt as well.
k3s -n "$NS" patch secret "$DST" --type=merge \
  -p "{\"data\":{\"ca.crt\":\"${ca_crt}\"}}" >/dev/null

echo "  keys: $(k3s -n "$NS" get secret "$DST" -o jsonpath='{.data}' | python3 -c 'import json,sys; print(sorted(json.load(sys.stdin).keys()))')"

echo "=== 2. apply issuer + certificate ==="
k3s apply -f /home/ubuntu/dokipon/docs/deploy/03-platform/envoy/20-commerce-tls.yaml 2>&1 | tail -3

echo "=== 3. wait for the certificate ==="
for i in $(seq 1 30); do
  ready="$(k3s -n "$NS" get certificate commerce-wildcard-tls -o jsonpath='{.status.conditions[?(@.type=="Ready")].status}' 2>/dev/null || true)"
  [ "$ready" = "True" ] && break
  sleep 2
done
k3s -n "$NS" get certificate commerce-wildcard-tls 2>&1

echo "=== 4. verify the chain ==="
k3s -n "$NS" get secret commerce-wildcard-tls -o jsonpath='{.data.tls\.crt}' | base64 -d > /tmp/commerce.crt
k3s -n "$NS" get secret commerce-ca -o jsonpath='{.data.ca\.crt}' | base64 -d > /tmp/commerce-ca.crt
openssl verify -CAfile /tmp/commerce-ca.crt /tmp/commerce.crt
openssl x509 -in /tmp/commerce.crt -noout -subject -issuer -dates -ext subjectAltName 2>/dev/null | head -8
