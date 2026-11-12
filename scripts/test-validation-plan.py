#!/usr/bin/env python3
"""Mutation coverage: removing a mandatory gate or emptying its registry fails before Cargo."""

from contextlib import redirect_stdout
import io
from pathlib import Path
import unittest

from line_test_support import Fixture
from validation_plan import REQUIRED, validate


class ReachabilityTests(unittest.TestCase):
    def test_actual_plan_and_each_removed_gate(self):
        fixture = Fixture()
        self.addCleanup(fixture.close)
        actual = (Path(__file__).resolve().parent / "validate.sh").read_text()
        fixture.write("scripts/validate.sh", actual)
        with redirect_stdout(io.StringIO()):
            validate(fixture.root)
        for command in REQUIRED:
            with self.subTest(command=command):
                fixture.write("scripts/validate.sh", actual.replace(command + "\n", "", 1))
                with self.assertRaises(ValueError):
                    validate(fixture.root)
        fixture.write("scripts/validate.sh", actual)
        fixture.write("scripts/generated-manifests.json", "[]")
        with self.assertRaises(ValueError):
            validate(fixture.root)


if __name__ == "__main__":
    unittest.main()
