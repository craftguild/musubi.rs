#!/usr/bin/env python3
"""Convert the canonical SDK registry to ignored shared ownership evidence."""

import hashlib
import json
from pathlib import Path
import re
import tomllib

from generated_registry import REGISTRY, registered_roots, validate_registered
from source_inventory import read_bytes

ROOT = Path(__file__).resolve().parent.parent


def digest(data):
    return hashlib.sha256(data).hexdigest()


def group(manifest, output, config):
    owner = config["package"]["name"]
    if not isinstance(owner, str) or not re.fullmatch(r"[A-Za-z0-9_-]+", owner):
        raise ValueError("Invalid shared SDK owner")
    return {
        "owner": owner,
        "manifest": f".craftguild/ownership/{owner}.json",
        "output_root": output,
        "reproduce": ["cargo", "+1.95.0", "run", "--locked", "--quiet", "--target-dir",
                      "target/product-tests", "-p", "musubi-codegen", "--example",
                      "reproduce-owned", "--", manifest, "{output}"],
        "rust_size_exception": True,
    }


def inputs(manifest, config):
    paths = {"Cargo.toml", "Cargo.lock", "craftguild.toml", REGISTRY, manifest,
             "scripts/standards-ownership.py", "scripts/generated_registry.py",
             "scripts/generated_ownership.py", "scripts/ownership_io.py",
             "scripts/ownership_unicode.py", "scripts/source_inventory.py",
             "crates/musubi-codegen/examples/reproduce-owned.rs"}
    for name in ("musubi-core", "musubi-runtime", "musubi-codegen", "musubi-testkit"):
        paths.add(f"crates/{name}/Cargo.toml")
        paths.update(path.relative_to(ROOT).as_posix()
                     for path in (ROOT / "crates" / name / "src").rglob("*.rs"))
    for contract in config.get("contracts", []):
        for name in ("abi", "creation_bytecode", "runtime_bytecode", "ast", "metadata", "storage_layout"):
            if name in contract:
                paths.add((Path(manifest).parent / contract[name]).as_posix())
    for typed in config.get("typed_data", []):
        paths.update((Path(manifest).parent / typed[name]).as_posix() for name in ("schema", "vectors"))
    return {name: digest(read_bytes(ROOT, name)) for name in sorted(paths)}


def main():
    entries = registered_roots(ROOT)
    owned = validate_registered(ROOT, entries)
    groups = [group(manifest, output, config) for manifest, output, _, config in entries]
    configured = tomllib.loads(read_bytes(ROOT, "craftguild.toml").decode())["generated"]
    if groups != configured:
        raise ValueError("Shared generated declarations differ from the canonical SDK registry")
    for selected, (manifest, output, _, config) in zip(groups, entries):
        source = inputs(manifest, config)
        record = {
            "schema_version": 1,
            "owner": selected["owner"],
            "generator_revision": digest(json.dumps(source, sort_keys=True).encode()),
            "inputs": source,
            "files": {name: digest(data) for name, data in sorted(owned.items())
                      if name.startswith(output + "/")},
        }
        destination = ROOT / selected["manifest"]
        for path in (ROOT / ".craftguild", destination.parent, destination):
            if path.is_symlink():
                raise ValueError("Symlink in shared ownership evidence")
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(json.dumps(record, indent=2) + "\n")


if __name__ == "__main__":
    main()
