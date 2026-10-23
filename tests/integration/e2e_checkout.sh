#!/usr/bin/env bash
# End-to-end checkout verification against the running services.
# Every assertion maps to a design-atlas case ID.
#
#   usage: bash tools/e2e_checkout.sh
#   usage: bash tests/integration/e2e_checkout.sh --mocks-up
#
# `--mocks-up` starts the 6 Rust mocks in `services/crates/commerce-mocks/`
# in the background, waits for their health, runs the same 22 assertions,
# then tears them down. The assertions themselves are unchanged — they
# exercise the order service contract regardless of whether the
# dependencies are real or mocked.
set -u

ORDER="http://127.0.0.1:8085"
# Native Windows curl and MSYS cat resolve /tmp to DIFFERENT real paths, so an
# absolute Windows path is the only thing both agree on.
TMPDIR_E2E="C:/Users/leo19/AppData/Local/Temp/gemarc-e2e"
mkdir -p "$TMPDIR_E2E"

# --- --mocks-up handling ----------------------------------------------
# When set, the 6 mock binaries are started on 9001..9006, waited on, and
# torn down at the end. The order service itself is not mocked here —
# this script assumes the real commerce-order is already on :8085.
MOCK_PIDS=()
MOCKS_UP=0
if [ "${1:-}" = "--mocks-up" ]; then
    MOCKS_UP=1
    : "${REPO_ROOT:=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)}"
    : "${MOCK_BUILD:=0}"   # set MOCK_BUILD=1 to rebuild before starting

    log_mock() { printf '[mock] %s\n' "$*" >&2; }

    # Resolve REPO_ROOT from the script's own location, then fall back to
    # the conventional layout if that fails (e.g. when the script is run
    # outside its worktree).
    REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." 2>/dev/null && pwd)"
    if [ -z "$REPO_ROOT" ] || [ ! -d "$REPO_ROOT/services" ]; then
        REPO_ROOT="D:/DOKIPON"
    fi

    start_mock() {
        local name="$1" bin="$2" port="$3"
        log_mock "starting $name on :$port"
        # Run the binary via cargo. We do not use a wrapper script; the cargo
        # invocation works whether the build cache is warm or cold. Stderr
        # captures build output so test logs stay readable.
        (
            cd "$REPO_ROOT/services"
            if [ "$MOCK_BUILD" = "1" ]; then
                cargo build -p "$bin" -q
            fi
            PORT="$port" cargo run -q -p "$bin" 2>"$TMPDIR_E2E/$name.build.log"
        ) >"$TMPDIR_E2E/$name.log" &
        MOCK_PIDS+=("$!")
    }

    wait_healthy() {
        local url="$1" tries=60
        while [ "$tries" -gt 0 ]; do
            if curl -s -o /dev/null -w "%{http_code}" "$url" 2>/dev/null | grep -q "^200$"; then
                return 0
            fi
            tries=$((tries - 1))
            sleep 1
        done
        return 1
    }

    # Resolve REPO_ROOT from the script's own location, then fall back to
    # the conventional layout if that fails (e.g. when the script is run
    # outside its worktree).
    REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." 2>/dev/null && pwd)"
    if [ -z "$REPO_ROOT" ] || [ ! -d "$REPO_ROOT/services" ]; then
        REPO_ROOT="D:/DOKIPON"
    fi

    start_mock "psp"            "mock-psp"            9001
    start_mock "broker"         "mock-broker"         9002
    start_mock "postgres"       "mock-postgres"       9003
    start_mock "line-identity"  "mock-line-identity"  9004
    start_mock "liff-webview"   "mock-liff-webview"   9005
    start_mock "ledger"         "mock-ledger"         9006

    for u in \
        http://127.0.0.1:9001/healthz \
        http://127.0.0.1:9002/healthz \
        http://127.0.0.1:9003/healthz \
        http://127.0.0.1:9004/healthz \
        http://127.0.0.1:9005/healthz \
        http://127.0.0.1:9006/healthz ; do
        wait_healthy "$u" || { log_mock "$u never came up"; exit 2; }
    done
    log_mock "all 6 mocks healthy"
fi

cleanup_mocks() {
    if [ "$MOCKS_UP" -eq 1 ]; then
        for pid in "${MOCK_PIDS[@]:-}"; do
            if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null || true
            fi
        done
    fi
}
trap cleanup_mocks EXIT
# --- end --mocks-up handling ------------------------------------------

