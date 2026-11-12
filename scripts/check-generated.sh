#!/bin/sh
# Exact regenerate-and-compare for every manifest in the same registry as the source-only gate.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
exec "$repo_root/scripts/python.sh" "$repo_root/scripts/check-generated.py"
