#!/usr/bin/env bash
# Standalone current-schema SDK gates; all compiler outputs and lockfiles stay in private copies.
set -euo pipefail
repo_root=$(cd "$(dirname "$0")/.." && pwd -P)
. "$repo_root/scripts/sdk-cli.sh"
cd "$repo_root"
mkdir -p "$repo_root/target"
temporary=$(mktemp -d "$repo_root/target/standalone-fixtures.XXXXXX")
trap 'rm -rf "$temporary"' EXIT

standalone() (
  cd "$repo_root"
  local manifest=$1
  cargo generate-lockfile --manifest-path "$manifest"
  local effective_target
  effective_target=$(cargo metadata --no-deps --format-version 1 --manifest-path "$manifest" | \
    python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
  if [ "$effective_target" = "$(dirname "$manifest")/target" ] && [ -z "${CARGO_TARGET_DIR+x}" ]; then
    export CARGO_TARGET_DIR="$repo_root/target/standalone-sdk"
  fi
  cargo check --quiet --locked --all-targets --manifest-path "$manifest"
  cargo test --quiet --locked --manifest-path "$manifest"
  cargo clippy --quiet --locked --all-targets --manifest-path "$manifest" -- -D warnings
  RUSTDOCFLAGS="-D warnings" cargo doc --quiet --locked --no-deps --manifest-path "$manifest"
)

validate_fixture() {
  local name=$1 input=$2
  python3 "$repo_root/scripts/rebase-fixture-runtime.py" "$input/musubi.toml" "$repo_root"
  sdk_cli --manifest "$input/musubi.toml" generate
  sdk_cli --manifest "$input/musubi.toml" check
  cargo run --quiet --locked -p musubi-testkit --example inspect_generated -- "$input/sdk"
  cp -R "$input/sdk" "$input/standalone-sdk"
  standalone "$input/standalone-sdk/Cargo.toml"
  sdk_cli --manifest "$input/musubi.toml" check
  echo "standalone SDK fixture checked: $name"
}

cp -R "$repo_root/release-fixture" "$temporary/mixed"
printf '[]\n' >"$temporary/mixed/interface.json"
printf '\n[[contracts]]\nid = "empty_interface"\nkind = "interface"\nabi = "interface.json"\n' \
  >>"$temporary/mixed/musubi.toml"
validate_fixture mixed-and-typed "$temporary/mixed"

for name in typed-data-array-v1 typed-data-edge-v1; do
  cp -R "$repo_root/external-acceptance/$name" "$temporary/$name"
  validate_fixture "$name" "$temporary/$name"
done
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/abi-corpus"
validate_fixture abi-corpus "$temporary/abi-corpus"

mkdir "$temporary/sparse"
printf '[]\n' >"$temporary/sparse/abi.json"
cat >"$temporary/sparse/musubi.toml" <<'MANIFEST'
schema_version = "musubi/config-v4"
[runtime]
source = "path"
root = "../../.."
[package]
name = "standalone-sparse-sdk"
version = "0.1.0"
edition = "2024"
[generation]
output = "sdk"
[[contracts]]
id = "Empty"
kind = "interface"
abi = "abi.json"
MANIFEST
validate_fixture sparse "$temporary/sparse"

mkdir "$temporary/safe-source"
cp "$repo_root/Cargo.toml" "$repo_root/Cargo.lock" "$temporary/safe-source/"
cp -R "$repo_root/crates" "$temporary/safe-source/crates"
sdk_cli --manifest "$repo_root/safe-v141-sdk.toml" check
standalone "$temporary/safe-source/crates/musubi-safe-v141-generated/Cargo.toml"
sdk_cli --manifest "$repo_root/safe-v141-sdk.toml" check
echo "standalone SDK fixture checked: safe"
