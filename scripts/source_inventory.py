"""Bounded no-follow source reads and fail-closed Git enumeration for repository gates."""

import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import time

from ownership_io import identity, open_directory, open_regular, read_open_file
from ownership_unicode import casefold

MAX_SOURCE_BYTES = 256 * 1024 * 1024
MAX_GIT_BYTES = 32 * 1024 * 1024
MAX_SOURCE_FILES = 100_000


def normal_path(value):
    if type(value) is not str or not value:
        raise ValueError("repository path must be nonempty UTF-8 text")
    value.encode("utf-8", errors="strict")
    parts = value.split("/")
    if ("\\" in value or len(parts) > 65
            or any(part in ("", ".", "..") or len(part.encode("utf-8")) > 255 for part in parts)
            or any(ord(char) < 32 or 0x7F <= ord(char) <= 0x9F for char in value)):
        raise ValueError(f"invalid repository path: {value!r}")
    return value


def read_bytes(root, relative, maximum=MAX_SOURCE_BYTES):
    with open_regular(Path(root) / normal_path(relative)) as source:
        return read_open_file(source, maximum)


def closed_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def read_json(root, relative, maximum=1024 * 1024):
    return json.loads(read_bytes(root, relative, maximum).decode("utf-8"), object_pairs_hook=closed_object)


def git_output(root, arguments, input_bytes=b"", accepted=(0,)):
    """Bound stdin/output, elapsed time and diagnostics; never hide a failed Git process."""
    with tempfile.TemporaryFile() as incoming, tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
        incoming.write(input_bytes)
        incoming.seek(0)
        environment = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
        process = subprocess.Popen(["git", "-C", os.fspath(root), *arguments],
                                   stdin=incoming, stdout=output, stderr=errors, env=environment)
        deadline = time.monotonic() + 30
        try:
            while process.poll() is None:
                if (time.monotonic() >= deadline
                        or os.fstat(output.fileno()).st_size > MAX_GIT_BYTES
                        or os.fstat(errors.fileno()).st_size > MAX_GIT_BYTES):
                    raise ValueError("Git source enumeration exceeded its bound")
                time.sleep(0.01)
        except BaseException:
            process.kill()
            process.wait()
            raise
        if process.returncode not in accepted or os.fstat(output.fileno()).st_size > MAX_GIT_BYTES:
            raise ValueError("Git source enumeration failed or exceeded its bound")
        output.seek(0)
        raw = output.read(MAX_GIT_BYTES + 1)
    if raw and not raw.endswith(b"\0"):
        raise ValueError("Git source enumeration is not NUL terminated")
    paths = raw.split(b"\0")[:-1]
    if len(paths) > MAX_SOURCE_FILES:
        raise ValueError("Git source count exceeds its bound")
    return {normal_path(path.decode("utf-8", errors="strict")) for path in paths}


def git_paths(root, *arguments):
    return git_output(root, ["ls-files", "-z", *arguments])


def scan_source_entries(root):
    """Git omits untracked FIFOs: inspect every nonignored entry with bounded no-follow walking."""
    pending = [""]
    count = 0
    while pending:
        relative = pending.pop()
        descriptor = open_directory(Path(root) / relative)
        try:
            before = os.fstat(descriptor)
            entries = []
            with os.scandir(descriptor) as iterator:
                for entry in iterator:
                    if not relative and entry.name == ".git":
                        continue
                    count += 1
                    if count > MAX_SOURCE_FILES:
                        raise ValueError("source scan entry count exceeds its bound")
                    path = normal_path(f"{relative}/{entry.name}" if relative else entry.name)
                    entries.append((path, entry.name))
            names = b"".join(path.encode("utf-8") + b"\0" for path, _ in entries)
            ignored = git_output(root, ["check-ignore", "--stdin", "-z"], names, (0, 1)) if names else set()
            for path, name in entries:
                if path in ignored:
                    continue
                metadata = os.stat(name, dir_fd=descriptor, follow_symlinks=False)
                if stat.S_ISDIR(metadata.st_mode):
                    pending.append(path)
                elif not stat.S_ISREG(metadata.st_mode):
                    raise ValueError(f"source scan found a nonregular entry: {path}")
            if identity(before) != identity(os.fstat(descriptor)):
                raise ValueError("source directory changed during inspection")
        finally:
            os.close(descriptor)


def source_paths(root):
    scan_source_entries(root)
    paths = git_paths(root, "--cached", "--others", "--exclude-standard")
    deleted = git_paths(root, "--deleted")
    if not paths or not deleted <= paths:
        raise ValueError("Git source inventory is empty or inconsistent")
    # Only Git-confirmed tracked deletions may be absent. Permission/scan failures stay errors.
    for path in deleted:
        try:
            os.lstat(Path(root) / path)
        except FileNotFoundError:
            continue
        raise ValueError("Git reported a deleted path that is present")
    paths -= deleted
    folded = {}
    for path in sorted(paths):
        parts = path.split("/")
        for index in range(1, len(parts) + 1):
            name = "/".join(parts[:index])
            previous = folded.setdefault(casefold(name), name)
            if previous != name:
                raise ValueError("repository source paths collide under casefold")
        metadata = os.lstat(Path(root) / path)
        if not stat.S_ISREG(metadata.st_mode):
            raise ValueError(f"repository source is not a regular file: {path}")
    return paths, deleted
