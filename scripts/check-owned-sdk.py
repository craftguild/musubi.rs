#!/usr/bin/env python3
"""Read-only ownership validation for an explicitly supplied SDK; no legacy repair or exact claim."""

from pathlib import Path
import sys

from ownership_io import read_tree

if __name__ == "__main__":
    try:
        if len(sys.argv) != 2:
            raise ValueError("usage: check-owned-sdk.py SDK_ROOT")
        files = read_tree(Path(sys.argv[1]))
        print(f"owned SDK: {len(files)} files, {sum(map(len, files.values()))} bytes")
    except (OSError, ValueError, RuntimeError, RecursionError) as error:
        print(f"owned SDK: {error}", file=sys.stderr)
        raise SystemExit(1) from error
