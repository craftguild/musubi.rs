#!/usr/bin/env python3
"""Atomically renames one filesystem entry without replacing the destination."""

import ctypes
import os
import sys


def rename_no_replace(source: str, destination: str) -> None:
    if not os.path.isabs(source) or not os.path.isabs(destination):
        raise ValueError("rename paths must be absolute")
    libc = ctypes.CDLL(None, use_errno=True)
    source_bytes = os.fsencode(source)
    destination_bytes = os.fsencode(destination)
    if sys.platform == "darwin":
        rename = libc.renamex_np
        rename.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint]
        rename.restype = ctypes.c_int
        result = rename(source_bytes, destination_bytes, 0x00000004)
    elif sys.platform.startswith("linux"):
        rename = libc.renameat2
        rename.argtypes = [
            ctypes.c_int,
            ctypes.c_char_p,
            ctypes.c_int,
            ctypes.c_char_p,
            ctypes.c_uint,
        ]
        rename.restype = ctypes.c_int
        result = rename(-100, source_bytes, -100, destination_bytes, 0x00000001)
    else:
        raise OSError(f"atomic no-replace rename is unsupported on {sys.platform}")
    if result != 0:
        error = ctypes.get_errno()
        raise OSError(error, os.strerror(error), destination)


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: rename-no-replace.py SOURCE DESTINATION", file=sys.stderr)
        return 2
    try:
        rename_no_replace(sys.argv[1], sys.argv[2])
    except (AttributeError, OSError, ValueError) as error:
        print(f"atomic no-replace rename failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
