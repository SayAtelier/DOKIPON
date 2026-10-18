#!/usr/bin/env bash
# @cypher schema=1 source_sha256=5d0cce8113962b1641e90872ce06eae114f3e68b0ce282080259016a30588e56
# MERGE (self:File {path:"tools/e2e_admin_smoke.sh"})
# MERGE (main:Symbol {id:"tools/e2e_admin_smoke.sh::main",kind:"script"})
# MERGE (request:Symbol {id:"tools/e2e_admin_smoke.sh::request",kind:"function"})
# MERGE (assert_status:Symbol {id:"tools/e2e_admin_smoke.sh::assert_status",kind:"function"})
# MERGE (json_field:Symbol {id:"tools/e2e_admin_smoke.sh::json_field",kind:"function"})
# MERGE (first_non_unknown_order:Symbol {id:"tools/e2e_admin_smoke.sh::first_non_unknown_order",kind:"function"})
# MERGE (encode_path_segment:Symbol {id:"tools/e2e_admin_smoke.sh::encode_path_segment",kind:"function"})
# MERGE (fail:Symbol {id:"tools/e2e_admin_smoke.sh::fail",kind:"function"})
# MERGE (admin_url:Config {name:"ADMIN_URL"})
# MERGE (admin_user:Config {name:"ADMIN_USER"})
# MERGE (admin_password:Config {name:"ADMIN_PASSWORD"})
# MERGE (http_code:Symbol {id:"tools/e2e_admin_smoke.sh::HTTP_CODE",kind:"variable"})
# MERGE (http_body:Symbol {id:"tools/e2e_admin_smoke.sh::HTTP_BODY",kind:"variable"})
# MERGE (pass_count:Symbol {id:"tools/e2e_admin_smoke.sh::PASS",kind:"variable"})
# MERGE (self)-[:DEFINES]->(main)
# MERGE (self)-[:DEFINES]->(request)
# MERGE (self)-[:DEFINES]->(assert_status)
# MERGE (self)-[:DEFINES]->(json_field)
# MERGE (self)-[:DEFINES]->(first_non_unknown_order)
# MERGE (self)-[:DEFINES]->(encode_path_segment)
# MERGE (self)-[:DEFINES]->(fail)
# MERGE (self)-[:DEFINES]->(http_code)
# MERGE (self)-[:DEFINES]->(http_body)
# MERGE (self)-[:DEFINES]->(pass_count)
# MERGE (request)-[:READS]->(admin_url)
# MERGE (request)-[:WRITES]->(http_code)
# MERGE (request)-[:WRITES]->(http_body)
# MERGE (main)-[:READS]->(admin_user)
# MERGE (main)-[:READS]->(admin_password)
# MERGE (main)-[:CALLS]->(request)
# MERGE (main)-[:CALLS]->(assert_status)
# MERGE (main)-[:CALLS]->(json_field)
# MERGE (main)-[:CALLS]->(first_non_unknown_order)
# MERGE (main)-[:CALLS]->(encode_path_segment)
# MERGE (main)-[:CALLS]->(fail)
# MERGE (assert_status)-[:CALLS]->(fail)
# MERGE (assert_status)-[:WRITES]->(pass_count)
# MERGE (self)-[:DEPENDS_ON]->(python:ExternalService {name:"python"})
# MERGE (self)-[:DEPENDS_ON]->(curl_cli:ExternalService {name:"curl"})
# MERGE (self)-[:DEPENDS_ON]->(sed:ExternalService {name:"sed"})
# MERGE (request)-[:DEPENDS_ON]->(curl_cli)
# MERGE (self)-[:CONFIGURES]->(admin:ExternalService {name:"commerce-admin"})
# @endcypher
# Admin smoke test against the commerce-admin HTTP API.
# Usage with the NodePort: ADMIN_USER=operator ADMIN_PASSWORD='<configured password>' bash tools/e2e_admin_smoke.sh
# Port-forward alternative: kubectl -n commerce port-forward svc/commerce-admin 18087:8087
#   ADMIN_USER=operator ADMIN_PASSWORD='<configured password>' ADMIN_URL=http://127.0.0.1:18087 bash tools/e2e_admin_smoke.sh
set -euo pipefail

ADMIN_URL="${ADMIN_URL:-http://localhost:30087}"
ADMIN_URL="${ADMIN_URL%/}"
ADMIN_USER="${ADMIN_USER:-operator}"
: "${ADMIN_PASSWORD:?Set ADMIN_PASSWORD to the operator credential configured in commerce-admin-credentials}"

TMPDIR_E2E="$(mktemp -d "${TMPDIR:-/tmp}/gemarc-admin-smoke.XXXXXX")"
trap 'rm -rf "$TMPDIR_E2E"' EXIT

HTTP_CODE=""
HTTP_BODY=""
PASS=0

