#!/bin/sh
# Generates and compiles the committed authored ABI acceptance corpus.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$repo_root/scripts/sdk-cli.sh"
manifest="$repo_root/external-acceptance/abi-corpus-v1/input/musubi.toml"
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT HUP INT TERM
checker="$repo_root/scripts/check-abi-corpus.sh"

wait_for_snapshot_hook() {
  pid=$1
  ready=$2
  attempts=0
  while test "$attempts" -lt 500; do
    test -e "$ready" && return
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.01
    attempts=$((attempts + 1))
  done
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  exit 1
}

"$checker"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
jq '.artifact_revision_coherence = "verified"' "$temporary/generated/inventory.json" \
  >"$temporary/forged.json"
mv "$temporary/forged.json" "$temporary/generated/inventory.json"
if "$checker" "$temporary/generated" >/dev/null 2>&1; then
  exit 1
fi

rm -rf "$temporary/generated"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
awk 'NR == 2 { print "  \"artifact_revision_coherence\": \"verified\"," } { print }' \
  "$temporary/generated/inventory.json" >"$temporary/duplicate-top-level.json"
mv "$temporary/duplicate-top-level.json" "$temporary/generated/inventory.json"
if "$checker" "$temporary/generated" >/dev/null 2>&1; then
  exit 1
fi

rm -rf "$temporary/generated"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
awk '
  !inserted && /"id": "sample_executor"/ {
    print "      \"id\": \"shadow\","; inserted = 1
  }
  { print }
' "$temporary/generated/inventory.json" >"$temporary/duplicate-contract.json"
mv "$temporary/duplicate-contract.json" "$temporary/generated/inventory.json"
if "$checker" "$temporary/generated" >/dev/null 2>&1; then
  exit 1
fi

rm -rf "$temporary/generated"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
printf '\n' >>"$temporary/generated/abi/erc20.json"
if "$checker" "$temporary/generated" >/dev/null 2>&1; then
  exit 1
fi

ln -s "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/corpus-link"
if "$checker" "$temporary/corpus-link" >/dev/null 2>&1; then
  exit 1
fi

rm -rf "$temporary/generated"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
printf 'musubi/abi-corpus-v1\n' >"$temporary/external-marker"
rm "$temporary/generated/.musubi-abi-corpus"
ln -s "$temporary/external-marker" "$temporary/generated/.musubi-abi-corpus"
if "$checker" "$temporary/generated" >/dev/null 2>&1; then
  exit 1
fi

rm -rf "$temporary/generated"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
cp "$temporary/generated/inventory.json" "$temporary/external-inventory.json"
rm "$temporary/generated/inventory.json"
ln -s "$temporary/external-inventory.json" "$temporary/generated/inventory.json"
if "$checker" "$temporary/generated" >/dev/null 2>&1; then
  exit 1
fi

rm -rf "$temporary/generated"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
cp "$temporary/generated/abi/erc20.json" "$temporary/external-abi.json"
rm "$temporary/generated/abi/erc20.json"
ln -s "$temporary/external-abi.json" "$temporary/generated/abi/erc20.json"
if "$checker" "$temporary/generated" >/dev/null 2>&1; then
  exit 1
fi

rm -rf "$temporary/generated"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
jq '.artifact_revision_coherence = "verified"' "$temporary/generated/inventory.json" \
  >"$temporary/external-inventory.json"
ready="$temporary/swap-ready"
resume="$temporary/swap-resume"
MUSUBI_CORPUS_SNAPSHOT_TEST_TARGET=inventory.json \
MUSUBI_CORPUS_SNAPSHOT_TEST_READY_FILE="$ready" \
MUSUBI_CORPUS_SNAPSHOT_TEST_CONTINUE_FILE="$resume" \
  "$checker" "$temporary/generated" >/dev/null 2>&1 &
snapshot_pid=$!
wait_for_snapshot_hook "$snapshot_pid" "$ready"
mv "$temporary/generated/inventory.json" "$temporary/opened-inventory.json"
ln -s "$temporary/external-inventory.json" "$temporary/generated/inventory.json"
touch "$resume"
wait "$snapshot_pid"

rm -rf "$temporary/generated"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
ready="$temporary/growth-ready"
resume="$temporary/growth-resume"
MUSUBI_CORPUS_SNAPSHOT_TEST_TARGET=inventory.json \
MUSUBI_CORPUS_SNAPSHOT_TEST_READY_FILE="$ready" \
MUSUBI_CORPUS_SNAPSHOT_TEST_CONTINUE_FILE="$resume" \
  "$checker" "$temporary/generated" >/dev/null 2>&1 &
snapshot_pid=$!
wait_for_snapshot_hook "$snapshot_pid" "$ready"
printf '\n' >>"$temporary/generated/inventory.json"
touch "$resume"
if wait "$snapshot_pid"; then
  exit 1
fi

rm -rf "$temporary/generated"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
ready="$temporary/same-size-ready"
resume="$temporary/same-size-resume"
MUSUBI_CORPUS_SNAPSHOT_TEST_TARGET=inventory.json \
MUSUBI_CORPUS_SNAPSHOT_TEST_READY_FILE="$ready" \
MUSUBI_CORPUS_SNAPSHOT_TEST_CONTINUE_FILE="$resume" \
  "$checker" "$temporary/generated" >/dev/null 2>&1 &
snapshot_pid=$!
wait_for_snapshot_hook "$snapshot_pid" "$ready"
python3 -c '
import os, sys
with open(sys.argv[1], "r+b", buffering=0) as target:
    raw = target.read()
    marker = b"\"abi_sha256\": \"0x"
    offset = raw.index(marker) + len(marker)
    target.seek(offset)
    target.write(b"0" if raw[offset:offset + 1] != b"0" else b"1")
    os.fsync(target.fileno())
' "$temporary/generated/inventory.json"
touch "$resume"
if wait "$snapshot_pid"; then
  exit 1
fi

rm -rf "$temporary/generated"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
root_bytes=$((
  $(wc -c <"$temporary/generated/.musubi-abi-corpus") +
  $(wc -c <"$temporary/generated/musubi.toml") +
  $(wc -c <"$temporary/generated/inventory.json")
))
if MUSUBI_CORPUS_SNAPSHOT_TEST_MAXIMUM_TOTAL_BYTES="$root_bytes" \
  "$checker" "$temporary/generated" >/dev/null 2>&1; then
  exit 1
fi

rm -rf "$temporary/generated" "$temporary/too-many-snapshot"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
entry=0
while test "$entry" -lt 40; do
  printf '[]\n' >"$temporary/generated/abi/extra-$entry.json"
  entry=$((entry + 1))
done
if "$repo_root/scripts/snapshot-abi-corpus.py" \
  "$temporary/generated" "$temporary/too-many-snapshot" \
  >"$temporary/too-many-out" 2>"$temporary/too-many-error"; then
  exit 1
fi
grep -q 'ABI entry count is invalid' "$temporary/too-many-error"

rm -rf "$temporary/generated"
cp -R "$repo_root/external-acceptance/abi-corpus-v1/input" "$temporary/generated"
rm "$temporary/generated/inventory.json"
mkfifo "$temporary/generated/inventory.json"
python3 -c '
import subprocess, sys
result = subprocess.run(sys.argv[1:], timeout=2,
    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
sys.exit(0 if result.returncode != 0 else 1)
' "$checker" "$temporary/generated"

sdk_cli --manifest "$manifest" doctor
