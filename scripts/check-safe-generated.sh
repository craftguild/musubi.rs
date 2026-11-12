#!/bin/sh
# Checks the repository Safe SDK explicit source and exact manifest provenance.
set -eu

if [ "$#" -ne 1 ] || [ "${1#/}" = "$1" ] || [ ! -d "$1" ]; then
  echo "Safe generated-crate root is invalid" >&2
  exit 2
fi

repository=$1
script_root=$(CDPATH= cd -P -- "$(dirname -- "$0")" && pwd -P)
"$script_root/python.sh" "$script_root/check-owned-sdk.py" \
  "$repository/crates/musubi-safe-v141-generated"
safe_release="$repository/crates/musubi-safe-v141-generated/musubi-release.json"
safe_manifest="$repository/crates/musubi-safe-v141-generated/Cargo.toml"
safe_configuration="$repository/safe-v141-sdk.toml"
if [ ! -f "$safe_release" ] || [ -L "$safe_release" ] || \
  [ ! -f "$safe_manifest" ] || [ -L "$safe_manifest" ] || \
  [ ! -f "$safe_configuration" ] || [ -L "$safe_configuration" ]; then
  echo "generated Safe SDK is missing or unsafe" >&2
  exit 1
fi

manifest_sha=0x$(shasum -a 256 "$safe_configuration" | awk '{print $1}')
if ! jq -e --arg digest "$manifest_sha" '
  type == "object" and
  .schema_version == "musubi/release-v4" and
  .manifest_schema == "musubi/config-v4" and .manifest_sha256 == $digest and
  (.runtime | type == "object" and keys == ["root", "source"] and
    .source == "path" and .root == "../../.")
' "$safe_release" >/dev/null; then
  echo "generated Safe SDK has inconsistent manifest or runtime identity" >&2
  exit 1
fi
