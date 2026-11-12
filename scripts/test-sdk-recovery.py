#!/usr/bin/env python3
"""Adversarial recovery archive and exclusive extraction tests; no live SDK is changed."""

import os
from pathlib import Path
import runpy
import struct
import subprocess
import sys
import tempfile

SCRIPT = Path(__file__).with_name("extract-sdk-recovery.py")
MAGIC = b"musubi/recovery-v1\n"
from ownership_test_support import owned_files

MARKER = next(iter(owned_files({}).items()))
MARKER = (MARKER[0].encode(), MARKER[1])


def archive(records):
    return MAGIC + struct.pack(">Q", len(records)) + b"".join(
        struct.pack(">IQ", len(name), len(value)) + name + value
        for name, value in records
    )


def run(source, output):
    return subprocess.run(
        [sys.executable, str(SCRIPT), str(source), str(output)],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=3, check=False,
    )


def directory_budget(root):
    """Count unique implied directories before any extraction destination is created."""
    loader = runpy.run_path(str(SCRIPT))["load"]
    source = root / "directory-archive"
    records = [(path.encode(), raw) for path, raw in owned_files({f"d{index}/nested/value": b"" for index in range(5000)}).items()]
    source.write_bytes(archive(records))
    assert len(loader(source)) == 5001  # Exactly 10,000 distinct directory prefixes.
    source.write_bytes(archive(records + [(b"excess/nested/value", b"")]))
    try:
        loader(source)
    except ValueError:
        pass
    else:
        raise AssertionError("recovery loader accepted more than 10,000 directories")
    rejected = root / "excess-directories"
    assert run(source, rejected).returncode != 0
    assert not rejected.exists()


def main():
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary).resolve()
        directory_budget(root)
        source = root / "archive"
        records = [(path.encode(), raw) for path, raw in owned_files({"src/lib.rs": b"//! recovered\n"}).items()]
        valid = archive(records)
        source.write_bytes(valid)
        output = root / "restored"
        assert run(source, output).returncode == 0
        assert (output / "src/lib.rs").read_bytes() == b"//! recovered\n"
        assert source.read_bytes() == valid
        assert run(source, output).returncode != 0

        invalid = [
            archive([(b".musubi-generated", b"musubi/generated-v1\n")]),
            b"unsupported\n", valid[:-1], valid + b"trailing",
            MAGIC + struct.pack(">Q", 10_001),
            MAGIC + struct.pack(">QIQ", 1, 16_641, 0),
            MAGIC + struct.pack(">QIQ", 1, 1, 256 * 1024 * 1024 + 1),
            archive([MARKER, MARKER]),
            archive([MARKER, (b"src", b"file"), (b"src/lib.rs", b"child")]),
            archive([MARKER, (b"src/lib.rs", b"child"), (b"src", b"file")]),
        ]
        for name in [b"../escape", b"/absolute", b"./dot", b"a//b", b"a\\b", b"a\x00b", b"a\nb", b"a/" * 65 + b"b"]:
            invalid.append(archive([MARKER, (name, b"invalid")]))
        for raw in invalid:
            source.write_bytes(raw)
            rejected = root / "must-not-exist"
            result = run(source, rejected)
            assert result.returncode != 0
            assert not rejected.exists()
            assert str(root).encode() not in result.stderr

        source.write_bytes(valid)
        for kind in ("file", "directory", "link", "fifo"):
            destination = root / kind
            if kind == "file":
                destination.write_bytes(b"keep")
            elif kind == "directory":
                destination.mkdir()
            elif kind == "link":
                destination.symlink_to(output, target_is_directory=True)
            else:
                os.mkfifo(destination)
            assert run(source, destination).returncode != 0
        linked_source = root / "source-link"
        linked_source.symlink_to(source)
        assert run(linked_source, root / "absent").returncode != 0
        fifo_source = root / "source-fifo"
        os.mkfifo(fifo_source)
        assert run(fifo_source, root / "absent").returncode != 0
        sparse = root / "sparse"
        with sparse.open("wb") as target:
            target.truncate(1024 * 1024 * 1024)
        assert run(sparse, root / "absent").returncode != 0
        assert (root / "file").read_bytes() == b"keep"
        assert (output / "src/lib.rs").read_bytes() == b"//! recovered\n"


if __name__ == "__main__":
    main()
