#!/bin/sh
# Source-only: no Cargo, CLI build, runtime resolution, or network access.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
exec "$repo_root/scripts/python.sh" "$repo_root/scripts/check_lines.py" "$@"
