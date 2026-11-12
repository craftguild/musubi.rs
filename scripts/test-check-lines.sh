#!/bin/sh
# Source-only adversarial checks; the fixture SDK is synthetic and no Cargo build is required.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
exec "$repo_root/scripts/python.sh" "$repo_root/scripts/test_check_lines.py"
