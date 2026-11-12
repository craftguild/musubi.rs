#!/usr/bin/env python3
"""Snapshots one regular compiler artifact and derives bound ABI evidence."""

import hashlib
import json
import os
import re
import stat
import sys
import time

MAXIMUM_BYTES = 64 * 1024 * 1024
DIGEST = re.compile(r"^0x[0-9a-f]{64}$")
COMPILER = re.compile(
    r"^(?:solc )?(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\."
    r"(?:0|[1-9][0-9]*)(?:\+commit\.[0-9a-f]{8})?$"
)


def reject_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON field")
        result[key] = value
    return result


def load_closed_json(raw):
    return json.loads(raw, object_pairs_hook=reject_duplicates)


def wait_for_test_hook():
    ready = os.environ.get("MUSUBI_SNAPSHOT_TEST_READY_FILE")
    resume = os.environ.get("MUSUBI_SNAPSHOT_TEST_CONTINUE_FILE")
    if not ready and not resume:
        return
    if not ready or not resume:
        raise ValueError("incomplete test hook")
    with open(ready, "xb") as marker:
        marker.write(b"ready\n")
    deadline = time.monotonic() + 5
    while not os.path.exists(resume):
        if time.monotonic() >= deadline:
            raise ValueError("test hook timed out")
        time.sleep(0.01)


def open_artifact(root, relative):
    if os.path.isabs(relative):
        raise ValueError("artifact path must be relative")
    components = relative.split("/")
    if not components or any(part in ("", ".", "..") for part in components):
        raise ValueError("artifact path is not canonical")
    directory_flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
    directory_flags |= getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    current = os.open(root, directory_flags)
    try:
        for component in components[:-1]:
            child = os.open(component, directory_flags, dir_fd=current)
            os.close(current)
            current = child
        flags = os.O_RDONLY | os.O_NONBLOCK
        flags |= getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
        return os.open(components[-1], flags, dir_fd=current)
    finally:
        os.close(current)


def snapshot(root, relative):
    descriptor = open_artifact(root, relative)
    try:
        before = os.fstat(descriptor)
        if not stat.S_ISREG(before.st_mode) or before.st_size > MAXIMUM_BYTES:
            raise ValueError("artifact is not a bounded regular file")
        wait_for_test_hook()
        chunks = []
        remaining = MAXIMUM_BYTES + 1
        while remaining:
            chunk = os.read(descriptor, min(1024 * 1024, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        raw = b"".join(chunks)
        after = os.fstat(descriptor)
        identity = (
            before.st_dev,
            before.st_ino,
            before.st_size,
            before.st_mtime_ns,
        )
        final_identity = (
            after.st_dev,
            after.st_ino,
            after.st_size,
            after.st_mtime_ns,
        )
        if len(raw) > MAXIMUM_BYTES or len(raw) != before.st_size or identity != final_identity:
            raise ValueError("artifact changed while it was snapshotted")
        return raw
    finally:
        os.close(descriptor)


def write_exclusive(path, raw):
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_CLOEXEC", 0)
    descriptor = os.open(path, flags, 0o600)
    try:
        view = memoryview(raw)
        while view:
            view = view[os.write(descriptor, view) :]
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def derive(raw, expected_target, identifier, artifact_relative):
    artifact = load_closed_json(raw)
    abi = artifact.get("abi")
    if not isinstance(abi, list):
        raise ValueError("artifact ABI is not an array")
    metadata_text = artifact.get("rawMetadata")
    if not isinstance(metadata_text, str):
        raise ValueError("artifact metadata is missing")
    metadata = load_closed_json(metadata_text)
    targets = metadata["settings"]["compilationTarget"]
    if not isinstance(targets, dict) or len(targets) != 1:
        raise ValueError("artifact compilation target is ambiguous")
    source, contract = next(iter(targets.items()))
    actual_target = f"{source}:{contract}"
    if actual_target != expected_target:
        raise ValueError("artifact compilation target does not match the acceptance entry")
    source_hash = metadata["sources"][source]["keccak256"]
    compiler_version = metadata["compiler"]["version"]
    if not isinstance(source_hash, str) or not DIGEST.fullmatch(source_hash):
        raise ValueError("artifact source hash is invalid")
    if not isinstance(compiler_version, str) or not COMPILER.fullmatch(compiler_version):
        raise ValueError("artifact compiler version is invalid")
    abi_raw = (json.dumps(abi, sort_keys=True, indent=2, ensure_ascii=False) + "\n").encode()
    record = {
        "id": identifier,
        "artifact": artifact_relative,
        "artifact_sha256": "0x" + hashlib.sha256(raw).hexdigest(),
        "abi_sha256": "0x" + hashlib.sha256(abi_raw).hexdigest(),
        "compilation_target": actual_target,
        "source_keccak256": source_hash,
        "compiler_version": compiler_version,
    }
    record_raw = (json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n").encode()
    return abi_raw, record_raw


def main():
    if len(sys.argv) != 8:
        return 2
    root, source, expected_target, identifier, artifact_relative, abi_output, record_output = sys.argv[1:]
    raw = snapshot(root, source)
    abi_raw, record_raw = derive(raw, expected_target, identifier, artifact_relative)
    write_exclusive(abi_output, abi_raw)
    write_exclusive(record_output, record_raw)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (KeyError, OSError, TypeError, ValueError, json.JSONDecodeError):
        raise SystemExit(1)
