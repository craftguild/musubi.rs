#!/usr/bin/env python3
"""Rust/Python shared ownership grammar and exact retained-byte conformance; no Cargo needed."""

import hashlib
import json
from pathlib import Path
import unittest

from generated_ownership import MARKER, MAX_FILES, MAX_MARKER, parse_marker, validate_files
from ownership_unicode import casefold

ROOT = Path(__file__).resolve().parent.parent


def marker(files):
    return json.dumps({
        "schema_version": "musubi/generated-v2", "layout": "musubi/semantic-layout-v1",
        "files": [{"path": path, "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}
                  for path, raw in sorted(files.items())],
    }, separators=(",", ":")).encode()


class OwnershipTests(unittest.TestCase):
    def test_shared_cases(self):
        cases = json.loads((ROOT / "tests/fixtures/ownership/cases.json").read_text())
        self.assertGreaterEqual(len(cases), 60)
        for case in cases:
            with self.subTest(case=case["name"]):
                raw = case["marker"].encode()
                files = {item["path"]: item["contents"].encode() for item in case["files"]}
                files[MARKER] = raw
                for function, argument, valid in [(parse_marker, raw, case["marker_valid"]), (validate_files, files, case["valid"])]:
                    if valid:
                        function(argument)
                    else:
                        with self.assertRaises((ValueError, UnicodeError)):
                            function(argument)

    def test_hard_marker_and_file_limits(self):
        exact = marker({}).ljust(MAX_MARKER, b" ")
        parse_marker(exact)
        with self.assertRaises(ValueError):
            parse_marker(exact + b" ")
        files = {f"owners/value_{index:05}.rs": b"// owner\n" for index in range(MAX_FILES - 1)}
        files[MARKER] = marker(files)
        validate_files(files)
        files["overflow"] = b""
        with self.assertRaises(ValueError):
            validate_files(files)

    def test_rejects_large_nested_values_and_arrays_before_decoding_them(self):
        for raw in [
            b'{"unknown":' + b'[' * 10000,
            b'{"files":[{"bytes":' + b'[' * 10000,
            b'{"files":[' + b'{"path":"a","bytes":0,"sha256":""},' * MAX_FILES,
        ]:
            with self.assertRaises(ValueError):
                parse_marker(raw)

    def test_unicode_version_does_not_depend_on_python(self):
        self.assertEqual(casefold("Straße"), "strasse")
        self.assertEqual(casefold("\ua7cb"), "\u0264")
        self.assertEqual(casefold("İ"), "i\u0307")


if __name__ == "__main__":
    unittest.main()
