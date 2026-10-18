#!/usr/bin/env bash
# @cypher schema=1 source_sha256=06c407917798cfdb1e1cb35b7a3fddfb8301c4cfea3e2a7293af465b6ffdddf1
# MERGE (self:File {path:"services/deploy/deploy.sh"})
# MERGE (services:Config {name:"SERVICES"})
# MERGE (ports:Config {name:"PORT"})
# MERGE (upstream_env:Config {name:"UPSTREAM_ENV"})
# MERGE (bff_env:Config {name:"bff_env"})
# MERGE (admin_env:Config {name:"admin_env"})
# MERGE (bff:ExternalService {name:"commerce-bff"})
# MERGE (identity:ExternalService {name:"commerce-identity"})
# MERGE (catalog:ExternalService {name:"commerce-catalog"})
# MERGE (cart:ExternalService {name:"commerce-cart"})
# MERGE (inventory:ExternalService {name:"commerce-inventory"})
# MERGE (order:ExternalService {name:"commerce-order"})
# MERGE (payment:ExternalService {name:"commerce-payment-adapter"})
# MERGE (admin:ExternalService {name:"commerce-admin"})
# MERGE (self)-[:CONFIGURES]->(services)
# MERGE (self)-[:CONFIGURES]->(ports)
# MERGE (self)-[:CONFIGURES]->(upstream_env)
# MERGE (self)-[:CONFIGURES]->(bff_env)
# MERGE (self)-[:CONFIGURES]->(admin_env)
# MERGE (services)-[:CONFIGURES]->(bff)
# MERGE (services)-[:CONFIGURES]->(identity)
# MERGE (services)-[:CONFIGURES]->(catalog)
# MERGE (services)-[:CONFIGURES]->(cart)
# MERGE (services)-[:CONFIGURES]->(inventory)
# MERGE (services)-[:CONFIGURES]->(order)
# MERGE (services)-[:CONFIGURES]->(payment)
# MERGE (services)-[:CONFIGURES]->(admin)
# MERGE (ports)-[:CONFIGURES]->(bff)
# MERGE (ports)-[:CONFIGURES]->(identity)
# MERGE (ports)-[:CONFIGURES]->(catalog)
# MERGE (ports)-[:CONFIGURES]->(cart)
# MERGE (ports)-[:CONFIGURES]->(inventory)
# MERGE (ports)-[:CONFIGURES]->(order)
# MERGE (ports)-[:CONFIGURES]->(payment)
# MERGE (ports)-[:CONFIGURES]->(admin)
# MERGE (bff_env)-[:CONFIGURES]->(identity)
# MERGE (bff_env)-[:CONFIGURES]->(catalog)
# MERGE (bff_env)-[:CONFIGURES]->(cart)
# MERGE (bff_env)-[:CONFIGURES]->(inventory)
# MERGE (bff_env)-[:CONFIGURES]->(order)
# MERGE (bff_env)-[:CONFIGURES]->(payment)
# MERGE (admin_env)-[:DEPENDS_ON]->(upstream_env)
# MERGE (admin_env)-[:CONFIGURES]->(identity)
# MERGE (admin_env)-[:CONFIGURES]->(catalog)
# MERGE (admin_env)-[:CONFIGURES]->(inventory)
# MERGE (admin_env)-[:CONFIGURES]->(order)
# MERGE (admin_env)-[:CONFIGURES]->(payment)
# @endcypher
# deploy.sh — install / restart / uninstall all 8 commerce service binaries on this VM.
# Run as root.
set -euo pipefail

# Layout
APP_ROOT="/opt/commerce"
SRC_ROOT="/tmp/dokipon/services"   # the git clone
RELEASE_DIR="${SRC_ROOT}/target/release"
SERVICE_FILES_DIR="/etc/systemd/system"
SERVICE_USER="commerce"
LOG_DIR="/var/log/commerce"

# All 7 service binaries (BFF is included; mocks are excluded by design).
SERVICES=(
  "commerce-bff"
  "commerce-identity"
  "commerce-catalog"
  "commerce-cart"
  "commerce-inventory"
  "commerce-order"
  "commerce-payment-adapter"
  "commerce-admin"
)

# Per-service ports; admin stays outside the customer OpenAPI contract.
declare -A PORT=(
  ["commerce-bff"]=8080
  ["commerce-identity"]=8081
  ["commerce-catalog"]=8082
  ["commerce-cart"]=8083
  ["commerce-inventory"]=8084
  ["commerce-order"]=8085
  ["commerce-payment-adapter"]=8086
  ["commerce-admin"]=8087
)