fail() {
  printf 'FAIL  %s (HTTP %s)\n' "$1" "${HTTP_CODE:-none}" >&2
  if [[ -s "$TMPDIR_E2E/curl.stderr" ]]; then
    sed 's/^/      /' "$TMPDIR_E2E/curl.stderr" >&2
  fi
  if [[ -n "${HTTP_BODY:-}" ]]; then
    printf '%s' "$HTTP_BODY" | python -c 'import json,sys
try:
    body=json.load(sys.stdin)
except Exception:
    print("<non-JSON response omitted>")
    raise SystemExit(0)
if isinstance(body, dict):
    body.pop("token", None)
    body.pop("password", None)
print(json.dumps(body, ensure_ascii=False))' >&2 || true
  fi
  exit 1
}

request() {
  local method=$1
  local path=$2
  local payload_file=${3:-}
  local token=${4:-}
  local response_file="$TMPDIR_E2E/response.json"
  local auth_header="$TMPDIR_E2E/auth.header"
  local -a args=(-sS -o "$response_file" -w '%{http_code}' -X "$method" "${ADMIN_URL}${path}")

  : > "$response_file"
  if [[ -n "$token" ]]; then
    # Write the real header value. An earlier version wrote the literal string
    # "Bearer ***", so every authenticated call carried a placeholder and the
    # upstream answered 401 — the smoke test could never have passed.
    printf 'Authorization: Bearer %s' "$token" > "$auth_header"
    chmod 600 "$auth_header"
    args+=(-H "@${auth_header}")
  fi
  if [[ -n "$payload_file" ]]; then
    args+=(-H 'content-type: application/json' --data-binary "@${payload_file}")
  fi

  HTTP_CODE=$(curl "${args[@]}" 2>"$TMPDIR_E2E/curl.stderr") || HTTP_CODE="000"
  HTTP_BODY=$(<"$response_file")
}

assert_status() {
  local expected=$1
  local label=$2
  if [[ "$HTTP_CODE" != "$expected" ]]; then
    fail "$label expected HTTP $expected"
  fi
  PASS=$((PASS + 1))
  printf 'PASS  %s\n' "$label"
}

json_field() {
  python -c 'import json,sys; print(json.load(sys.stdin).get(sys.argv[1], ""))' "$1" <<< "$2" 2>/dev/null || true
}

first_non_unknown_order() {
  python -c 'import json,sys
items=json.load(sys.stdin).get("items", [])
print(next((item.get("orderId", "") for item in items if item.get("state") != "payment_unknown"), ""))' <<< "$1" 2>/dev/null || true
}

encode_path_segment() {
  python -c 'from urllib.parse import quote; import sys; print(quote(sys.argv[1], safe=""))' "$1"
}

export ADMIN_USER ADMIN_PASSWORD
python -c 'import json,os; print(json.dumps({"user":os.environ["ADMIN_USER"],"password":os.environ["ADMIN_PASSWORD"]}))' > "$TMPDIR_E2E/login.json"

printf '=== E2E: commerce-admin smoke ===\n'
request POST '/admin/v1/sessions' "$TMPDIR_E2E/login.json"
assert_status 200 'operator login'
TOKEN=$(json_field token "$HTTP_BODY")
ROLE=$(json_field role "$HTTP_BODY")
if [[ -z "$TOKEN" ]]; then
  fail 'login response did not contain a token'
fi
if [[ "$ROLE" != 'operator' ]]; then
  fail "login role must be operator, got ${ROLE:-empty}"
fi
PASS=$((PASS + 1))
printf 'PASS  login role is operator\n'

request GET '/admin/v1/orders?limit=100' '' "$TOKEN"
assert_status 200 'list orders'
ORDER_ID=$(first_non_unknown_order "$HTTP_BODY")
if [[ -z "$ORDER_ID" ]]; then
  fail 'order list contains no non-payment_unknown order'
fi
ORDER_ID=$(encode_path_segment "$ORDER_ID")

request GET "/admin/v1/orders/${ORDER_ID}" '' "$TOKEN"
assert_status 200 'fetch order detail'
ORDER_STATE=$(json_field state "$HTTP_BODY")
if [[ "$ORDER_STATE" == 'payment_unknown' || -z "$ORDER_STATE" ]]; then
  fail "selected order is not a usable non-payment_unknown order (state=${ORDER_STATE:-empty})"
fi
PASS=$((PASS + 1))
printf 'PASS  selected order is non-payment_unknown\n'

printf '%s\n' '{"authorized":true}' > "$TMPDIR_E2E/reconcile.json"
request POST "/admin/v1/orders/${ORDER_ID}/reconcile" "$TMPDIR_E2E/reconcile.json" "$TOKEN"
assert_status 409 'reconcile of non-payment_unknown order is refused'

printf '=== RESULT: %s passed, 0 failed ===\n' "$PASS"
