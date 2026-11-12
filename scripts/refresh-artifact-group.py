#!/usr/bin/env python3
"""Stage reviewed extraction hashes for an existing artifact group; never add exception paths."""

import hashlib
import json
from pathlib import Path
import sys

from artifact_inventory import INVENTORY, validate_artifacts
from source_inventory import normal_path, read_bytes, read_json, source_paths


def render(inventory):
    lines = ['{', '  "schema_version": "musubi/artifact-inventory-v1",', '  "groups": [']
    for index, group in enumerate(inventory["groups"]):
        lines.extend(['    {', '      "authority": ' + json.dumps(group["authority"]) + ',',
                      '      "gate": ' + json.dumps(group["gate"]) + ',', '      "files": ['])
        for position, record in enumerate(group["files"]):
            lines.append('        ' + json.dumps(record) + (',' if position + 1 < len(group["files"]) else ''))
        lines.extend(['      ]', '    }' + (',' if index + 1 < len(inventory["groups"]) else '')])
    return "\n".join(lines + ['  ]', '}']) + "\n"


def refresh(root, prefix, candidate, destination):
    prefix = normal_path(prefix) + "/"
    sources, _ = source_paths(root)
    validate_artifacts(root, sources)
    inventory = read_json(root, INVENTORY)
    selected = [group for group in inventory["groups"] if all(record["path"].startswith(prefix) for record in group["files"])]
    if len(selected) != 1:
        raise ValueError("refresh must select exactly one existing complete artifact group")
    for record in selected[0]["files"]:
        raw = read_bytes(candidate, record["path"][len(prefix):])
        record["bytes"], record["sha256"] = len(raw), hashlib.sha256(raw).hexdigest()
    # Destination is a new file in the updater's owned staging directory. Installation uses its
    # existing transaction together with the extraction; there is no permissive line-gate mode.
    with destination.open("x", encoding="utf-8") as output:
        output.write(render(inventory))


if __name__ == "__main__":
    if len(sys.argv) != 5:
        raise SystemExit("usage: refresh-artifact-group.py REPOSITORY REGISTERED_PREFIX CANDIDATE NEW_OUTPUT")
    refresh(Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3]), Path(sys.argv[4]))
