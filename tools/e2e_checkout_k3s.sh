#!/usr/bin/env bash
# @cypher schema=1 source_sha256=12f76a202bc98003b0daba52c3dba5bbc3bc61d5237d146eb6ea17f72062c719
# MERGE (self:File {path:"tools/e2e_checkout_k3s.sh"})
# MERGE (main:Symbol {id:"tools/e2e_checkout_k3s.sh::main",kind:"script"})
# MERGE (check:Symbol {id:"tools/e2e_checkout_k3s.sh::check",kind:"function"})
# MERGE (submit:Symbol {id:"tools/e2e_checkout_k3s.sh::submit",kind:"function"})
# MERGE (jq_get:Symbol {id:"tools/e2e_checkout_k3s.sh::jq_get",kind:"function"})
# MERGE (curl:Symbol {id:"tools/e2e_checkout_k3s.sh::curl",kind:"function"})
# MERGE (key:Symbol {id:"tools/e2e_checkout_k3s.sh::k",kind:"function"})
# MERGE (session:Symbol {id:"tools/e2e_checkout_k3s.sh::s",kind:"function"})
# MERGE (pass_count:Symbol {id:"tools/e2e_checkout_k3s.sh::PASS",kind:"variable"})
# MERGE (fail_count:Symbol {id:"tools/e2e_checkout_k3s.sh::FAIL",kind:"variable"})
# MERGE (last_response:Symbol {id:"tools/e2e_checkout_k3s.sh::LAST_RESPONSE",kind:"variable"})
# MERGE (order_url:Config {name:"ORDER_URL"})
# MERGE (tmpdir:Config {name:"TMPDIR"})
# MERGE (curl_cli:ExternalService {name:"curl"})
# MERGE (python:ExternalService {name:"python"})
# MERGE (self)-[:DEFINES]->(main)
# MERGE (self)-[:DEFINES]->(check)
# MERGE (self)-[:DEFINES]->(submit)
# MERGE (self)-[:DEFINES]->(jq_get)
# MERGE (self)-[:DEFINES]->(curl)
# MERGE (self)-[:DEFINES]->(key)
# MERGE (self)-[:DEFINES]->(session)
# MERGE (self)-[:DEFINES]->(pass_count)
# MERGE (self)-[:DEFINES]->(fail_count)
# MERGE (self)-[:DEFINES]->(last_response)
# MERGE (main)-[:READS]->(order_url)
# MERGE (main)-[:READS]->(tmpdir)
# MERGE (main)-[:CALLS]->(check)
# MERGE (main)-[:CALLS]->(submit)
# MERGE (main)-[:CALLS]->(jq_get)
# MERGE (main)-[:CALLS]->(curl)
# MERGE (main)-[:CALLS]->(key)
# MERGE (main)-[:CALLS]->(session)
# MERGE (submit)-[:CALLS]->(curl)
# MERGE (curl)-[:DEPENDS_ON]->(curl_cli)
# MERGE (jq_get)-[:DEPENDS_ON]->(python)
# MERGE (check)-[:WRITES]->(pass_count)
# MERGE (check)-[:WRITES]->(fail_count)
# MERGE (check)-[:READS]->(last_response)
# MERGE (self)-[:CONFIGURES]->(order:ExternalService {name:"commerce-order"})
# @endcypher
# End-to-end checkout verification against the running services.
# Every assertion maps to a design-atlas case ID.
# Usage (terminal 1): kubectl -n commerce port-forward svc/commerce-order 18085:8085
# Usage (terminal 2): ORDER_URL=http://127.0.0.1:18085 bash tools/e2e_checkout_k3s.sh [--verbose]
# The default ORDER_URL resolves only inside the commerce namespace.
set -euo pipefail

