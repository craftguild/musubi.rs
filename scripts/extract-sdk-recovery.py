#!/usr/bin/env python3
"""Extract reviewed recovery-v1 file bytes into a new directory, without overwriting an SDK.

The archive is only a bounded recovery snapshot, not an authenticity attestation. Review the
recovered SDK before manually moving any live output. A failed extraction may leave a partial
new directory, but never replaces an existing destination or follows an entry symlink.
"""

import os
import stat
import struct
import sys

from generated_ownership import (MARKER, MAX_FILES, MAX_DIRECTORIES, MAX_BYTES, MAX_PATH,
                                 MAX_MARKER, Paths, validate_files, validate_path)
from ownership_io import DIRECTORY, identity, open_directory, open_regular

MAGIC = b"musubi/recovery-v1\n"


def exact(source, length):
    raw = source.read(length)
    if len(raw) != length:
        raise ValueError("truncated recovery evidence")
    return raw


def load(path):
    with open_regular(path) as source:
        before = os.fstat(source.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_size > MAX_BYTES + MAX_FILES * (MAX_PATH + 12) + len(MAGIC) + 8:
            raise ValueError("recovery input is not a bounded regular file")
        if exact(source, len(MAGIC)) != MAGIC:
            raise ValueError("unsupported recovery schema")
        count = struct.unpack(">Q", exact(source, 8))[0]
        if not 1 <= count <= MAX_FILES:
            raise ValueError("invalid recovery file count")
        files = {}
        paths = Paths()
        total = 0
        for _ in range(count):
            path_length, length = struct.unpack(">IQ", exact(source, 12))
            total += length
            if not 1 <= path_length <= MAX_PATH or total > MAX_BYTES:
                raise ValueError("recovery record exceeds its bound")
            name = exact(source, path_length).decode("utf-8", errors="strict")
            validate_path(name)
            paths.insert(name)
            if name == MARKER and length > MAX_MARKER:
                raise ValueError("recovery marker exceeds its bound")
            files[name] = exact(source, length)
        if source.read(1) or identity(before) != identity(os.fstat(source.fileno())):
            raise ValueError("recovery evidence changed or has trailing bytes")
    validate_files(files)
    return files


def extract(files, destination):
    validate_files(files)
    parent_path, name = os.path.split(os.fsencode(destination))
    if name in (b"", b".", b".."):
        raise ValueError("invalid new extraction destination")
    parent = open_directory(parent_path or b".")
    try:
        os.mkdir(name, 0o700, dir_fd=parent)
        root = os.open(name, DIRECTORY, dir_fd=parent)
    finally:
        os.close(parent)
    try:
        for relative, raw in files.items():
            parts = relative.split("/")
            directory = os.dup(root)
            try:
                for component in parts[:-1]:
                    try:
                        os.mkdir(component, 0o700, dir_fd=directory)
                    except FileExistsError:
                        pass
                    child = os.open(component, DIRECTORY, dir_fd=directory)
                    os.close(directory)
                    directory = child
                descriptor = os.open(parts[-1], os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=directory)
                with os.fdopen(descriptor, "wb") as target:
                    target.write(raw)
                    target.flush()
                    os.fsync(target.fileno())
            finally:
                os.close(directory)
        os.fsync(root)
    finally:
        os.close(root)


if __name__ == "__main__":
    try:
        if len(sys.argv) != 3 or os.name != "posix":
            raise ValueError("expected a recovery file and new destination on a supported Unix host")
        extract(load(sys.argv[1]), sys.argv[2])
    except (OSError, ValueError, OverflowError, struct.error):
        print("SDK recovery extraction failed; existing destinations were not replaced", file=sys.stderr)
        sys.exit(1)
