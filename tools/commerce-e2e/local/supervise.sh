#!/usr/bin/env bash
# Start every process of the local deployment, inside one container.
#
# ## Why this script exists
#
# The Vercel deployment runs each commerce service as a **Function**: its own
# process, its own in-memory state, reached over HTTP by the gateway. The
# local deployment has to be the same in that respect, or a test run locally
# would be testing a topology the deployment does not have — the in-process
# edge answers the same contract, but it keeps every service's state in one
# process, and a cart created in one Function is invisible to another.
#
# So this script starts the ten processes the deployment's ten Functions
# stand for:
#
#   the eight commerce services, the gateway, and the single-process
#   fallback that `/api/index` is rewritten to.
#
# The addresses are the ones the gateway and the admin facade read from the
# environment (`CATALOG_URL` and friends), so the gateway forwards to the
# services exactly as the deployed gateway forwards to its Functions.

set -u

BASE_PORT="${LOCAL_BASE_PORT:?LOCAL_BASE_PORT must be set}"
BIN_DIR="${LOCAL_BIN_DIR:-/work/services/target/release}"

CATALOG_PORT=$((BASE_PORT + 1))
CART_PORT=$((BASE_PORT + 2))
ORDER_PORT=$((BASE_PORT + 3))
CONTENT_PORT=$((BASE_PORT + 4))
IDENTITY_PORT=$((BASE_PORT + 5))
INVENTORY_PORT=$((BASE_PORT + 6))
PAYMENT_PORT=$((BASE_PORT + 7))
ADMIN_PORT=$((BASE_PORT + 8))
GATEWAY_PORT=$((BASE_PORT + 9))
EDGE_PORT=$((BASE_PORT + 10))
MOCK_LINE_IDENTITY_PORT=$((BASE_PORT + 12))

export CATALOG_URL="http://127.0.0.1:${CATALOG_PORT}"
export CART_URL="http://127.0.0.1:${CART_PORT}"
export ORDER_URL="http://127.0.0.1:${ORDER_PORT}"
export CONTENT_URL="http://127.0.0.1:${CONTENT_PORT}"
export IDENTITY_URL="http://127.0.0.1:${IDENTITY_PORT}"
export INVENTORY_URL="http://127.0.0.1:${INVENTORY_PORT}"
export PAYMENT_ADAPTER_URL="http://127.0.0.1:${PAYMENT_PORT}"
export ADMIN_URL="http://127.0.0.1:${ADMIN_PORT}"

pids=()

start() {
  local name="$1" port="$2" binary="$3"
  if [ ! -x "$BIN_DIR/$binary" ]; then
    echo "==> $name: $BIN_DIR/$binary is missing; the build did not produce it" >&2
    exit 1
  fi
  PORT="$port" "$BIN_DIR/$binary" &
  pids+=("$!")
  echo "[$name] pid $! port $port"
}

# The eight services: one process each, one in-memory state each.
start catalog "$CATALOG_PORT" commerce-catalog
start cart "$CART_PORT" commerce-cart
start order "$ORDER_PORT" commerce-order
start content "$CONTENT_PORT" commerce-content
start identity "$IDENTITY_PORT" commerce-identity
start inventory "$INVENTORY_PORT" commerce-inventory
start payment "$PAYMENT_PORT" commerce-payment-adapter
start admin "$ADMIN_PORT" commerce-admin

# The gateway: the same router the Vercel gateway Function serves.
start gateway "$GATEWAY_PORT" local-gateway

# The single-process fallback: the whole contract in one process, which is
# what `/api/index` is rewritten to.
start edge "$EDGE_PORT" commerce-edge

# The LINE id-token issuer.
#
# Not a dependency of the deployment: the product verifies tokens itself and
# never calls an issuer. It is here so a **test** can mint a token the
# deployment's identity service accepts — signed with the same key and for
# the same channel, because both come from the fixture set's environment.
start mock-line-identity "$MOCK_LINE_IDENTITY_PORT" mock-line-identity

echo "==> the local stack is up"

terminate() {
  echo "==> stopping the local stack"
  kill "${pids[@]}" 2>/dev/null
  wait
  exit 0
}
trap terminate TERM INT

wait