VERBOSE=false
if (( $# > 1 )); then
  echo "usage: $0 [--verbose]" >&2
  exit 2
fi
if (( $# == 1 )); then
  if [[ "$1" != "--verbose" ]]; then
    echo "usage: $0 [--verbose]" >&2
    exit 2
  fi
  VERBOSE=true
fi

ORDER="${ORDER_URL:-http://commerce-order:8085}"
TMPDIR_E2E="${TMPDIR:-/tmp}/gemarc-e2e"
mkdir -p "$TMPDIR_E2E"
LAST_RESPONSE=""

# Keep transport failures in the assertion path so the suite still prints counts.
curl() {
  command curl "$@" || true
}

# The service keeps idempotency records for 24h, so a second run with the same
# keys would (correctly) replay the first run's responses. Scope every key and
# session id to this run.
RUN="$(date +%s)$$"
k() { echo "$RUN-$1"; }
s() { echo "u-$RUN-$1"; }
PASS=0
FAIL=0

check() { # check <name> <expected> <actual>
  local name=$1 expected=$2 actual=$3
  if [ "$expected" = "$actual" ]; then
    PASS=$((PASS + 1))
    printf '  PASS  %-46s %s\n' "$name" "$actual"
  else
    FAIL=$((FAIL + 1))
    printf '  FAIL  %-46s expected=%s actual=%s\n' "$name" "$expected" "$actual"
    if [[ "$VERBOSE" == true ]]; then
      printf '        raw response:\n%s\n' "${LAST_RESPONSE:-<empty response>}"
    fi
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

jq_get() { echo "$1" | python -c "import sys,json;d=json.load(sys.stdin);print(d.get('$2',''))" 2>/dev/null || true; }

echo
echo "=== E2E: checkout Saga ==="
echo

# ---------------------------------------------------------------- happy path
echo "CASE-ORDER-001  happy path reaches confirmed"
BODY='{"lines":[{"sku":"sku-bra-01","quantity":2,"unit_amount_minor":8900,"currency":"JPY"}],"payment_method_ref":"tok_test"}'
R=$(submit "$(s 1)" "$(k 1)" "" "$BODY")
LAST_RESPONSE="$R"
check "status" "confirmed" "$(jq_get "$R" status)"
CID=$(jq_get "$R" checkoutId)
echo "        checkoutId=$CID"

# ------------------------------------------------------- idempotent replay
echo
echo "CASE-ORDER-008  same key + same payload replays"
R2=$(submit "$(s 1)" "$(k 1)" "" "$BODY")
LAST_RESPONSE="$R2"
check "same checkoutId returned" "$CID" "$(jq_get "$R2" checkoutId)"

echo
echo "CASE-ORDER-009  same key + different payload is 409"
BODY_DIFF='{"lines":[{"sku":"sku-bra-01","quantity":9,"unit_amount_minor":8900,"currency":"JPY"}],"payment_method_ref":"tok_test"}'
CODE=$(curl -s -o "$TMPDIR_E2E/e2e_409.json" -w "%{http_code}" -X POST "$ORDER/v1/checkouts" \
  -H "x-session-id: $(s 1)" -H "idempotency-key: $(k 1)" \
  -H "content-type: application/json" -d "$BODY_DIFF")
LAST_RESPONSE="$(cat "$TMPDIR_E2E/e2e_409.json")"
check "http status" "409" "$CODE"
check "error code" "checkout.idempotency_conflict" "$(jq_get "$(cat "$TMPDIR_E2E/e2e_409.json")" code)"

# ------------------------------------------------------------ key scoping
echo
echo "CASE-ORDER-010  key is scoped per buyer"
R3=$(submit "$(s 2)" "$(k 1)" "" "$BODY")
CID3=$(jq_get "$R3" checkoutId)
LAST_RESPONSE="$R3"
if [ "$CID" != "$CID3" ]; then
  PASS=$((PASS + 1)); echo "  PASS  different buyer gets a different checkout   $CID3"
else
  FAIL=$((FAIL + 1)); echo "  FAIL  different buyer reused the checkout"
  if [[ "$VERBOSE" == true ]]; then
    printf '        raw response:\n%s\n' "$R3"
  fi
fi

# ------------------------------------------------------------ declined path
echo
echo "CASE-ORDER-002  payment declined -> rejected"
R4=$(submit "$(s 3)" "$(k 3)" declined "$BODY")
LAST_RESPONSE="$R4"
check "status" "rejected" "$(jq_get "$R4" status)"

# ------------------------------------------------------- payment unknown
echo
echo "CASE-ORDER-003  payment timeout -> payment_unknown (not terminal, not failure)"
R5=$(submit "$(s 4)" "$(k 4)" unknown "$BODY")
LAST_RESPONSE="$R5"
check "status" "payment_unknown" "$(jq_get "$R5" status)"
CID5=$(jq_get "$R5" checkoutId)

S=$(curl -s "$ORDER/v1/checkouts/$CID5" -H "x-session-id: $(s 4)")
LAST_RESPONSE="$S"
check "pollable (terminal=false)" "False" "$(jq_get "$S" terminal)"
check "awaitingReconciliation" "True" "$(jq_get "$S" awaitingReconciliation)"

# ---------------------------------------- reconciliation resolves the unknown
echo
echo "CASE-ORDER-004  reconcile(authorized) -> confirmed"
ORD5=$(jq_get "$S" orderId)
R6=$(curl -s -X POST "$ORDER/v1/orders/$ORD5/reconcile" \
  -H "x-session-id: $(s 4)" -H "content-type: application/json" \
  -d '{"authorized":true}')
LAST_RESPONSE="$R6"
check "state" "confirmed" "$(jq_get "$R6" state)"
S6=$(curl -s "$ORDER/v1/checkouts/$CID5" -H "x-session-id: $(s 4)")
LAST_RESPONSE="$S6"
check "awaitingReconciliation cleared" "False" "$(jq_get "$S6" awaitingReconciliation)"
check "terminal now" "True" "$(jq_get "$S6" terminal)"

echo
echo "CASE-ORDER-005  reconcile(declined) -> rejected"
R7=$(submit "$(s 8)" "$(k 8)" unknown "$BODY")
ORD8=$(jq_get "$(curl -s "$ORDER/v1/checkouts/$(jq_get "$R7" checkoutId)" -H "x-session-id: $(s 8)")" orderId)
R8=$(curl -s -X POST "$ORDER/v1/orders/$ORD8/reconcile" \
  -H "x-session-id: $(s 8)" -H "content-type: application/json" \
  -d '{"authorized":false}')
LAST_RESPONSE="$R8"
check "state" "rejected" "$(jq_get "$R8" state)"

echo
echo "SEC-07  reconcile on somebody else's order is refused"
CODE=$(curl -s -o "$TMPDIR_E2E/e2e_sec07.json" -w "%{http_code}" -X POST "$ORDER/v1/orders/$ORD5/reconcile" \
  -H "x-session-id: attacker-$RUN" -H "content-type: application/json" -d '{"authorized":true}')
LAST_RESPONSE="$(cat "$TMPDIR_E2E/e2e_sec07.json")"
check "http status" "403" "$CODE"

echo
echo "CASE-ORDER-006  reconcile on a non-unknown checkout is refused"
CID1STATUS=$(curl -s "$ORDER/v1/checkouts/$CID" -H "x-session-id: $(s 1)" | python -c "import sys,json;print(json.load(sys.stdin)['orderId'])") || true
CODE=$(curl -s -o "$TMPDIR_E2E/e2e_non_unknown.json" -w "%{http_code}" -X POST "$ORDER/v1/orders/$CID1STATUS/reconcile" \
  -H "x-session-id: $(s 1)" -H "content-type: application/json" -d '{"authorized":true}')
LAST_RESPONSE="$(cat "$TMPDIR_E2E/e2e_non_unknown.json")"
check "http status" "409" "$CODE"

# ------------------------------------------------------------------- IDOR
echo
echo "CASE-ORDER-014 / SEC-07  another session cannot read the order"
CODE=$(curl -s -o "$TMPDIR_E2E/e2e_idor.json" -w "%{http_code}" "$ORDER/v1/checkouts/$CID" -H "x-session-id: attacker-$RUN")
LAST_RESPONSE="$(cat "$TMPDIR_E2E/e2e_idor.json")"
check "http status" "403" "$CODE"

echo
echo "CASE-ORDER-014  order listing is scoped to the buyer"
N1_RESPONSE=$(curl -s "$ORDER/v1/orders" -H "x-session-id: $(s 1)")
N1=$(printf '%s' "$N1_RESPONSE" | python -c "import sys,json;print(len(json.load(sys.stdin)['items']))") || true
N2_RESPONSE=$(curl -s "$ORDER/v1/orders" -H "x-session-id: $(s 9)")
N2=$(printf '%s' "$N2_RESPONSE" | python -c "import sys,json;print(len(json.load(sys.stdin)['items']))") || true
# (s 1) made exactly one checkout in this run; the reconcile cases belong to
# different sessions, so the owner must still see exactly one.
LAST_RESPONSE="$N1_RESPONSE"
check "owner's order count" "1" "$N1"
LAST_RESPONSE="$N2_RESPONSE"
check "stranger's order count" "0" "$N2"

# --------------------------------------------------------- mixed currency
echo
echo "CASE-ORDER-011  mixed-currency order is rejected"
BODY_MIX='{"lines":[{"sku":"a","quantity":1,"unit_amount_minor":100,"currency":"JPY"},{"sku":"b","quantity":1,"unit_amount_minor":100,"currency":"USD"}],"payment_method_ref":"tok"}'
CODE=$(curl -s -o "$TMPDIR_E2E/e2e_mix.json" -w "%{http_code}" -X POST "$ORDER/v1/checkouts" \
  -H "x-session-id: $(s 5)" -H "idempotency-key: key-e2e-5" \
  -H "content-type: application/json" -d "$BODY_MIX")
LAST_RESPONSE="$(cat "$TMPDIR_E2E/e2e_mix.json")"
check "http status" "422" "$CODE"
check "error code" "order.mixed_currency" "$(jq_get "$(cat "$TMPDIR_E2E/e2e_mix.json")" code)"

# --------------------------------------------------------------- empty cart
echo
echo "CASE-ORDER-012  empty cart is rejected"
CODE=$(curl -s -o "$TMPDIR_E2E/e2e_empty_cart.json" -w "%{http_code}" -X POST "$ORDER/v1/checkouts" \
  -H "x-session-id: $(s 6)" -H "idempotency-key: key-e2e-6" \
  -H "content-type: application/json" -d '{"lines":[],"payment_method_ref":"tok"}')
LAST_RESPONSE="$(cat "$TMPDIR_E2E/e2e_empty_cart.json")"
check "http status" "422" "$CODE"

# ------------------------------------------------------- missing idem key
echo
echo "FR-SECURITY-03  write without Idempotency-Key is refused"
CODE=$(curl -s -o "$TMPDIR_E2E/e2e_missing_idem.json" -w "%{http_code}" -X POST "$ORDER/v1/checkouts" \
  -H "x-session-id: $(s 7)" -H "content-type: application/json" -d "$BODY")
LAST_RESPONSE="$(cat "$TMPDIR_E2E/e2e_missing_idem.json")"
check "http status" "400" "$CODE"

echo
echo "=== RESULT: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] || exit 1
