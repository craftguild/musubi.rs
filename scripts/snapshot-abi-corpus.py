#!/usr/bin/env python3
"""Snapshots a ABI corpus through retained directory and file handles."""

import json
import os
import stat
import sys
import time

MAXIMUM_FILE_BYTES = 64 * 1024 * 1024
MAXIMUM_TOTAL_BYTES = 128 * 1024 * 1024
MAXIMUM_ABI_FILES = 64


def reject_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON field")
        result[key] = value
    return result


def reject_constant(_value):
    raise ValueError("non-finite JSON number")


def maximum_total_bytes():
    value = os.environ.get("MUSUBI_CORPUS_SNAPSHOT_TEST_MAXIMUM_TOTAL_BYTES")
    if value is None:
        return MAXIMUM_TOTAL_BYTES
    if not value.isascii() or not value.isdecimal() or value.startswith("0"):
        raise ValueError("invalid test aggregate byte limit")
    limit = int(value)
    if limit > MAXIMUM_TOTAL_BYTES:
        raise ValueError("test aggregate byte limit cannot raise the production limit")
    return limit


def validate_closed_json(raw):
    json.loads(
        raw,
        object_pairs_hook=reject_duplicates,
        parse_constant=reject_constant,
    )


def wait_for_test_hook(relative):
    target = os.environ.get("MUSUBI_CORPUS_SNAPSHOT_TEST_TARGET")
    ready = os.environ.get("MUSUBI_CORPUS_SNAPSHOT_TEST_READY_FILE")
    resume = os.environ.get("MUSUBI_CORPUS_SNAPSHOT_TEST_CONTINUE_FILE")
    if not target and not ready and not resume:
        return
    if not target or not ready or not resume:
        raise ValueError("incomplete corpus snapshot test hook")
    if target != relative:
        return
    with open(ready, "xb") as marker:
        marker.write(b"ready\n")
    deadline = time.monotonic() + 5
    while not os.path.exists(resume):
        if time.monotonic() >= deadline:
            raise ValueError("corpus snapshot test hook timed out")
        time.sleep(0.01)


def identity(metadata):
    return (
        metadata.st_dev,
        metadata.st_ino,
        metadata.st_mode,
        metadata.st_size,
        metadata.st_mtime_ns,
    )


def read_regular(parent, name, relative, remaining_total):
    if not name or name in (".", "..") or "/" in name or "\0" in name:
        raise ValueError("corpus entry name is not canonical")
    flags = os.O_RDONLY | os.O_NONBLOCK
    flags |= getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(name, flags, dir_fd=parent)
    with os.fdopen(descriptor, "rb") as source:
        before = os.fstat(source.fileno())
        if (
            not stat.S_ISREG(before.st_mode)
            or before.st_size > MAXIMUM_FILE_BYTES
            or before.st_size > remaining_total
        ):
            raise ValueError("corpus input is not a bounded regular file")
        wait_for_test_hook(relative)
        raw = source.read(min(MAXIMUM_FILE_BYTES, remaining_total) + 1)
        after = os.fstat(source.fileno())
    if len(raw) > MAXIMUM_FILE_BYTES or len(raw) != before.st_size:
        raise ValueError("corpus input changed size while being read")
    if identity(before) != identity(after):
        raise ValueError("corpus input changed while being read")
    return raw


def write_regular(parent, name, raw):
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(name, flags, 0o600, dir_fd=parent)
    with os.fdopen(descriptor, "wb") as target:
        target.write(raw)


def snapshot(source_path, output_path):
    directory_flags = (
        os.O_RDONLY
        | os.O_NONBLOCK
        | os.O_DIRECTORY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    source = os.open(source_path, directory_flags)
    try:
        root = source
        files = {}
        total = 0
        total_limit = maximum_total_bytes()
        for name in (
            ".musubi-abi-corpus",
            "musubi.toml",
            "inventory.json",
        ):
            raw = read_regular(root, name, name, total_limit - total)
            total += len(raw)
            files[name] = raw
        validate_closed_json(files["inventory.json"])

        abi = os.open("abi", directory_flags, dir_fd=root)
        try:
            names = []
            with os.scandir(abi) as entries:
                for entry in entries:
                    if len(names) == MAXIMUM_ABI_FILES:
                        raise ValueError("ABI entry count is invalid")
                    names.append(entry.name)
            names.sort()
            if not names:
                raise ValueError("ABI entry count is invalid")
            abi_files = {}
            for name in names:
                if not name.endswith(".json"):
                    raise ValueError("ABI entry name is invalid")
                relative = f"abi/{name}"
                raw = read_regular(abi, name, relative, total_limit - total)
                total += len(raw)
                validate_closed_json(raw)
                abi_files[name] = raw
        finally:
            os.close(abi)
    finally:
        os.close(source)

    os.mkdir(output_path, 0o700)
    output = os.open(output_path, directory_flags)
    try:
        for name, raw in files.items():
            write_regular(output, name, raw)
        os.mkdir("abi", 0o700, dir_fd=output)
        output_abi = os.open("abi", directory_flags, dir_fd=output)
        try:
            for name, raw in abi_files.items():
                write_regular(output_abi, name, raw)
        finally:
            os.close(output_abi)
    finally:
        os.close(output)


def main():
    if len(sys.argv) != 3:
        raise ValueError(
            "usage: snapshot-abi-corpus.py <source> <new-output>"
        )
    snapshot(sys.argv[1], sys.argv[2])


if __name__ == "__main__":
    try:
        main()
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError) as error:
        print(f"invalid ABI acceptance corpus: {error}", file=sys.stderr)
        sys.exit(1)
