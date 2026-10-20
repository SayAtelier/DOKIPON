#!/usr/bin/env bash
# build-web-images.sh — build the Flutter Web bundles and import them into k3s.
#
# Two apps, one recipe:
#   admin_web  -> dokipon/admin-web:0.1.0   --base-href /admin/
#   shop_web   -> dokipon/shop-web:0.1.0    --base-href /
#
# They differ only in base href, which is why the nginx config is identical: with
# /admin/ the bundle asks for assets under that prefix and nginx's SPA fallback
# still serves them from the document root, and with / it asks from the root.
#
# Why the image is built here rather than pulled: the payload is ~40 MB of
# canvaskit per app and there is no registry to push to. Nginx serves it as
# static files, which is all a built Flutter SPA needs.
#
# Usage:
#   bash build-web-images.sh            # both
#   bash build-web-images.sh shop_web   # one
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-/home/ubuntu/dokipon}"
FLUTTER="${FLUTTER:-/mnt/c/tools/flutter/bin/flutter}"
APPS_DIR="${REPO_ROOT}/apps"
WORK="${WORK:-/var/tmp/web-images}"

if [ "$#" -gt 0 ]; then
  APPS=("$@")
else
  APPS=(admin_web shop_web)
fi

# The base href each app is built with. Anything else needs its own nginx config.
# Both apps are built with --base-href /.
#
# admin_web used to be built with /admin/, which made the bundle request
# /admin/main.dart.js. The gateway serves the admin-web container's files from
# the root, so the browser got a 404 for the bundle, the Flutter engine never
# mounted, and the page stayed blank — with no error in the console, because a
# 404 on a script tag is only visible in the network tab.
#
# Envoy separates the two apps by Host header (shop.local / admin.local), not by
# path prefix, so both can sit at the root of their own hostname.
base_href_for() {
  case "$1" in
    admin_web) echo "/" ;;
    shop_web)  echo "/" ;;
    *) echo "unknown app: $1" >&2; exit 2 ;;
  esac
}

# GEMARC_API_BASE=/ compiles a same-origin API base into the bundle, so the app
# calls /v1/... on whatever host served it. That is what lets Envoy route by Host
# (shop.local / admin.local) with no client-side configuration, and it is why the
# bundle must be rebuilt whenever the gateway hostname changes.
API_BASE="${GEMARC_API_BASE:-/}"
# The image tag is pinned here and mirrored in services/deploy/k3s/10-services.yaml.
# Keep them in step: imagePullPolicy: Never plus an unchanged tag means a
# rebuilt bundle is never actually picked up.
TAG="${TAG:-0.1.1}"
CHANNEL="${GEMARC_CHANNEL:-web}"

mkdir -p "$WORK"

for app in "${APPS[@]}"; do
  src="${APPS_DIR}/${app}"
  [ -d "$src" ] || { echo "x no such app: ${src}" >&2; exit 2; }

  href="$(base_href_for "$app")"
  name="${app/_/-}"
  image="dokipon/${name}:${TAG}"
  stage="${WORK}/${app}"

  echo "=== ${app} (--base-href ${href}, GEMARC_API_BASE=${API_BASE}) ==="

  # --no-web-resources-cdn makes the build use the canvaskit/ directory that is
  # shipped in the image instead of fetching it from gstatic.com. Without it the
  # bundle sets useLocalCanvasKit=false and the browser tries to download
  # canvaskit.js from the CDN, which fails on a host with no access to gstatic
  # and leaves the page blank. The canvaskit payload is already in the image
  # (40 MB), so serving it locally costs nothing extra.
  if [ -x "$FLUTTER" ] || command -v "$FLUTTER" >/dev/null 2>&1; then
    ( cd "$src" && "$FLUTTER" build web --release --base-href "$href" \
        --no-web-resources-cdn \
        --dart-define=GEMARC_CHANNEL="$CHANNEL" \
        --dart-define=GEMARC_API_BASE="$API_BASE" )
  else
    echo "  flutter not found at ${FLUTTER}; using the existing build/web"
    [ -d "${src}/build/web" ] || { echo "x ${src}/build/web missing" >&2; exit 3; }
  fi

  rm -rf "$stage"
  mkdir -p "$stage"
  cp -r "${src}/build/web" "${stage}/web"

  # SPA fallback is required: a Flutter app is a single-page app, so any deep
  # link must return index.html rather than 404.
  cat > "${stage}/Dockerfile" <<'DOCKEREOF'
FROM nginx:alpine
COPY --chown=nginx:nginx web/ /usr/share/nginx/html/
RUN printf '%s\n' \
  'server {' \
  '  listen 8080;' \
  '  server_name _;' \
  '  root /usr/share/nginx/html;' \
  '  index index.html;' \
  '  absolute_redirect off;' \
  '  client_max_body_size 4m;' \
  '  location / {' \
  '    try_files $uri $uri/ /index.html;' \
  '  }' \
  '  location = /index.html {' \
  '    add_header Cache-Control "no-store" always;' \
  '  }' \
  '  location ~* \.(js|css|wasm|json|png|jpg|svg|woff2?)$ {' \
  '    expires 1h;' \
  '  }' \
  '}' \
  > /etc/nginx/conf.d/default.conf
RUN touch /var/run/nginx.pid \
 && chown -R nginx:nginx /var/run/nginx.pid /var/cache/nginx /etc/nginx/conf.d \
 && sed -i 's#^pid .*#pid /var/run/nginx.pid;#' /etc/nginx/nginx.conf
USER nginx
EXPOSE 8080
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s \
  CMD wget -q -O /dev/null http://127.0.0.1:8080/ || exit 1
CMD ["nginx", "-g", "daemon off;"]
DOCKEREOF

  docker build -t "$image" "$stage"
  docker save "$image" | sudo /usr/local/bin/k3s ctr -n k8s.io images import -
  echo "  ok ${image}"
  echo "    (manifest must reference the same tag: services/deploy/k3s/10-services.yaml)"
done

echo
echo "=== web images in k3s containerd ==="
sudo /usr/local/bin/k3s ctr -n k8s.io images ls -q | grep -E 'dokipon/(admin-web|shop-web)' || echo "(none)"
