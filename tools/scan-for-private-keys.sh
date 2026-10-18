#!/usr/bin/env bash
# scan-for-private-keys.sh — fail if any tracked file carries private key
# material, including the base64 form that a plain `grep "PRIVATE KEY"` misses.
#
# Why this exists: docs/deploy/01-k8s-manifests/50-secret-*-tls.yaml carried six
# real leaf private keys as base64 in `data.tls.key`. Grepping the repo for the
# literal string "PRIVATE KEY" returns nothing, because the YAML holds base64.
# That invisibility is why the keys survived review and reached the default
# branch.
#
# A key is trusted as private key material only when it is NOT gitignored: a PEM
# under certs/ that is correctly ignored is a local developer artefact, not a
# leak. Anything git tracks with key material inside is a finding.
#
# Usage:
#   bash tools/scan-for-private-keys.sh            # scan the working tree
#   bash tools/scan-for-private-keys.sh --staged   # only what is about to commit
#
# Exit codes: 0 clean, 1 key material found in a tracked file.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 2

MODE="tree"
[[ "${1:-}" == "--staged" ]] && MODE="staged"

FINDINGS=0
SCANNED=0

# Files to consider. Tracked files only in tree mode; the index in staged mode.
collect() {
  if [[ "$MODE" == "staged" ]]; then
    git diff --cached --name-only --diff-filter=ACMR
  else
    git ls-files
  fi
}

# Decode a base64 blob and report the PEM banner it contains.
pem_banner() {
  local blob="$1"
  printf '%s' "$blob" \
    | tr -d '\r' \
    | base64 -d 2>/dev/null \
    | grep -oE -- '-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----' \
    | head -1
}

report() {
  local file="$1" where="$2" banner="$3"
  FINDINGS=$((FINDINGS + 1))
  printf 'LEAK  %s  [%s]  %s\n' "$file" "$where" "$banner" >&2
}

# A hit only counts when it is a real PEM block, not a source-code literal.
# Test assertions like `key.contains("-----BEGIN PRIVATE KEY-----")` and
# doc comments that quote the header are not credentials; requiring the armored
# payload to follow is what separates the two.
#   $1 = file, $2 = raw text (plaintext mode), $3 = base64 blob (base64 mode)
is_real_pem() {
  local file="$1" mode="$2" blob="${3:-}" text="" banner

  if [[ "$mode" == "plaintext" ]]; then
    text="$(cat "$file" 2>/dev/null)"
  else
    text="$(printf '%s' "$blob" | tr -d '\r' | base64 -d 2>/dev/null)"
  fi

  banner="$(printf '%s' "$text" \
    | grep -aoE -- '-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----' | head -1)"
  [[ -z "$banner" ]] && return 1

  # Base64 payload between the BEGIN and END armor, long enough to be a key.
  if printf '%s' "$text" \
      | tr -d '\n\r' \
      | grep -qE -- "$(printf '%s' "$banner" | sed 's/[][\.*^$()+?{}|]/\\&/g')[A-Za-z0-9+/=]{80,}"; then
    return 0
  fi
  return 1
}

while IFS= read -r file; do
  [[ -z "$file" ]] && continue
  [[ -f "$file" ]] || continue

  case "$file" in
    *.png|*.jpg|*.jpeg|*.gif|*.ico|*.pdf|*.zip|*.gz|*.tgz|*.jar|*.wasm|*.so|*.dll|*.exe) continue ;;
  esac

  SCANNED=$((SCANNED + 1))

  # 1. Plaintext PEM.
  if is_real_pem "$file" plaintext; then
    banner="$(grep -aoE -- '-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----' "$file" 2>/dev/null | head -1)"
    report "$file" "plaintext" "$banner"
    continue
  fi

  # 2. base64-wrapped PEM. Pull every base64-looking blob long enough to hold a
  #    PEM header and decode it.
  while IFS= read -r blob; do
    [[ ${#blob} -lt 120 ]] && continue
    if is_real_pem "$file" base64 "$blob"; then
      report "$file" "base64" "$(pem_banner "$blob")"
      break
    fi
  done < <(grep -aoE '[A-Za-z0-9+/]{120,}={0,2}' "$file" 2>/dev/null | sort -u)
done < <(collect)

echo
if [[ "$FINDINGS" -gt 0 ]]; then
  echo "x $FINDINGS file(s) carry private key material." >&2
  echo >&2
  echo "  Certificates belong to cert-manager or an external secret store." >&2
  echo "  A tracked Secret with a real key in data.tls.key is a credential leak," >&2
  echo "  and it must be rotated, not just deleted — see" >&2
  echo "  docs/deploy/03-platform/certmanager/SECURITY-NOTES.md" >&2
  exit 1
fi

echo "ok  no private key material in tracked files ($SCANNED scanned, $MODE mode)"
exit 0
