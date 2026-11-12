#!/bin/sh
# Verifies the retained Solidity emitted-log harness, artifact, and vector.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fixture="$repo_root/external-acceptance/event-rebalance-v1"
source_file="$fixture/solidity/test/RebalanceEventReference.t.sol"
artifact="$fixture/solidity/out/RebalanceEventReference.t.sol/RebalanceEventReferenceTest.json"
vector="$fixture/vector.json"

source_sha=$(shasum -a 256 "$source_file" | awk '{print $1}')
artifact_sha=$(shasum -a 256 "$artifact" | awk '{print $1}')
settings_sha=$(jq --sort-keys --compact-output '.metadata.settings' "$artifact" |
  shasum -a 256 | awk '{print $1}')
test "0x$source_sha" = "$(jq -r '.evidence.solidity_source_sha256' "$vector")"
test "0x$artifact_sha" = "$(jq -r '.evidence.artifact_sha256' "$vector")"
test "0x$settings_sha" = "$(jq -r '.evidence.compiler_settings_sha256' "$vector")"
test "$(jq -r '.metadata.compiler.version' "$artifact")" = \
  "$(jq -r '.evidence.compiler_version' "$vector")"
test "$(jq -r '.metadata.settings.compilationTarget | to_entries[0] |
  "\(.key):\(.value)"' "$artifact")" = "$(jq -r '.evidence.artifact_target' "$vector")"
test "$(jq -r '.event.topics | length' "$vector")" = 4
test "$(jq -r '.event.data | length' "$vector")" = 1218
