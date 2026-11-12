#!/bin/sh
# Proves deterministic config-v4 corpus manifests derive only from reviewed record identities.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT HUP INT TERM
inventory="$repo_root/external-acceptance/abi-corpus-v1/input/inventory.json"
renderer="$repo_root/scripts/render-abi-corpus-manifest.py"
python3 - "$renderer" <<'PY'
import subprocess
import sys
result = subprocess.run([sys.executable, sys.argv[1]], input=b'[' * 1100 + b'0' + b']' * 1100,
                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=3, check=False)
assert result.returncode == 1
assert result.stdout == b''
assert result.stderr == b'invalid corpus manifest authority\n'
PY
python3 "$renderer" <"$inventory" >"$temporary/first"
python3 "$renderer" <"$inventory" >"$temporary/second"
cmp "$temporary/first" "$temporary/second"
test "$(head -n 1 "$temporary/first")" = 'schema_version = "musubi/config-v4"'
test "$(grep -c '^output = "sdk"$' "$temporary/first")" -eq 1
test "$(grep -c '^\[\[contracts\]\]$' "$temporary/first")" -eq 25
for mutation in \
  '.contracts[0].id = "../escape"' \
  '.contracts[0].id = .contracts[1].id' \
  '.contracts = []' \
  '.artifact_revision_coherence = "verified"'; do
  jq "$mutation" "$inventory" >"$temporary/invalid"
  if python3 "$renderer" <"$temporary/invalid" >"$temporary/rejected" 2>/dev/null; then
    exit 1
  fi
  test ! -s "$temporary/rejected"
done
