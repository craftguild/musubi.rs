#!/usr/bin/env bash
# Exercises compiler-artifact same-handle snapshot and exact-target checks.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT
artifact_root="$temporary/root"
mkdir -p "$artifact_root/artifacts"
artifact="$artifact_root/artifacts/artifact.json"
cat >"$artifact" <<'JSON'
{"abi":[],"rawMetadata":"{\"compiler\":{\"version\":\"0.8.30+commit.73712a01\"},\"settings\":{\"compilationTarget\":{\"src/Vault.sol\":\"Vault\"}},\"sources\":{\"src/Vault.sol\":{\"keccak256\":\"0x1111111111111111111111111111111111111111111111111111111111111111\"}}}"}
JSON
expected_target=src/Vault.sol:Vault

"$repo_root/scripts/snapshot-compiler-artifact.py" \
  "$artifact_root" artifacts/artifact.json "$expected_target" vault contract/out/Vault.json \
  "$temporary/abi.json" "$temporary/record.json"
test "$(cat "$temporary/abi.json")" = '[]'
test "$(jq -r .compilation_target "$temporary/record.json")" = "$expected_target"

if "$repo_root/scripts/snapshot-compiler-artifact.py" \
  "$artifact_root" artifacts/artifact.json src/Other.sol:Other vault contract/out/Vault.json \
  "$temporary/wrong-abi.json" "$temporary/wrong-record.json"; then
  exit 1
fi
test ! -e "$temporary/wrong-abi.json"

outside="$temporary/outside.json"
cp "$artifact" "$outside"
ready="$temporary/swap-ready"
resume="$temporary/swap-resume"
MUSUBI_SNAPSHOT_TEST_READY_FILE=$ready \
MUSUBI_SNAPSHOT_TEST_CONTINUE_FILE=$resume \
  "$repo_root/scripts/snapshot-compiler-artifact.py" \
  "$artifact_root" artifacts/artifact.json "$expected_target" vault contract/out/Vault.json \
  "$temporary/swap-abi.json" "$temporary/swap-record.json" &
snapshot_pid=$!
while ! test -e "$ready"; do sleep 0.01; done
mv "$artifact" "$temporary/original.json"
ln -s "$outside" "$artifact"
touch "$resume"
wait "$snapshot_pid"
test "$(shasum -a 256 "$temporary/original.json" | awk '{print $1}')" = \
  "$(jq -r .artifact_sha256 "$temporary/swap-record.json" | sed 's/^0x//')"

rm "$artifact"
cp "$temporary/original.json" "$artifact"
ready="$temporary/growth-ready"
resume="$temporary/growth-resume"
MUSUBI_SNAPSHOT_TEST_READY_FILE=$ready \
MUSUBI_SNAPSHOT_TEST_CONTINUE_FILE=$resume \
  "$repo_root/scripts/snapshot-compiler-artifact.py" \
  "$artifact_root" artifacts/artifact.json "$expected_target" vault contract/out/Vault.json \
  "$temporary/growth-abi.json" "$temporary/growth-record.json" &
snapshot_pid=$!
while ! test -e "$ready"; do sleep 0.01; done
printf ' ' >>"$artifact"
touch "$resume"
if wait "$snapshot_pid"; then exit 1; fi
test ! -e "$temporary/growth-abi.json"

fifo="$temporary/artifact.fifo"
mkfifo "$fifo"
if "$repo_root/scripts/snapshot-compiler-artifact.py" \
  "$temporary" artifact.fifo "$expected_target" vault contract/out/Vault.json \
  "$temporary/fifo-abi.json" "$temporary/fifo-record.json"; then
  exit 1
fi

mkdir "$temporary/outside-directory"
cp "$temporary/original.json" "$temporary/outside-directory/artifact.json"
ln -s "$temporary/outside-directory" "$artifact_root/linked"
if "$repo_root/scripts/snapshot-compiler-artifact.py" \
  "$artifact_root" linked/artifact.json "$expected_target" vault contract/out/Vault.json \
  "$temporary/link-abi.json" "$temporary/link-record.json"; then
  exit 1
fi
test ! -e "$temporary/link-abi.json"
