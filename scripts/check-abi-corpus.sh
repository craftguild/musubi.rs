#!/bin/sh
# Checks the repo-owned ABI corpus structure and binds inventory to exact ABI bytes.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
if [ "$#" -gt 1 ]; then
  echo "usage: $0 [generated-corpus]" >&2
  exit 2
fi
source_corpus=${1:-"$repo_root/external-acceptance/abi-corpus-v1/input"}
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT
corpus="$temporary/corpus"
"$repo_root/scripts/snapshot-abi-corpus.py" "$source_corpus" "$corpus"
manifest="$corpus/musubi.toml"
inventory="$corpus/inventory.json"
abi_directory="$corpus/abi"
marker="$corpus/.musubi-abi-corpus"

test -d "$corpus" && test ! -L "$corpus"
test -f "$manifest" && test ! -L "$manifest"
test -f "$inventory" && test ! -L "$inventory"
test -d "$abi_directory" && test ! -L "$abi_directory"
test -f "$marker" && test ! -L "$marker"

test "$(cat "$marker")" = \
  'musubi/abi-corpus-v1'
python3 "$repo_root/scripts/render-abi-corpus-manifest.py" \
  <"$inventory" >"$temporary/expected-manifest.toml"
cmp "$temporary/expected-manifest.toml" "$manifest"

awk '
  function finish() {
    if (!active) return
    if (id == "" || rust_module != id || kind != "interface" ||
        abi != "abi/" id ".json") exit 1
    print id
    count++
  }
  /^\[\[contracts\]\]$/ {
    finish()
    active = 1
    id = rust_module = kind = abi = ""
    next
  }
  active && /^id = "/ { id = $0; sub(/^id = "/, "", id); sub(/"$/, "", id) }
  active && /^rust_module = "/ {
    rust_module = $0; sub(/^rust_module = "/, "", rust_module); sub(/"$/, "", rust_module)
  }
  active && /^kind = "/ { kind = $0; sub(/^kind = "/, "", kind); sub(/"$/, "", kind) }
  active && /^abi = "/ { abi = $0; sub(/^abi = "/, "", abi); sub(/"$/, "", abi) }
  END { finish(); if (count != 25) exit 1 }
' "$manifest" | LC_ALL=C sort >"$temporary/manifest-ids"

jq -r '.contracts[].id' "$inventory" | LC_ALL=C sort >"$temporary/inventory-ids"
find "$abi_directory" -type f -name '*.json' -print | while IFS= read -r path; do
  basename "$path" .json
done | LC_ALL=C sort >"$temporary/abi-ids"
test "$(wc -l <"$temporary/abi-ids" | tr -d ' ')" -eq 25
test "$(find "$abi_directory" -mindepth 1 -maxdepth 1 | wc -l | tr -d ' ')" -eq 25
diff -u "$temporary/inventory-ids" "$temporary/manifest-ids"
diff -u "$temporary/inventory-ids" "$temporary/abi-ids"

jq -r '.contracts[] | "\(.id)|\(.abi_sha256)"' "$inventory" |
while IFS='|' read -r id expected; do
  abi="$abi_directory/$id.json"
  test -f "$abi" && test ! -L "$abi"
  jq -e 'type == "array"' "$abi" >/dev/null
  actual=0x$(shasum -a 256 "$abi" | awk '{print $1}')
  test "$actual" = "$expected"
done
