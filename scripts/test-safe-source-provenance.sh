#!/bin/sh
# Mutation checks for the Safe source-revision provenance verifier.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT
revision=bf943f80fec5ac647159d26161446ac5d716a294
url="https://raw.githubusercontent.com/safe-global/safe-smart-account/$revision/contracts/Safe.sol"
printf 'reviewed Safe source\n' >"$temporary/package.sol"
cp "$temporary/package.sol" "$temporary/pinned.sol"
sha256=$(shasum -a 256 "$temporary/pinned.sol" | awk '{print $1}')
verify="$repo_root/scripts/verify-safe-source-provenance.sh"

"$verify" "$revision" "$sha256" "$temporary/package.sol" "$temporary/pinned.sol" "$url"
if "$verify" 0000000000000000000000000000000000000000 "$sha256" \
  "$temporary/package.sol" "$temporary/pinned.sol" "$url"; then
  echo "revision/URL mismatch was accepted" >&2
  exit 1
fi
printf 'different source\n' >"$temporary/package.sol"
if "$verify" "$revision" "$sha256" \
  "$temporary/package.sol" "$temporary/pinned.sol" "$url"; then
  echo "revision/package source mismatch was accepted" >&2
  exit 1
fi
