#!/usr/bin/env bash
# build-vercel.sh — assemble the Vercel deployment.
#
# The deployment has three parts:
#
#   1. `public/`          the static Flutter Web bundles
#   2. `api/index.rs`     the Rust function (already in the repo)
#   3. `vercel.json`      the routing and runtime configuration
#
# The static part needs a build step, because `flutter build web` writes into
# each app's own `build/web` directory and Vercel serves one `public/`
# directory. This script builds both apps and merges them:
#
#   public/                the storefront (the root)
#   public/admin/          the admin console
#
# The merge is a copy, not a symlink: Vercel uploads the directory as it is
# on disk, and a symlink to a path outside the output directory would either
# be broken or pull in the whole repository.
#
# The API base is baked into the Dart build, and it is a **relative** path
# (`/`), so one build works on any domain — there is nothing to substitute
# per environment.

set -euo pipefail

# The repository root. This script lives in `services/deploy/vercel/`, so the
# root is three levels up: `vercel` → `deploy` → `services` → the root. One
# level too few lands on `services/`, where there is no `apps/` and no
# `public/`, and the build stops on the first `cd`.
REPO="${REPO:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)}"
PUBLIC="$REPO/public"

# The Flutter SDK. On a developer machine it is already installed; on the
# Vercel builder it is not, and this script installs it. `FLUTTER_ROOT` is
# honoured so a cached SDK is reused rather than downloaded again.
FLUTTER_VERSION="${FLUTTER_VERSION:-3.47.7}"
# The sha256 the Flutter release index publishes for the Linux archive. It is
# checked whenever the SDK has to be downloaded.
FLUTTER_SHA256="${FLUTTER_SHA256:-af7455f540df951184c7953fbe3cf884766e7541dacf8571f9f582db03dd6230}"
FLUTTER_ROOT="${FLUTTER_ROOT:-$REPO/.flutter-sdk}"

find_flutter() {
  if command -v flutter >/dev/null 2>&1; then
    command -v flutter
    return 0
  fi
  if [ -x "$FLUTTER_ROOT/bin/flutter" ]; then
    echo "$FLUTTER_ROOT/bin/flutter"
    return 0
  fi
  return 1
}

if ! FLUTTER="$(find_flutter)"; then
  echo "==> installing the Flutter SDK $FLUTTER_VERSION"
  ARCHIVE="flutter_linux_${FLUTTER_VERSION}-stable.tar.xz"
  URL="https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/$ARCHIVE"
  echo "    $URL"
  curl -fsSL "$URL" -o "/tmp/$ARCHIVE"

  # The archive is verified against the checksum the Flutter release index
  # publishes. Without this, a build step that downloads and executes a
  # binary from the internet is a supply-chain hole; with it, a tampered or
  # truncated download fails the build instead of running.
  if command -v sha256sum >/dev/null 2>&1 && [ -n "${FLUTTER_SHA256:-}" ]; then
    echo "    verifying sha256"
    echo "$FLUTTER_SHA256  /tmp/$ARCHIVE" | sha256sum -c - || {
      echo "==> the Flutter SDK download failed its checksum; refusing to build"
      exit 1
    }
  fi

  rm -rf "$FLUTTER_ROOT"
  mkdir -p "$FLUTTER_ROOT"
  tar -xJf "/tmp/$ARCHIVE" -C "$FLUTTER_ROOT" --strip-components=1
  rm -f "/tmp/$ARCHIVE"
  FLUTTER="$FLUTTER_ROOT/bin/flutter"
fi

# The builder runs as root, and the freshly extracted SDK is owned by root, so
# git refuses to read it ("detected dubious ownership in repository"). The
# Flutter tool asks git for the SDK's own version, so that refusal surfaces as
# a hard failure: `flutter pub get` dies with exit code 128 and the build
# stops with a bare "exited with 128" and no useful message.
#
# The fix has to be in place *before* the first flutter command, which is why
# it is here rather than next to the pub get step. Two mechanisms are used:
# the environment form needs no writable HOME and therefore cannot fail, and
# the global-config form covers git versions that predate the environment
# form. On a developer machine this is a no-op — the SDK is already owned by
# the user running it.
SDK_DIR="$(cd "$(dirname "$FLUTTER")/.." && pwd)"
export GIT_CONFIG_COUNT="${GIT_CONFIG_COUNT:-1}"
export GIT_CONFIG_KEY_0="${GIT_CONFIG_KEY_0:-safe.directory}"
export GIT_CONFIG_VALUE_0="${GIT_CONFIG_VALUE_0:-*}"
git config --global --add safe.directory "$SDK_DIR" 2>/dev/null || true
git config --global --add safe.directory "*" 2>/dev/null || true

echo "==> flutter: $FLUTTER"
"$FLUTTER" --version 2>&1 | head -2 || true

# The Dart packages have to be resolved before the build. The workspace uses
# path dependencies, so this is a local resolve, not a download of the world.
echo "==> resolving the Dart packages"
for app in shop_web admin_web; do
  ( cd "$REPO/apps/$app" && "$FLUTTER" pub get ) 2>&1 | tail -1
done

echo "==> building the storefront"
( cd "$REPO/apps/shop_web" && "$FLUTTER" build web --release \
    --base-href / \
    --no-web-resources-cdn \
    --dart-define=GEMARC_CHANNEL=web \
    --dart-define=GEMARC_API_BASE=/ ) 2>&1 | tail -3

echo "==> building the admin console"
( cd "$REPO/apps/admin_web" && "$FLUTTER" build web --release \
    --base-href /admin/ \
    --no-web-resources-cdn \
    --dart-define=GEMARC_CHANNEL=web \
    --dart-define=GEMARC_API_BASE=/ ) 2>&1 | tail -3

echo "==> assembling public/"
rm -rf "$PUBLIC"
mkdir -p "$PUBLIC/admin"

# The storefront is the site root.
cp -R "$REPO/apps/shop_web/build/web/." "$PUBLIC/"

# The admin console lives under /admin/. Its assets are referenced relative
# to that path, so the whole bundle moves together.
cp -R "$REPO/apps/admin_web/build/web/." "$PUBLIC/admin/"

# The admin console's Flutter service worker would otherwise claim the whole
# origin and serve the storefront's routes from the admin bundle. It is
# dropped: the admin console is a rarely-visited surface and a service worker
# that hijacks the origin is worse than one fewer cache.
rm -f "$PUBLIC/admin/flutter_service_worker.js"

# The `.symbols` files are DWARF debug tables for the WASM modules. They are
# roughly half the size of the canvaskit directory and are only read by a
# debugger. A browser never fetches them.
find "$PUBLIC" -name "*.symbols" -delete

# The WASM "heavy" variants are the CPU-fallback renderers. They are only
# loaded by a browser without WebGPU or without the primary skwasm path, and
# they are large. The skwasm build works everywhere that matters, so the
# heavy fallbacks are dropped.
rm -f "$PUBLIC"/canvaskit/skwasm_heavy.* "$PUBLIC"/admin/canvaskit/skwasm_heavy.*

# The root service worker is kept but told not to cache the API, so a
# deployment's new function is picked up rather than served from a stale
# cache.
if [ -f "$PUBLIC/flutter_service_worker.js" ]; then
  echo "==> keeping the storefront service worker (API paths are not cached)"
fi

echo "==> public/ is ready:"
du -sh "$PUBLIC" 2>/dev/null || true
ls "$PUBLIC" | head -20
echo "    admin/:"
ls "$PUBLIC/admin" | head -20
