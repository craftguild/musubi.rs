"""Explicit synthetic ownership fixture construction; never imported by a production repair path."""

import hashlib
import json

from generated_ownership import LAYOUT, MARKER, SCHEMA


def owned_files(files):
    files = dict(files)
    records = [{"path": path, "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}
               for path, raw in sorted(files.items()) if path != MARKER]
    files[MARKER] = json.dumps({"schema_version": SCHEMA, "layout": LAYOUT, "files": records}, separators=(",", ":")).encode()
    return files
