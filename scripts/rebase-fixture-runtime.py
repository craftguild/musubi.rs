#!/usr/bin/env python3
"""Rebase an explicit Path runtime in a private fixture manifest to the candidate root.

Only test copies call this helper. Canonical inputs keep their reviewed relative runtime root.
Artifact/output fields and compiler evidence are unchanged; the edited manifest is new input.
"""

import os
from pathlib import Path
import re
import sys


def rebase(manifest, candidate):
    source = manifest.read_text()
    blocks = list(re.finditer(r'(?ms)^\[runtime\]\n.*?(?=^\[|\Z)', source))
    if len(blocks) != 1 or 'schema_version = "musubi/config-v4"' not in source:
        raise ValueError("fixture must have an explicit current Path runtime")
    fields = [line.strip() for line in blocks[0].group().splitlines()[1:]
              if line.strip() and not line.lstrip().startswith('#')]
    if len(fields) != 2 or 'source = "path"' not in fields or not any(
            re.fullmatch(r'root = "[^"\\]*"', field) for field in fields):
        raise ValueError("fixture runtime must have only explicit Path source and root")
    root = os.path.relpath(candidate.resolve(), manifest.parent.resolve())
    if any(character in root for character in '\\"\n\r'):
        raise ValueError("fixture path cannot be represented")
    replacement = f'[runtime]\nsource = "path"\nroot = "{root}"\n'
    rewritten, count = re.subn(r'(?ms)^\[runtime\]\n.*?(?=^\[|\Z)', lambda _: replacement + '\n', source)
    if count != 1:
        raise ValueError("fixture runtime table is ambiguous")
    manifest.write_text(rewritten)


if __name__ == "__main__":
    rebase(Path(sys.argv[1]), Path(sys.argv[2]))
