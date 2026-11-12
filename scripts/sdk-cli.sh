#!/bin/sh
# Use the shared verified build from the trusted repository configuration root.
# Source after defining repo_root; runtime/package/output remain manifest-owned.
sdk_cli() (
  cd "$repo_root"
  sh tools/standards.sh verify-receipt --artifact musubi-cli
  binary=$(sh scripts/python.sh -c 'import json; print(json.load(open(".craftguild/musubi/receipt.json"))["context"]["output"])')
  "$binary" "$@"
)
