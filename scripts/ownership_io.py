"""No-follow, metadata-stable, bounded filesystem snapshots for ownership and source gates."""

from contextlib import contextmanager
import os
import stat

from generated_ownership import (
    MARKER, MAX_BYTES, MAX_DEPTH, MAX_FILES, MAX_MARKER, Paths, validate_files, validate_path,
)

DIRECTORY = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
READ = os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW | os.O_CLOEXEC


def identity(metadata):
    return (metadata.st_dev, metadata.st_ino, metadata.st_mode, metadata.st_size,
            metadata.st_mtime_ns, metadata.st_ctime_ns)


def open_directory(path):
    """Traverse every root component without implicitly canonicalizing a symlink alias."""
    path = os.fspath(path)
    if isinstance(path, bytes):
        path = path.decode("utf-8", errors="strict")
    path.encode("utf-8", errors="strict")
    descriptor = os.open("/" if path.startswith("/") else ".", DIRECTORY)
    try:
        for component in path.split("/"):
            if component in ("", "."):
                continue
            child = os.open(component, DIRECTORY, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = child
        return descriptor
    except BaseException:
        os.close(descriptor)
        raise


@contextmanager
def open_regular(path):
    parent, name = os.path.split(os.fspath(path))
    descriptor = open_directory(parent or ".")
    try:
        file_descriptor = os.open(name, READ, dir_fd=descriptor)
    finally:
        os.close(descriptor)
    with os.fdopen(file_descriptor, "rb") as source:
        if not stat.S_ISREG(os.fstat(source.fileno()).st_mode):
            raise ValueError("input is not a regular file")
        yield source


def read_open_file(source, maximum, before=None):
    before = os.fstat(source.fileno()) if before is None else before
    if not stat.S_ISREG(before.st_mode) or not 0 <= before.st_size <= maximum:
        raise ValueError("input is not a bounded regular file")
    raw = source.read(before.st_size + 1)
    if len(raw) != before.st_size or identity(before) != identity(os.fstat(source.fileno())):
        raise ValueError("input changed during bounded inspection")
    return raw


class Directory:
    """An open traversal frame; depth-first walking retains at most depth+1 directory handles."""

    def __init__(self, descriptor, relative):
        self.descriptor = descriptor
        self.relative = relative
        self.entries = None
        try:
            self.before = os.fstat(descriptor)
            self.entries = os.scandir(descriptor)
        except BaseException:
            self.close()
            raise
        self.count = 0

    def close(self):
        if self.entries is not None:
            self.entries.close()
        os.close(self.descriptor)


def read_tree(root, maximum_files=MAX_FILES, maximum_bytes=MAX_BYTES, depth=MAX_DEPTH, after_open=None):
    maximum_files, maximum_bytes, depth = min(maximum_files, MAX_FILES), min(maximum_bytes, MAX_BYTES), min(depth, MAX_DEPTH)
    paths = Paths(maximum_files)
    directories = set()
    files = {}
    total = 0
    frames = [Directory(open_directory(root), "")]
    try:
        while frames:
            frame = frames[-1]
            try:
                entry = next(frame.entries)
            except StopIteration:
                if frame.count == 0 and frame.relative:
                    raise ValueError("ownership tree has an empty directory")
                if identity(frame.before) != identity(os.fstat(frame.descriptor)):
                    raise ValueError("ownership directory changed during inspection")
                frames.pop().close()
                continue
            relative = f"{frame.relative}/{entry.name}" if frame.relative else entry.name
            validate_path(relative, depth)
            frame.count += 1
            descriptor = os.open(entry.name, READ, dir_fd=frame.descriptor)
            try:
                metadata = os.fstat(descriptor)
                if stat.S_ISDIR(metadata.st_mode):
                    if len(frames) > depth:
                        raise ValueError("ownership directory depth exceeds its bound")
                    paths.insert(relative, True)
                    directories.add(relative)
                    retained = descriptor
                    descriptor = None
                    frames.append(Directory(retained, relative))
                elif stat.S_ISREG(metadata.st_mode):
                    if len(files) >= maximum_files:
                        raise ValueError("ownership file count exceeds its bound")
                    paths.insert(relative)
                    maximum = min(maximum_bytes - total, MAX_MARKER if relative == MARKER else MAX_BYTES)
                    with os.fdopen(descriptor, "rb", closefd=False) as source:
                        if after_open is not None:
                            after_open(relative)
                        raw = read_open_file(source, maximum, metadata)
                    total += len(raw)
                    files[relative] = raw
                else:
                    raise ValueError("ownership tree has a special entry")
            finally:
                if descriptor is not None:
                    os.close(descriptor)
        ancestors = {"/".join(path.split("/")[:index]) for path in files for index in range(1, len(path.split("/")))}
        if directories != ancestors:
            raise ValueError("ownership directories differ from file ancestors")
        validate_files(files, maximum_files, maximum_bytes, depth)
        return files
    finally:
        for frame in reversed(frames):
            frame.close()