# Upstream service URLs used by the BFF and admin systemd units.
declare -A UPSTREAM_ENV=(
  ["commerce-identity"]="IDENTITY_URL"
  ["commerce-catalog"]="CATALOG_URL"
  ["commerce-cart"]="CART_URL"
  ["commerce-inventory"]="INVENTORY_URL"
  ["commerce-order"]="ORDER_URL"
  ["commerce-payment-adapter"]="PAYMENT_ADAPTER_URL"
)

action="${1:-install}"

if [[ "$action" == "install" || "$action" == "reinstall" ]]; then
  echo "=== install: ensure commerce user exists ==="
  if ! id "$SERVICE_USER" >/dev/null 2>&1; then
    useradd --system --home "$APP_ROOT" --shell /usr/sbin/nologin "$SERVICE_USER"
  fi
  mkdir -p "$APP_ROOT/bin" "$LOG_DIR"
  chown -R "$SERVICE_USER:$SERVICE_USER" "$APP_ROOT" "$LOG_DIR"

  echo "=== install: copy release binaries ==="
  for svc in "${SERVICES[@]}"; do
    src="${RELEASE_DIR}/${svc}"
    if [[ ! -x "$src" ]]; then
      echo "  ✗ missing binary: $src" >&2
      echo "    build first: cd $SRC_ROOT && cargo build --release" >&2
      exit 1
    fi
    install -m 0755 "$src" "${APP_ROOT}/bin/${svc}"
  done

  echo "=== install: write systemd unit files ==="
  # Build BFF environment (point at every upstream)
  bff_env=""
  for svc in commerce-identity commerce-catalog commerce-cart commerce-inventory commerce-order commerce-payment-adapter; do
    var="${UPSTREAM_ENV[$svc]}"
    bff_env+="${var}=http://127.0.0.1:${PORT[$svc]} "
  done
  admin_env=""
  for svc in commerce-identity commerce-catalog commerce-inventory commerce-order commerce-payment-adapter; do
    var="${UPSTREAM_ENV[$svc]}"
    admin_env+="${var}=http://127.0.0.1:${PORT[$svc]} "
  done

  for svc in "${SERVICES[@]}"; do
    port="${PORT[$svc]}"
    unit="/etc/systemd/system/${svc}.service"
    cat > "$unit" <<EOF
[Unit]
Description=Commerce service: ${svc}
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=${SERVICE_USER}
Group=${SERVICE_USER}
WorkingDirectory=${APP_ROOT}
ExecStart=${APP_ROOT}/bin/${svc}
Environment=RUST_LOG=info
Environment=PORT=${port}
EOF
    if [[ "$svc" == "commerce-bff" ]]; then
      cat >> "$unit" <<EOF
Environment=${bff_env}
EOF
    elif [[ "$svc" == "commerce-admin" ]]; then
      cat >> "$unit" <<EOF
Environment=${admin_env}
EOF
    fi
    cat >> "$unit" <<EOF
Restart=on-failure
RestartSec=3
StandardOutput=append:${LOG_DIR}/${svc}.log
StandardError=append:${LOG_DIR}/${svc}.log
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=${LOG_DIR}

[Install]
WantedBy=multi-user.target
EOF
  done
  systemctl daemon-reload
  echo "=== install: enable + start ==="
  for svc in "${SERVICES[@]}"; do
    systemctl enable --now "${svc}.service"
  done
  echo "  ✓ all 8 services installed and started"
  echo "  log dir: $LOG_DIR"
  echo "  check: systemctl status commerce-bff.service"

elif [[ "$action" == "uninstall" ]]; then
  echo "=== uninstall: stop + remove ==="
  for svc in "${SERVICES[@]}"; do
    systemctl disable --now "${svc}.service" 2>/dev/null || true
    rm -f "/etc/systemd/system/${svc}.service"
  done
  systemctl daemon-reload
  rm -rf "$APP_ROOT"
  echo "  ✓ uninstalled"

elif [[ "$action" == "restart" ]]; then
  echo "=== restart ==="
  for svc in "${SERVICES[@]}"; do
    systemctl restart "${svc}.service"
  done
  for svc in "${SERVICES[@]}"; do
    systemctl --no-pager --full status "${svc}.service" | head -3 | sed "s/^/  /"
  done

elif [[ "$action" == "status" ]]; then
  echo "=== status ==="
  for svc in "${SERVICES[@]}"; do
    if systemctl is-active --quiet "${svc}.service"; then
      printf "  %-30s ACTIVE\n" "$svc"
    else
      printf "  %-30s INACTIVE\n" "$svc"
    fi
  done

elif [[ "$action" == "logs" ]]; then
  svc="${2:-commerce-bff}"
  tail -n 200 "${LOG_DIR}/${svc}.log" 2>&1 || true

else
  echo "usage: $0 [install|uninstall|restart|status|logs <svc>]" >&2
  exit 2
fi