# The service keeps idempotency records for 24h, so a second run with the same
# keys would (correctly) replay the first run's responses. Scope every key and
# session id to this run.
RUN="$(date +%s)$$"
k() { echo "$RUN-$1"; }
s() { echo "u-$RUN-$1"; }
PASS=0
FAIL=0

check() { # check <name> <expected> <actual>
  if [ "$2" = "$3" ]; then
    PASS=$((PASS + 1))
    printf '  PASS  %-46s %s\n' "$1" "$3"
  else
    FAIL=$((FAIL + 1))
    printf '  FAIL  %-46s expected=%s actual=%s\n' "$1" "$2" "$3"
  fi
}

submit() { # submit <session> <key> <simulated-outcome> <body>
  # Build the args array: bash cannot leave an empty argument behind when an
  # unquoted ${3:+...} expansion collapses to nothing, which silently drops the
  # simulation header and makes every run look "authorized".
  local args=(-s -X POST "$ORDER/v1/checkouts"
    -H "x-session-id: $1"
    -H "idempotency-key: $2"
    -H "content-type: application/json")
  if [ -n "$3" ]; then
    args+=(-H "x-simulated-payment: $3")
  fi
  args+=(-d "$4")
  curl "${args[@]}"
}

jq_get() { echo "$1" | python -c "import sys,json;d=json.load(sys.stdin);print(d.get('$2',''))" 2>/dev/null; }

echo
echo "=== E2E: checkout Saga ==="
echo

# ---------------------------------------------------------------- happy path
echo "CASE-ORDER-001  happy path reaches confirmed"
BODY='{"lines":[{"sku":"sku-bra-01","quantity":2,"unit_amount_minor":8900,"currency":"JPY"}],"payment_method_ref":"tok_test"}'
R=$(submit "$(s 1)" "$(k 1)" "" "$BODY")
check "status" "confirmed" "$(jq_get "$R" status)"
CID=$(jq_get "$R" checkoutId)
echo "        checkoutId=$CID"

# ------------------------------------------------------- idempotent replay
echo
echo "CASE-ORDER-008  same key + same payload replays"
R2=$(submit "$(s 1)" "$(k 1)" "" "$BODY")
check "same checkoutId returned" "$CID" "$(jq_get "$R2" checkoutId)"

echo
echo "CASE-ORDER-009  same key + different payload is 409"
BODY_DIFF='{"lines":[{"sku":"sku-bra-01","quantity":9,"unit_amount_minor":8900,"currency":"JPY"}],"payment_method_ref":"tok_test"}'
CODE=$(curl -s -o "$TMPDIR_E2E/e2e_409.json" -w "%{http_code}" -X POST "$ORDER/v1/checkouts" \
  -H "x-session-id: $(s 1)" -H "idempotency-key: $(k 1)" \
  -H "content-type: application/json" -d "$BODY_DIFF")
check "http status" "409" "$CODE"
check "error code" "checkout.idempotency_conflict" "$(jq_get "$(cat "$TMPDIR_E2E/e2e_409.json")" code)"

# ------------------------------------------------------------ key scoping
echo
echo "CASE-ORDER-010  key is scoped per buyer"
R3=$(submit "$(s 2)" "$(k 1)" "" "$BODY")
CID3=$(jq_get "$R3" checkoutId)
if [ "$CID" != "$CID3" ]; then
  PASS=$((PASS + 1)); echo "  PASS  different buyer gets a different checkout   $CID3"
else
  FAIL=$((FAIL + 1)); echo "  FAIL  different buyer reused the checkout"
fi

# ------------------------------------------------------------ declined path
echo
echo "CASE-ORDER-002  payment declined -> rejected"
R4=$(submit "$(s 3)" "$(k 3)" declined "$BODY")
check "status" "rejected" "$(jq_get "$R4" status)"

# ------------------------------------------------------- payment unknown
echo
echo "CASE-ORDER-003  payment timeout -> payment_unknown (not terminal, not failure)"
R5=$(submit "$(s 4)" "$(k 4)" unknown "$BODY")
check "status" "payment_unknown" "$(jq_get "$R5" status)"
CID5=$(jq_get "$R5" checkoutId)

S=$(curl -s "$ORDER/v1/checkouts/$CID5" -H "x-session-id: $(s 4)")
check "pollable (terminal=false)" "False" "$(jq_get "$S" terminal)"
check "awaitingReconciliation" "True" "$(jq_get "$S" awaitingReconciliation)"

# ---------------------------------------- reconciliation resolves the unknown
echo
echo "CASE-ORDER-004  reconcile(authorized) -> confirmed"
ORD5=$(jq_get "$S" orderId)
R6=$(curl -s -X POST "$ORDER/v1/orders/$ORD5/reconcile" \
  -H "x-session-id: $(s 4)" -H "content-type: application/json" \
  -d '{"authorized":true}')
