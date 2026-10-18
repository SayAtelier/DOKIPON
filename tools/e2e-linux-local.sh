#!/usr/bin/env bash
# e2e-linux-local.sh — run the existing checkout e2e suite against a k3s
# cluster from the host, without modifying tools/e2e_checkout.sh.
#
# tools/e2e_checkout.sh is a tracked artifact whose line 8-10 comment explains
# why its TMPDIR is an absolute Windows path: native Windows curl and MSYS cat
# resolve /tmp differently, so only a full Windows path satisfies both. That is
# correct for its intended host and wrong everywhere else.
#
# This wrapper reproduces the suite on Linux with exactly two substitutions and
# nothing else. Every assertion, test id, and expected value is untouched; the
# diff against the original is verifiable:
#
#   diff <(sed -n '7p;10p' tools/e2e_checkout.sh) <(sed -n '7p;10p' <this-derived-file>)
#
# Substitutions:
#   line  7  ORDER      -> ${ORDER_URL:-http://127.0.0.1:8085}
#   line 10  TMPDIR_E2E -> ${TMPDIR:-/tmp}/gemarc-e2e
#
# Prerequisites (k3s):
#   # expose the ClusterIP order service on the host loopback
#   sudo k3s kubectl port-forward -n commerce svc/commerce-order 8085:8085 --address 127.0.0.1 &
#
#   # the suite calls `python`, Ubuntu ships python3
#   sudo ln -sf /usr/bin/python3 /usr/local/bin/python
#
# Usage:
#   bash tools/e2e-linux-local.sh
#   ORDER_URL=http://127.0.0.1:18085 bash tools/e2e-linux-local.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ORIGINAL="${REPO_ROOT}/tools/e2e_checkout.sh"

if [[ ! -f "$ORIGINAL" ]]; then
  echo "x cannot find ${ORIGINAL}" >&2
  exit 2
fi

DERIVED="$(mktemp -t e2e-checkout-XXXXXX.sh)"
trap 'rm -f "$DERIVED"' EXIT

sed \
  -e 's|^ORDER=.*|ORDER="${ORDER_URL:-http://127.0.0.1:8085}"|' \
  -e 's|^TMPDIR_E2E=.*|TMPDIR_E2E="${TMPDIR:-/tmp}/gemarc-e2e"|' \
  "$ORIGINAL" > "$DERIVED"

# Fail loudly if the port-forward is not up, instead of reporting 22 confusing
# connection failures.
if ! curl -s -m 5 -o /dev/null "${ORDER_URL:-http://127.0.0.1:8085}/healthz"; then
  cat >&2 <<'MSG'
x cannot reach the order service.

  Start the port-forward first:
    sudo k3s kubectl port-forward -n commerce svc/commerce-order 8085:8085 --address 127.0.0.1 &

  Or point ORDER_URL at a reachable address.
MSG
  exit 3
fi

echo "=== derived script: only lines 7 and 10 differ from the original ==="
diff <(sed -n '7p;10p' "$ORIGINAL") <(sed -n '7p;10p' "$DERIVED") || true
echo

bash "$DERIVED"
