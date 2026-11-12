#!/usr/bin/env python3
"""Real-CLI acceptance for indivisible event, shared tuple and typed-data owners over 400 lines."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from generated_ownership import MARKER
from ownership_io import read_tree

ROOT = Path(__file__).resolve().parent.parent


def cli(manifest, command):
    subprocess.run(["cargo", "run", "--quiet", "--locked", "-p", "musubi-cli", "--",
                    "--manifest", str(manifest), command], cwd=ROOT, check=True)


def fixture(root):
    fields = [{"name": f"field{index}", "type": "uint256"} for index in range(140)]
    abi = [
        {"type": "event", "name": "LargeEvent", "anonymous": False,
         "inputs": [dict(field, indexed=False) for field in fields]},
        {"type": "function", "name": "largeTuple", "stateMutability": "view",
         "inputs": [{"name": "value", "type": "tuple", "components": fields}], "outputs": []},
    ]
    (root / "abi.json").write_text(json.dumps(abi))
    schema = {"schema_version": "musubi/typed-data-schema-v1",
              "types": [{"name": "LargeTypedData", "fields": fields}],
              "roots": [{"kind": "struct_hash", "type": "LargeTypedData"}]}
    (root / "schema.json").write_text(json.dumps(schema))
    helper = root / "vector-helper"
    (helper / "src").mkdir(parents=True)
    shutil.copyfile(ROOT / "scripts/large_owner_vector.rs", helper / "src/main.rs")
    (helper / "Cargo.toml").write_text('[package]\nname = "large-owner-vector"\nversion = "0.1.0"\n'
                                       'edition = "2024"\n[workspace]\n[dependencies]\n'
                                       'hex = "0.4"\nmusubi-core = { path = "../../../crates/musubi-core" }\n')
    # This input only exercises layout/resource bounds. Its synthetic evidence is not a retained
    # compiler differential vector; all real compiler and Safe goldens run in their separate gates.
    hashes = subprocess.check_output(["cargo", "run", "--quiet"], cwd=helper,
                                     env=cargo_environment(), text=True).splitlines()
    vectors = json.loads((ROOT / "external-acceptance/typed-data-edge-v1/vectors.json").read_text())
    vectors["evidence"].update(solidity_source_path="test/SyntheticLarge.t.sol",
                               artifact_target="test/SyntheticLarge.t.sol:SyntheticLarge",
                               vector_command="synthetic large-owner hashing fixture (no compiler invocation)")
    vectors["vectors"] = [{"root": "LargeTypedData", "type_hash": "0x" + hashes[0],
                           "struct_hash": "0x" + hashes[1],
                           "type_string": "LargeTypedData(" + ",".join("uint256 " + field["name"] for field in fields) + ")",
                           "value": {field["name"]: "0" for field in fields}}]
    (root / "vectors.json").write_text(json.dumps(vectors))
    manifest = root / "musubi.toml"
    manifest.write_text('schema_version = "musubi/config-v4"\n'
                        '[runtime]\nsource = "path"\nroot = "../.."\n'
                        '[package]\nname = "large-semantic-sdk"\nversion = "0.1.0"\nedition = "2024"\n'
                        '[generation]\noutput = "sdk"\n'
                        '[[contracts]]\nid = "large"\nkind = "interface"\nabi = "abi.json"\n'
                        '[[typed_data]]\nid = "large_hash"\nrust_module = "large_hash"\nauthority = "closed-schema-v1"\n'
                        'schema = "schema.json"\nvectors = "vectors.json"\n')
    return manifest


def cargo_environment():
    environment = dict(os.environ)
    target = environment.get("CARGO_TARGET_DIR")
    if target is None:
        target = str(Path(environment.get("CARGO_HOME", str(Path.home() / ".cargo"))) / "musubi/target")
    environment["CARGO_TARGET_DIR"] = str(ROOT / target)
    return environment


def main():
    with tempfile.TemporaryDirectory(prefix="semantic-large-", dir=ROOT / "target") as temporary:
        root = Path(temporary)
        manifest = fixture(root)
        cli(manifest, "generate")
        cli(manifest, "check")
        cli(manifest, "doctor")
        files = read_tree(root / "sdk")
        for prefix in ["src/large/events/", "src/large/shared_types/", "src/large_hash/types/"]:
            owners = [(path, raw.count(b"\n")) for path, raw in files.items() if path.startswith(prefix)]
            assert owners and max(lines for _, lines in owners) > 400, owners
            print("large semantic owner:", max(owners, key=lambda item: item[1]), flush=True)
        # Cargo.lock and injected tests belong to this disposable source copy, never the SDK.
        private = root / "standalone-sdk"
        shutil.copytree(root / "sdk", private)
        environment = cargo_environment()
        subprocess.run(["cargo", "clippy", "--all-targets", "--", "-D", "warnings"],
                       cwd=private, env=environment, check=True)
        cli(manifest, "check")
        assert read_tree(root / "sdk") == files
        assert MARKER in files


if __name__ == "__main__":
    main()
