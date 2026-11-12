"""Exact non-SDK compiler/vendor inventory; authority and validation gate remain reviewable."""

import hashlib

from generated_ownership import HASH
from source_inventory import normal_path, read_bytes, read_json

INVENTORY = "scripts/artifact-inventory.json"


def validate_artifacts(root, sources):
    inventory = read_json(root, INVENTORY)
    if (type(inventory) is not dict or set(inventory) != {"schema_version", "groups"}
            or inventory["schema_version"] != "musubi/artifact-inventory-v1"
            or type(inventory["groups"]) is not list or not inventory["groups"]):
        raise ValueError("non-SDK artifact inventory is invalid")
    output = set()
    authorities = set()
    for group in inventory["groups"]:
        if type(group) is not dict or set(group) != {"authority", "gate", "files"}:
            raise ValueError("non-SDK artifact group is not closed")
        authority, gate, files = group["authority"], group["gate"], group["files"]
        if type(authority) is not str or not 0 < len(authority) <= 1000 or authority in authorities:
            raise ValueError("non-SDK artifact authority is missing or repeated")
        authorities.add(authority)
        if normal_path(gate) not in sources or type(files) is not list or not files:
            raise ValueError("non-SDK artifact verification gate or files are absent")
        previous = ""
        for record in files:
            if type(record) is not dict or set(record) != {"path", "bytes", "sha256"}:
                raise ValueError("non-SDK artifact record is not closed")
            path = normal_path(record["path"])
            if path <= previous or path in output or path not in sources:
                raise ValueError("non-SDK artifact inventory is not exact, sorted and unique")
            if (type(record["bytes"]) is not int or not 0 <= record["bytes"] <= 256 * 1024 * 1024
                    or type(record["sha256"]) is not str or not HASH.fullmatch(record["sha256"])):
                raise ValueError("non-SDK artifact length or digest is invalid")
            raw = read_bytes(root, path, record["bytes"])
            if len(raw) != record["bytes"] or hashlib.sha256(raw).hexdigest() != record["sha256"]:
                raise ValueError(f"non-SDK artifact changed; review its authority and gate: {path}")
            output.add(path)
            previous = path
    return output
