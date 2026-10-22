#!/usr/bin/env bash
# e2e_admin_in_cluster.sh — run tools/e2e_admin_smoke.sh against the deployed
# k3s cluster, with the operator credential read straight from the Secret.
#
# Two environment facts this wrapper exists to encode.
#
# 1. commerce-order scopes every read by buyer session; CASE-ORDER-014 asserts it
#    ("another session cannot read the order" -> 403). commerce-admin therefore
#    reads through a single fixed session (admin_session_id, default
#    admin-console), so it only ever sees orders placed under that session. The
#    smoke suite asserts a non-payment_unknown order exists, which means an order
#    must exist under admin-console before the suite can pass. Verified by
#    experiment: after seeding one, admin lists 7; without it, items is []. That
#    empty list is correct behaviour, not a broken deployment.
#
# 2. The credential must not be passed through
#    `multipass exec -- bash -lc 'ADMIN_PASSWORD=$PW ...'`. That value crosses
#    three shells of quoting before bash sees it and comes back empty, which
#    surfaces as the misleading
#    "Set ADMIN_PASSWORD to the operator credential" error even when the Secret
#    is present. Reading it inside the script removes the extra shells.
#
# Usage (on the host):
#   multipass transfer 'D:\repo\tools\e2e_admin_in_cluster.sh' <vm>:/home/ubuntu/
#   multipass exec <vm> -- sudo bash /home/ubuntu/e2e_admin_in_cluster.sh
set -euo pipefail

# Overridable so the script is not welded to one machine's layout.
REPO_ROOT="${REPO_ROOT:-/home/ubuntu/dokipon}"
k3s() { sudo /usr/local/bin/k3s kubectl "$@"; }
NS=commerce
SVC=commerce-admin
PORT=8087

k3s -n "$NS" port-forward "svc/$SVC" "${PORT}:8087" --address 127.0.0.1 >/var/tmp/pf-admin.log 2>&1 &
PF=$!
trap 'kill $PF 2>/dev/null || true' EXIT

for _ in $(seq 1 20); do
  if curl -s -m 2 -o /dev/null "http://127.0.0.1:${PORT}/healthz"; then break; fi
  sleep 1
done

if ! curl -s -m 3 -o /dev/null "http://127.0.0.1:${PORT}/healthz"; then
  echo "x port-forward to $SVC never came up" >&2
  cat /var/tmp/pf-admin.log >&2
  exit 3
fi

export ADMIN_USER=operator
export ADMIN_URL="http://127.0.0.1:${PORT}"
# The Secret keys are UPPERCASE (ADMIN_USER, OPERATOR_PASSWORD, ...); a
# lowercase key silently yields an empty string and the suite then reports the
# credential is missing.
export ADMIN_PASSWORD="$(k3s -n "$NS" get secret commerce-admin-credentials \
  -o jsonpath="{.data.OPERATOR_PASSWORD}" | base64 -d)"

if [ -z "$ADMIN_PASSWORD" ]; then
  echo "x Secret commerce-admin-credentials has no operator-password key" >&2
  exit 4
fi

# Seed one order under the admin-console session. commerce-order scopes reads by
# buyer session, so without this the suite's "order list contains no
# non-payment_unknown order" assertion fails on an empty list rather than on a
# broken deployment.
k3s -n commerce port-forward svc/commerce-order 8085:8085 --address 127.0.0.1 >/var/tmp/pf-seed.log 2>&1 &
SEEDPF=$!
for _ in $(seq 1 20); do curl -s -m 2 -o /dev/null http://127.0.0.1:8085/healthz && break; sleep 1; done
curl -s -o /dev/null -X POST http://127.0.0.1:8085/v1/checkouts \
  -H 'x-session-id: admin-console' \
  -H "idempotency-key: seed-$RANDOM$RANDOM" \
  -H 'content-type: application/json' \
  -d '{"lines":[{"sku":"sku-bra-01","quantity":1,"unit_amount_minor":8900,"currency":"JPY"}],"payment_method_ref":"tok_test"}'
kill $SEEDPF 2>/dev/null || true

cd "${REPO_ROOT:-/home/ubuntu/dokipon}"
bash tools/e2e_admin_smoke.sh
