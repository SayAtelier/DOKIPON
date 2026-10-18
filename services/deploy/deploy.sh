#!/usr/bin/env bash
# deploy.sh — install / restart / uninstall all 7 commerce services on this VM.
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
)

# Per-service port assignments (matches contracts/openapi.yaml)
declare -A PORT=(
  ["commerce-bff"]=8080
  ["commerce-identity"]=8081
  ["commerce-catalog"]=8082
  ["commerce-cart"]=8083
  ["commerce-inventory"]=8084
  ["commerce-order"]=8085
  ["commerce-payment-adapter"]=8086
)

# Upstream service URLs (used by commerce-bff via env). One per non-bff service.
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
  echo "  ✓ all 7 services installed and started"
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
