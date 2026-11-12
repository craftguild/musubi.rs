#!/usr/bin/env bash
# Proves second-rename failure restores every pre-update byte.
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
source "$repo_root/scripts/update-transaction.sh"
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT
mkdir "$temporary/output" "$temporary/staged" "$temporary/staged-second"
printf 'old\n' >"$temporary/output/value"
printf 'new\n' >"$temporary/staged/value"
printf 'second\n' >"$temporary/staged-second/value"

# A failed BSD-format probe can emit filesystem text on GNU stat before fallback.
# Keep this tool behavior local; all transaction cases below use the native stat.
(
  expected=$(python3 -c 'import os, sys; print(os.stat(sys.argv[1]).st_dev)' "$temporary")
  stat() {
    case "$1" in
      -f) printf 'filesystem details for %s\n' "$3"; return 1 ;;
      -c) python3 -c 'import os, sys; print(os.stat(sys.argv[1]).st_dev)' "$3" ;;
      *) return 2 ;;
    esac
  }
  actual=$(path_device "$temporary")
  if test "$actual" != "$expected"; then
    printf 'device fallback mixed failed probe output: expected %s, received %s\n' \
      "$expected" "$actual" >&2
    exit 1
  fi
)

if MUSUBI_TEST_DEVICE_MISMATCH_INDEX=1 replace_paths_transactionally \
  "$temporary/transaction-device-mismatch" \
  "$temporary/staged" "$temporary/output"; then
  exit 1
fi
test "$(cat "$temporary/output/value")" = old
test "$(cat "$temporary/staged/value")" = new
test ! -e "$temporary/transaction-device-mismatch"

if MUSUBI_TEST_FAIL_INSTALL_INDEX=1 replace_paths_transactionally \
  "$temporary/transaction-fail-first" \
  "$temporary/staged" "$temporary/output"; then
  exit 1
fi
test "$(cat "$temporary/output/value")" = old
test "$(cat "$temporary/staged/value")" = new

mkdir "$temporary/output-second"
printf 'old-second\n' >"$temporary/output-second/value"
if MUSUBI_TEST_FAIL_INSTALL_INDEX=2 replace_paths_transactionally \
  "$temporary/transaction-fail-second" \
  "$temporary/staged" "$temporary/output" \
  "$temporary/staged-second" "$temporary/output-second"; then
  exit 1
fi
test "$(cat "$temporary/output/value")" = old
test "$(cat "$temporary/output-second/value")" = old-second
test "$(cat "$temporary/staged/value")" = new
test "$(cat "$temporary/staged-second/value")" = second

preserved="$temporary/preserved-transaction"
if MUSUBI_TEST_FAIL_INSTALL_INDEX=1 MUSUBI_TEST_FAIL_ROLLBACK_INDEX=1 \
  replace_paths_transactionally "$preserved" \
    "$temporary/staged" "$temporary/output"; then
  exit 1
fi
test ! -e "$temporary/output"
test "$(cat "$preserved/backups/0/value")" = old
mv "$preserved/backups/0" "$temporary/output"
rm -rf "$preserved"

raced="$temporary/raced-transaction"
if MUSUBI_TEST_RACE_INSTALL_INDEX=1 replace_paths_transactionally "$raced" \
  "$temporary/staged" "$temporary/output"; then
  exit 1
fi
test "$(cat "$temporary/output/intruder")" = "racing markerless output"
test "$(cat "$raced/backups/0/value")" = old
test "$(cat "$temporary/staged/value")" = new
rm -rf "$temporary/output"
rename_no_replace "$raced/backups/0" "$temporary/output"
rm -rf "$raced"

raced_second="$temporary/raced-second-transaction"
raced_second_output="$temporary/raced-second-output"
raced_second_staged="$temporary/raced-second-staged"
mkdir "$raced_second_output" "$raced_second_staged"
printf 'old-second\n' >"$raced_second_output/value"
printf 'second\n' >"$raced_second_staged/value"
if MUSUBI_TEST_RACE_INSTALL_INDEX=2 replace_paths_transactionally "$raced_second" \
  "$temporary/staged" "$temporary/output" \
  "$raced_second_staged" "$raced_second_output"; then
  exit 1
fi
test "$(cat "$temporary/output/value")" = old
test "$(cat "$temporary/staged/value")" = new
test "$(cat "$raced_second_output/intruder")" = "racing markerless output"
test "$(cat "$raced_second/backups/1/value")" = old-second
rm -rf "$raced_second_output"
rename_no_replace "$raced_second/backups/1" "$raced_second_output"
rm -rf "$raced_second"

replace_paths_transactionally "$temporary/transaction-success" \
  "$temporary/staged" "$temporary/output" \
  "$temporary/staged-second" "$temporary/output-second"
test "$(cat "$temporary/output/value")" = new
test "$(cat "$temporary/output-second/value")" = second
