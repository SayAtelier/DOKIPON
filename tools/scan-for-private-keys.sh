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

# Report a finding. Called for each hit.
report() {
  local file="$1"
  local where="$2"
  local banner="$3"
  FINDINGS=$((FINDINGS + 1))
  echo "LEAK  $file  [$where]  $banner" >&2
}


# Scan every candidate file for private key material, in ONE python3 process.
#
# The first implementation started a python3 interpreter per file, and the one
# before that shelled out to tr/base64/grep for every base64 blob; both took
# over ten minutes across 2200 files. A single interpreter that reads the file
# list and does the decoding in-process takes seconds.
#
# A hit only counts when it is a real PEM block, not a source-code literal.
# Test assertions like `key.contains("-----BEGIN PRIVATE KEY-----")` and doc
# comments quoting the header are not credentials; requiring the armored
# payload to follow is what separates the two.
#
# Output: one "<file>\t<where>\t<banner>" line per hit on stdout.
#
# The file list arrives on stdin, NUL-separated, not as argv: 2300 paths overrun
# the command line ("Argument list too long") and the scan silently reported
# clean. A directory argument is accepted too, for scanning one subtree.
# One pass over the whole file list, in a single python3 process.
mapfile -t FILES < <(collect)
SCANNED=${#FILES[@]}
if [[ ${#FILES[@]} -gt 0 ]]; then
  # Write the scanner to a fixed temp file using a here-string (does NOT
  # consume the function's stdin). Then pipe the path list to python3.
  # This worked perfectly in isolation.
  SCANNER_FILE="D:/tmp-dokipon/dokipon_scanner.py"
  cat > "$SCANNER_FILE" <<'PYEOF'
import base64, os, re, sys

PEM = rb"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----"
PAYLOAD = rb"[A-Za-z0-9+/=\s]{80,}"

def hits(data):
    m = re.search(PEM + PAYLOAD, data)
    if m:
        return [("plaintext", m.group(0).decode("ascii", "replace"))]
    for blob in set(re.findall(rb"[A-Za-z0-9+/]{120,}={0,2}", data)):
        try:
            dec = base64.b64decode(blob, validate=False)
        except Exception:
            continue
        m = re.search(PEM + PAYLOAD, dec)
        if m:
            return [("base64", m.group(0).decode("ascii", "replace"))]
    return []

data = sys.stdin.buffer.read()
paths = [p.decode("utf-8", "replace") for p in data.split(b"\0") if p]

for path in paths:
    if not os.path.isfile(path):
        continue
    try:
        fdata = open(path, "rb").read()
    except OSError:
        continue
    for where, banner in hits(fdata):
        import base64
        banner_b64 = base64.b64encode(banner.encode("utf-8")).decode("ascii")
        print(f"{path}\t{where}\t{banner_b64}")
PYEOF
  # Capture output in the main shell so `report` function is available.
  SCANNER_OUTPUT="$(printf '%s\0' "${FILES[@]}" | python3 "$SCANNER_FILE")"
  rm -f "$SCANNER_FILE"
  # The scanner emits "<file>\t<where>\t<banner>" per line. Banner may contain
  # newlines/tabs, so read line by line and split on the first two tabs only.
  # Split on the first two tabs with parameter expansion. `read` mangles the
  # leading field when IFS is a tab, which truncated the file name in the
  # report; this keeps the path and the banner intact.
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    file="${line%%$'\t'*}"
    rest="${line#*$'\t'}"
    where="${rest%%$'\t'*}"
    banner_b64="${rest#*$'\t'}"
    banner="$(printf '%s' "$banner_b64" | base64 -d 2>/dev/null)"
    report "$file" "$where" "$banner"
  done <<< "$SCANNER_OUTPUT"
fi

echo
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
