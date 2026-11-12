#!/bin/sh
# Proves Safe source metadata, closed fields and regular-file checks fail closed.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
temporary=$(mktemp -d)
temporary=$(CDPATH= cd -P -- "$temporary" && pwd -P)
trap 'rm -rf "$temporary"' EXIT HUP INT TERM
fixture="$temporary/repository"
generated="$fixture/crates/musubi-safe-v141-generated"
mkdir -p "$fixture/crates"
cp -R "$repo_root/crates/musubi-safe-v141-generated" "$generated"
cp "$repo_root/safe-v141-sdk.toml" "$fixture/safe-v141-sdk.toml"
cp "$repo_root/crates/musubi-safe-v141-generated/Cargo.toml" "$generated/Cargo.toml"
cp "$repo_root/crates/musubi-safe-v141-generated/musubi-release.json" \
  "$generated/musubi-release.json"

# Only this freshly copied synthetic test tree is resealed, so subsequent rejections exercise
# the closed Safe release check after ownership succeeds rather than merely stale SHA records.
seal_fixture() {
  "$repo_root/scripts/python.sh" - "$repo_root/scripts" "$generated" <<'PYTHON'
import sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
from ownership_test_support import owned_files
root = Path(sys.argv[2])
files = {str(path.relative_to(root)): path.read_bytes() for path in root.rglob("*") if path.is_file()}
(root / ".musubi-generated").write_bytes(owned_files(files)[".musubi-generated"])
PYTHON
}

"$repo_root/scripts/check-safe-generated.sh" "$fixture"
jq '.runtime = {source: "path", root: "../different"}' \
  "$generated/musubi-release.json" >"$temporary/different.json"
mv "$temporary/different.json" "$generated/musubi-release.json"
seal_fixture
if "$repo_root/scripts/check-safe-generated.sh" "$fixture" >/dev/null 2>&1; then
  exit 1
fi

cp "$repo_root/crates/musubi-safe-v141-generated/musubi-release.json" \
  "$generated/musubi-release.json"
jq '.runtime.unreviewed = true' "$generated/musubi-release.json" \
  >"$temporary/unknown.json"
mv "$temporary/unknown.json" "$generated/musubi-release.json"
seal_fixture
if "$repo_root/scripts/check-safe-generated.sh" "$fixture" >/dev/null 2>&1; then
  exit 1
fi

external="$temporary/external-release.json"
cp "$repo_root/crates/musubi-safe-v141-generated/musubi-release.json" "$external"
rm "$generated/musubi-release.json"
ln -s "$external" "$generated/musubi-release.json"
if "$repo_root/scripts/check-safe-generated.sh" "$fixture" >/dev/null 2>&1; then
  exit 1
fi
