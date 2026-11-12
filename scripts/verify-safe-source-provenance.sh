#!/bin/sh
# Verifies that fetched Safe source bytes are bound to the recorded immutable revision URL.
set -eu

revision=$1
expected_sha256=$2
package_source=$3
pinned_source=$4
source_url=$5

case "$revision" in
  *[!0-9a-f]*|'') exit 1 ;;
esac
test "${#revision}" -eq 40
case "$expected_sha256" in
  *[!0-9a-f]*|'') exit 1 ;;
esac
test "${#expected_sha256}" -eq 64
expected_url="https://raw.githubusercontent.com/safe-global/safe-smart-account/$revision/contracts/Safe.sol"
test "$source_url" = "$expected_url"
test -f "$package_source"
test -f "$pinned_source"
actual_sha256=$(shasum -a 256 "$pinned_source" | awk '{print $1}')
test "$actual_sha256" = "$expected_sha256"
cmp "$package_source" "$pinned_source"
