"""Manifest-only SDK registry shared by source ownership and mandatory CLI exact checks."""

import hashlib
from pathlib import PurePosixPath
import tomllib

from generated_ownership import MARKER
from ownership_io import read_tree
from ownership_unicode import casefold
from source_inventory import normal_path, read_bytes, read_json, closed_object
import json

REGISTRY = "scripts/generated-manifests.json"


def registered_roots(root):
    manifests = read_json(root, REGISTRY)
    if (type(manifests) is not list or not manifests or len(manifests) > 1000
            or any(type(path) is not str for path in manifests)
            or manifests != sorted(set(manifests))):
        raise ValueError("generated manifest registry must be nonempty, sorted and unique")
    result = []
    folded_manifests = set()
    for manifest_path in manifests:
        normal_path(manifest_path)
        folded = casefold(manifest_path)
        if folded in folded_manifests:
            raise ValueError("generated manifest registry has a casefold collision")
        folded_manifests.add(folded)
        raw = read_bytes(root, manifest_path, 1024 * 1024)
        config = tomllib.loads(raw.decode("utf-8"))
        if config.get("schema_version") != "musubi/config-v4":
            raise ValueError("registered manifest is not config-v4")
        generation = config.get("generation")
        if type(generation) is not dict or set(generation) != {"output"}:
            raise ValueError("registered generation table must contain only output")
        output = normal_path(generation["output"])
        output = normal_path(str(PurePosixPath(manifest_path).parent / output))
        folded_output = casefold(output)
        for _, other, _, _ in result:
            other = casefold(other)
            if other == folded_output or other.startswith(folded_output + "/") or folded_output.startswith(other + "/"):
                raise ValueError("registered SDK roots overlap or collide")
        result.append((manifest_path, output, raw, config))
    return result


def validate_registered(root, entries=None):
    """Bind manifest bytes and package identity to the retained release and exact owned tree."""
    entries = registered_roots(root) if entries is None else entries
    owned = {}
    for manifest, output, raw, config in entries:
        files = read_tree(root / output)
        if not {"Cargo.toml", "src/lib.rs", "musubi-release.json", MARKER} <= files.keys():
            raise ValueError(f"registered SDK is incomplete: {output}")
        release = json.loads(files["musubi-release.json"].decode("utf-8"), object_pairs_hook=closed_object)
        if (release.get("schema_version") != "musubi/release-v4"
                or release.get("manifest_schema") != "musubi/config-v4"
                or release.get("manifest_sha256") != "0x" + hashlib.sha256(raw).hexdigest()):
            raise ValueError(f"registered manifest/release identity differs: {manifest}")
        cargo = tomllib.loads(files["Cargo.toml"].decode("utf-8"))
        package = config.get("package", {})
        if any(cargo.get("package", {}).get(key) != package.get(key) for key in ("name", "version", "edition")):
            raise ValueError(f"registered manifest/package identity differs: {manifest}")
        owned.update((f"{output}/{path}", contents) for path, contents in files.items())
    return owned
