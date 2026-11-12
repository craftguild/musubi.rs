"""Disposable Git repository and synthetic SDK bytes for source-only gate adversarial tests."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from ownership_test_support import owned_files

GIT = shutil.which("git")


def git(root, *arguments):
    environment = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    subprocess.run([GIT, "-C", str(root), *arguments], check=True, env=environment)


class Fixture:
    def __init__(self):
        self.temporary = tempfile.TemporaryDirectory()
        # Resolve the known macOS /var alias before handing the explicit root to no-follow code.
        self.root = Path(self.temporary.name).resolve()
        git(self.root, "init", "--quiet")
        self.write("scripts/generated-manifests.json", '["musubi.toml"]\n')
        manifest = ('schema_version = "musubi/config-v4"\n'
                    '[runtime]\nsource = "path"\nroot = "missing-private-runtime"\n'
                    '[package]\nname = "fixture-sdk"\nversion = "0.1.0"\nedition = "2024"\n'
                    '[generation]\noutput = "generated/sdk"\n')
        self.write("musubi.toml", manifest)
        release = {"schema_version": "musubi/release-v4", "manifest_schema": "musubi/config-v4",
                   "manifest_sha256": "0x" + hashlib.sha256(manifest.encode()).hexdigest()}
        self.files = owned_files({
            "Cargo.toml": b'[package]\nname = "fixture-sdk"\nversion = "0.1.0"\nedition = "2024"\n',
            "src/lib.rs": b"// synthetic generated owner\n" * 401,
            "musubi-release.json": json.dumps(release).encode(),
        })
        self.install()
        self.write("scripts/artifact-gate.sh", "#!/bin/sh\nexit 0\n")
        artifact = b"[]\n" * 401
        self.write("external-acceptance/out/reviewed.json", artifact)
        self.write("scripts/artifact-inventory.json", json.dumps({
            "schema_version": "musubi/artifact-inventory-v1", "groups": [{
                "authority": "explicit synthetic compiler artifact", "gate": "scripts/artifact-gate.sh",
                "files": [{"path": "external-acceptance/out/reviewed.json", "bytes": len(artifact),
                           "sha256": hashlib.sha256(artifact).hexdigest()}],
            }],
        }))

    def write(self, path, raw):
        path = self.root / path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw.encode() if isinstance(raw, str) else raw)
        return path

    def install(self):
        for path, raw in self.files.items():
            self.write("generated/sdk/" + path, raw)

    def close(self):
        self.temporary.cleanup()
