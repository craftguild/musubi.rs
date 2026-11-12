#!/bin/sh
# Proves consumer rustfmt cannot rewrite generator-owned Rust bytes.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$repo_root/scripts/sdk-cli.sh"
mkdir -p "$repo_root/target"
temporary=$(mktemp -d "$repo_root/target/generated-rustfmt.XXXXXX")
trap 'rm -rf -- "$temporary"' EXIT HUP INT TERM
cp -R "$repo_root/release-fixture/." "$temporary/"
output="$temporary/sdk"
manifest="$temporary/musubi.toml"
python3 "$repo_root/scripts/rebase-fixture-runtime.py" "$manifest" "$repo_root"

sdk_cli --manifest "$manifest" generate
find "$output/src" -type f -name '*.rs' -exec shasum -a 256 {} \; | \
  LC_ALL=C sort >"$temporary/before"

cargo fmt --manifest-path "$output/Cargo.toml" --all

find "$output/src" -type f -name '*.rs' -exec shasum -a 256 {} \; | \
  LC_ALL=C sort >"$temporary/after"
cmp "$temporary/before" "$temporary/after"
sdk_cli --manifest "$manifest" check
