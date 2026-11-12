"""Pinned Unicode 16.0 full default casefold, independent of the Python runtime's Unicode version."""

from functools import cache
import hashlib
import os
from pathlib import Path
import stat

TABLE_BYTES = 86092
TABLE_SHA256 = "6f1f9c588eb4a5c718d9e8f93b782685e5c7fec872cf05e8e6878053599e09bb"


@cache
def mappings():
    path = Path(__file__).parent / "vendor/unicode-16.0.0/CaseFolding.txt"
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as source:
        before = os.fstat(source.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_size != TABLE_BYTES:
            raise ValueError("Unicode ownership table is invalid")
        data = source.read(TABLE_BYTES + 1)
        after = os.fstat(source.fileno())
        if before.st_size != after.st_size or before.st_mtime_ns != after.st_mtime_ns or before.st_ctime_ns != after.st_ctime_ns:
            raise ValueError("Unicode ownership table changed")
    if len(data) != TABLE_BYTES or hashlib.sha256(data).hexdigest() != TABLE_SHA256:
        raise ValueError("Unicode ownership table differs from its reviewed source")
    output = {}
    for line in data.decode("utf-8").splitlines():
        fields = line.split("#", 1)[0].strip().split(";")
        if len(fields) >= 3 and fields[1].strip() in ("C", "F"):
            output[int(fields[0], 16)] = "".join(chr(int(value, 16)) for value in fields[2].split())
    return output


def casefold(value):
    return value.translate(mappings())