check "state" "confirmed" "$(jq_get "$R6" state)"
S6=$(curl -s "$ORDER/v1/checkouts/$CID5" -H "x-session-id: $(s 4)")
check "awaitingReconciliation cleared" "False" "$(jq_get "$S6" awaitingReconciliation)"
check "terminal now" "True" "$(jq_get "$S6" terminal)"

echo
echo "CASE-ORDER-005  reconcile(declined) -> rejected"
R7=$(submit "$(s 8)" "$(k 8)" unknown "$BODY")
ORD8=$(jq_get "$(curl -s "$ORDER/v1/checkouts/$(jq_get "$R7" checkoutId)" -H "x-session-id: $(s 8)")" orderId)
R8=$(curl -s -X POST "$ORDER/v1/orders/$ORD8/reconcile" \
  -H "x-session-id: $(s 8)" -H "content-type: application/json" \
  -d '{"authorized":false}')
check "state" "rejected" "$(jq_get "$R8" state)"

echo
echo "SEC-07  reconcile on somebody else's order is refused"
CODE=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$ORDER/v1/orders/$ORD5/reconcile" \
  -H "x-session-id: attacker-$RUN" -H "content-type: application/json" -d '{"authorized":true}')
check "http status" "403" "$CODE"

echo
echo "CASE-ORDER-006  reconcile on a non-unknown checkout is refused"
CID1STATUS=$(curl -s "$ORDER/v1/checkouts/$CID" -H "x-session-id: $(s 1)" | python -c "import sys,json;print(json.load(sys.stdin)['orderId'])")
CODE=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$ORDER/v1/orders/$CID1STATUS/reconcile" \
  -H "x-session-id: $(s 1)" -H "content-type: application/json" -d '{"authorized":true}')
check "http status" "409" "$CODE"

# ------------------------------------------------------------------- IDOR
echo
echo "CASE-ORDER-014 / SEC-07  another session cannot read the order"
CODE=$(curl -s -o /dev/null -w "%{http_code}" "$ORDER/v1/checkouts/$CID" -H "x-session-id: attacker-$RUN")
check "http status" "403" "$CODE"

echo
echo "CASE-ORDER-014  order listing is scoped to the buyer"
N1=$(curl -s "$ORDER/v1/orders" -H "x-session-id: $(s 1)" | python -c "import sys,json;print(len(json.load(sys.stdin)['items']))")
N2=$(curl -s "$ORDER/v1/orders" -H "x-session-id: $(s 9)" | python -c "import sys,json;print(len(json.load(sys.stdin)['items']))")
# (s 1) made exactly one checkout in this run; the reconcile cases belong to
# different sessions, so the owner must still see exactly one.
check "owner's order count" "1" "$N1"
check "stranger's order count" "0" "$N2"

# --------------------------------------------------------- mixed currency
echo
echo "CASE-ORDER-011  mixed-currency order is rejected"
BODY_MIX='{"lines":[{"sku":"a","quantity":1,"unit_amount_minor":100,"currency":"JPY"},{"sku":"b","quantity":1,"unit_amount_minor":100,"currency":"USD"}],"payment_method_ref":"tok"}'
CODE=$(curl -s -o "$TMPDIR_E2E/e2e_mix.json" -w "%{http_code}" -X POST "$ORDER/v1/checkouts" \
  -H "x-session-id: $(s 5)" -H "idempotency-key: key-e2e-5" \
  -H "content-type: application/json" -d "$BODY_MIX")
check "http status" "422" "$CODE"
check "error code" "order.mixed_currency" "$(jq_get "$(cat "$TMPDIR_E2E/e2e_mix.json")" code)"

# --------------------------------------------------------------- empty cart
echo
echo "CASE-ORDER-012  empty cart is rejected"
CODE=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$ORDER/v1/checkouts" \
  -H "x-session-id: $(s 6)" -H "idempotency-key: key-e2e-6" \
  -H "content-type: application/json" -d '{"lines":[],"payment_method_ref":"tok"}')
check "http status" "422" "$CODE"

# ------------------------------------------------------- missing idem key
echo
echo "FR-SECURITY-03  write without Idempotency-Key is refused"
CODE=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$ORDER/v1/checkouts" \
  -H "x-session-id: $(s 7)" -H "content-type: application/json" -d "$BODY")
check "http status" "400" "$CODE"

echo
echo "=== RESULT: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] || exit 1
