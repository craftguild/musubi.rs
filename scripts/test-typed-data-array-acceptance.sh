#!/bin/sh
# Verifies the retained consumer-produced Solidity dynamic-struct-array evidence.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$repo_root/scripts/sdk-cli.sh"
fixture="$repo_root/external-acceptance/typed-data-array-v1"
source_file="$fixture/solidity/test/TypedDataArrayReference.t.sol"
artifact="$fixture/solidity/out/TypedDataArrayReference.t.sol/TypedDataArrayReferenceTest.json"
vectors="$fixture/vectors.json"
manifest="$fixture/musubi.toml"

source_sha=$(shasum -a 256 "$source_file" | awk '{print $1}')
artifact_sha=$(shasum -a 256 "$artifact" | awk '{print $1}')
settings_sha=$(jq --sort-keys --compact-output '.metadata.settings' "$artifact" |
  shasum -a 256 | awk '{print $1}')
test "0x$source_sha" = "$(jq -r '.evidence.solidity_source_sha256' "$vectors")"
test "0x$artifact_sha" = "$(jq -r '.evidence.artifact_sha256' "$vectors")"
test "0x$settings_sha" = "$(jq -r '.evidence.compiler_settings_sha256' "$vectors")"
test "$(jq -r '.metadata.compiler.version' "$artifact")" = \
  "$(jq -r '.evidence.compiler_version' "$vectors")"
test "$(jq -r '.metadata.settings.compilationTarget | to_entries[0] |
  "\(.key):\(.value)"' "$artifact")" = "$(jq -r '.evidence.artifact_target' "$vectors")"
sdk_cli --manifest "$manifest" doctor
