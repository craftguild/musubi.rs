"""Closed, bounded v2 ownership grammar shared by source-only gates and recovery extraction.

This binds retained bytes to their own record; it does not authenticate the generator or replace
the separate exact regeneration gate. Parsing is incremental by closed field, so a 16 MiB marker
cannot allocate an unbounded nested JSON tree or millions of file records before validation.
"""

import hashlib
import json
import re
import sys

from ownership_unicode import casefold

if sys.version_info < (3, 11):
    raise SystemExit("Python 3.11 or later is required; no interpreter is installed automatically")

MARKER = ".musubi-generated"
SCHEMA = "musubi/generated-v2"
LAYOUT = "musubi/semantic-layout-v1"
MAX_MARKER = 16 * 1024 * 1024
MAX_FILES = 10_000
MAX_DIRECTORIES = 10_000
MAX_BYTES = 256 * 1024 * 1024
MAX_DEPTH = 64
MAX_PATH = (MAX_DEPTH + 1) * 256
HASH = re.compile(r"[0-9a-f]{64}\Z")
UNSIGNED = re.compile(r"(?:0|[1-9][0-9]*)")


def validate_path(path, depth=MAX_DEPTH):
    if type(path) is not str:
        raise ValueError("ownership path is not text")
    encoded = path.encode("utf-8", errors="strict")
    parts = path.split("/")
    if (
        not encoded or "\\" in path
        or any(ord(value) < 32 or 0x7F <= ord(value) <= 0x9F for value in path)
        or any(part in ("", ".", "..") or len(part.encode("utf-8")) > 255 for part in parts)
        or len(parts) > min(depth, MAX_DEPTH) + 1
        or casefold(parts[0]) in ("cargo.lock", "target", ".cargo", "cache")
    ):
        raise ValueError("ownership path is invalid or exceeds its bound")
    return parts


class Paths:
    """Tracks exact spelling and kind for ancestors as well as leaves, with a directory budget."""

    def __init__(self, maximum=MAX_DIRECTORIES):
        self.entries = {}
        self.directories = set()
        self.maximum = min(maximum, MAX_DIRECTORIES)

    def insert(self, path, directory=False):
        parts = validate_path(path)
        for index in range(1, len(parts)):
            self._entry("/".join(parts[:index]), True)
        self._entry(path, directory)

    def _entry(self, path, directory):
        folded = casefold(path)
        previous = self.entries.get(folded)
        if previous is not None:
            if previous != (path, True) or not directory:
                raise ValueError("ownership paths collide")
            return
        if directory:
            if len(self.directories) >= self.maximum:
                raise ValueError("ownership directory count exceeds its bound")
            self.directories.add(path)
        self.entries[folded] = (path, directory)


class MarkerParser:
    def __init__(self, raw, maximum_files):
        self.text = raw.decode("utf-8", errors="strict")
        self.cursor = 0
        self.decoder = json.JSONDecoder()
        self.maximum_files = maximum_files

    def whitespace(self):
        while self.cursor < len(self.text) and self.text[self.cursor] in " \n\r\t":
            self.cursor += 1

    def take(self, character):
        self.whitespace()
        if self.text[self.cursor:self.cursor + 1] == character:
            self.cursor += 1
            return True
        return False

    def require(self, character):
        if not self.take(character):
            raise ValueError("ownership JSON structure is invalid")

    def string(self):
        self.whitespace()
        if self.text[self.cursor:self.cursor + 1] != '"':
            raise ValueError("ownership field must be a string")
        value, self.cursor = self.decoder.raw_decode(self.text, self.cursor)
        return value

    def integer(self):
        self.whitespace()
        match = UNSIGNED.match(self.text, self.cursor)
        if match is None or len(match[0]) > 20:
            raise ValueError("ownership length is not an unsigned integer")
        self.cursor = match.end()
        value = int(match[0])
        if value > (1 << 64) - 1:
            raise ValueError("ownership length exceeds u64")
        return value

    def object(self, fields):
        self.require("{")
        result = {}
        if not self.take("}"):
            while True:
                key = self.string()
                if key not in fields or key in result:
                    raise ValueError("ownership object has an unknown or duplicate field")
                self.require(":")
                result[key] = fields[key]()
                if self.take("}"):
                    break
                self.require(",")
        if result.keys() != fields.keys():
            raise ValueError("ownership object has missing fields")
        return result

    def records(self):
        self.require("[")
        records = []
        if not self.take("]"):
            while True:
                if len(records) >= self.maximum_files - 1:
                    raise ValueError("ownership file count exceeds its bound")
                records.append(self.object({"path": self.string, "bytes": self.integer, "sha256": self.string}))
                if self.take("]"):
                    break
                self.require(",")
        return records

    def parse(self):
        marker = self.object({"schema_version": self.string, "layout": self.string, "files": self.records})
        self.whitespace()
        if self.cursor != len(self.text):
            raise ValueError("ownership marker has trailing bytes")
        return marker


def parse_marker(raw, maximum_files=MAX_FILES, maximum_bytes=MAX_BYTES, depth=MAX_DEPTH):
    maximum_files = min(maximum_files, MAX_FILES)
    maximum_bytes = min(maximum_bytes, MAX_BYTES)
    if len(raw) > min(MAX_MARKER, maximum_bytes) or maximum_files < 1:
        raise ValueError("ownership marker exceeds its bound")
    marker = MarkerParser(raw, maximum_files).parse()
    if marker["schema_version"] != SCHEMA or marker["layout"] != LAYOUT:
        raise ValueError("unsupported ownership schema or layout")
    previous = None
    total = len(raw)
    paths = Paths(maximum_files)
    paths.insert(MARKER)
    for record in marker["files"]:
        path = record["path"]
        validate_path(path, depth)
        if path == MARKER or (previous is not None and previous >= path) or not HASH.fullmatch(record["sha256"]):
            raise ValueError("ownership record order, self-reference, or hash is invalid")
        paths.insert(path)
        total += record["bytes"]
        if total > maximum_bytes:
            raise ValueError("ownership aggregate bytes exceed their bound")
        previous = path
    return marker


def validate_files(files, maximum_files=MAX_FILES, maximum_bytes=MAX_BYTES, depth=MAX_DEPTH):
    if len(files) > min(maximum_files, MAX_FILES) or MARKER not in files:
        raise ValueError("ownership file inventory is invalid")
    for path in files:
        validate_path(path, depth)
    marker = parse_marker(files[MARKER], maximum_files, maximum_bytes, depth)
    if files.keys() != {MARKER, *(record["path"] for record in marker["files"])}:
        raise ValueError("ownership file set differs")
    for record in marker["files"]:
        contents = files[record["path"]]
        if len(contents) != record["bytes"] or hashlib.sha256(contents).hexdigest() != record["sha256"]:
            raise ValueError("ownership file bytes differ")
    return marker
