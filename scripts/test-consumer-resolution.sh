#!/bin/sh
# Proves the released core/runtime graph resolves with the supported downstream dependency floor.
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
manifest="$repository_root/tests/fixtures/consumer-resolution/Cargo.toml"
cargo metadata --manifest-path "$manifest" --locked --format-version 1 >/dev/null
