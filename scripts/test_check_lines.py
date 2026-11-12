#!/usr/bin/env python3
"""Product regressions for registration, complete SDK ownership and reviewed artifacts."""

from contextlib import redirect_stdout
import io
import importlib.util
import json
import os
from pathlib import Path
import unittest
from unittest.mock import patch

from check_lines import check
from generated_ownership import MARKER
from line_test_support import Fixture, GIT, git
from ownership_test_support import owned_files


class LineTests(unittest.TestCase):
    def setUp(self):
        self.fixture = Fixture()
        self.addCleanup(self.fixture.close)
        self.root = self.fixture.root

    def run_gate(self):
        with redirect_stdout(io.StringIO()):
            return check(self.root)

    def rejects(self):
        with self.assertRaises((OSError, ValueError, UnicodeError)):
            self.run_gate()

    def test_source_only_without_cargo_cli_runtime_or_network(self):
        binary = self.root / "bin"
        binary.mkdir()
        wrapper = self.fixture.write("bin/git", "#!/bin/sh\nexec "+ GIT + " \"$@\"\n")
        wrapper.chmod(0o700)
        with patch.dict(os.environ, {"PATH": str(binary), "CARGO_HOME": str(self.root / "unavailable")}):
            result = self.run_gate()
        self.assertEqual(result["generated_rust"], 1)
        self.assertEqual(result["artifacts"], 1)
        self.assertGreater(result["authored_files"], 0)

    def test_registry_missing_empty_duplicate_unknown_and_removed_entry(self):
        registry = self.root / "scripts/generated-manifests.json"
        for text in ['[]', '["musubi.toml", "musubi.toml"]', '["missing.toml"]', '{}']:
            registry.write_text(text)
            self.rejects()
        registry.unlink()
        self.rejects()

    def test_unregistered_marker_never_adopts_a_root(self):
        self.fixture.write("unregistered/" + MARKER, b"musubi/generated-v1\n")
        self.rejects()

    def test_unknown_short_file_empty_directory_missing_file_and_hash_drift(self):
        short = self.fixture.write("generated/sdk/unknown.rs", "// short\n")
        self.rejects()
        short.unlink()
        empty = self.root / "generated/sdk/empty"
        empty.mkdir()
        self.rejects()
        empty.rmdir()
        source = self.root / "generated/sdk/src/lib.rs"
        source.write_bytes(b"// changed\n")
        self.rejects()
        source.unlink()
        self.rejects()

    def test_manifest_release_binding_and_root_overlap(self):
        self.fixture.write("musubi.toml", (self.root / "musubi.toml").read_bytes() + b"\n")
        self.rejects()
        self.fixture.write("second.toml", (self.root / "musubi.toml").read_bytes())
        self.fixture.write("scripts/generated-manifests.json", '["musubi.toml", "second.toml"]')
        self.rejects()

    def test_tracking_deletion_is_distinct_from_read_and_git_failures(self):
        gone = self.fixture.write("gone.rs", "// tracked\n")
        git(self.root, "add", "--", "gone.rs")
        gone.unlink()
        self.assertEqual(self.run_gate()["tracked_deletions"], 1)
        with patch("source_inventory.read_open_file", side_effect=PermissionError("denied")):
            self.rejects()
        binary = self.root / "bin"
        binary.mkdir()
        failed = self.fixture.write("bin/git", "#!/bin/sh\nexit 7\n")
        failed.chmod(0o700)
        with patch.dict(os.environ, {"PATH": str(binary)}):
            self.rejects()

    def test_symlink_parent_final_fifo_and_non_utf8_names(self):
        source = self.fixture.write("manual.rs", "// handwritten\n")
        source.unlink()
        source.symlink_to(self.root / "musubi.toml")
        self.rejects()
        source.unlink()
        os.mkfifo(source)
        self.rejects()
        source.unlink()
        (self.root / "alias").symlink_to(self.root / "generated", target_is_directory=True)
        self.rejects()
        (self.root / "alias").unlink()
        try:
            descriptor = os.open(os.fsencode(self.root) + b"/bad-\xff.rs", os.O_CREAT | os.O_WRONLY, 0o600)
        except OSError as error:
            if error.errno != 92:  # macOS refuses invalid UTF-8 names at creation.
                raise
        else:
            os.close(descriptor)
            self.rejects()

    def test_artifact_exemption_is_exact_and_missing_authority_gate_fails(self):
        artifact = self.root / "external-acceptance/out/reviewed.json"
        artifact.write_bytes(artifact.read_bytes() + b" ")
        self.rejects()
        inventory = json.loads((self.root / "scripts/artifact-inventory.json").read_text())
        inventory["groups"][0]["gate"] = "missing.sh"
        self.fixture.write("scripts/artifact-inventory.json", json.dumps(inventory))
        self.rejects()

    def test_artifact_refresh_updates_only_an_existing_reviewed_group(self):
        source = Path(__file__).resolve().parent / "refresh-artifact-group.py"
        spec = importlib.util.spec_from_file_location("artifact_refresh_fixture", source)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        candidate = self.root / "candidate"
        self.fixture.write("candidate/reviewed.json", b"[1]\n")
        destination = self.root / "staged-inventory.json"
        module.refresh(self.root, "external-acceptance/out", candidate, destination)
        record = json.loads(destination.read_text())["groups"][0]["files"][0]
        self.assertEqual(record["path"], "external-acceptance/out/reviewed.json")
        self.assertEqual(record["bytes"], 4)
        with self.assertRaises(FileExistsError):
            module.refresh(self.root, "external-acceptance/out", candidate, destination)
        with self.assertRaises(ValueError):
            module.refresh(self.root, "unknown", candidate, self.root / "unknown.json")
        self.fixture.write("external-acceptance/out/reviewed.json", b"edited previous artifact")
        with self.assertRaises(ValueError):
            module.refresh(self.root, "external-acceptance/out", candidate, self.root / "invalid.json")

    def test_self_consistent_marker_is_ownership_only(self):
        self.fixture.files["src/lib.rs"] = b"// self-consistent changed bytes\n" * 401
        self.fixture.files = owned_files(self.fixture.files)
        self.fixture.install()
        self.run_gate()  # Mandatory CLI exact regeneration, separately wired, must reject this.


if __name__ == "__main__":
    unittest.main()
