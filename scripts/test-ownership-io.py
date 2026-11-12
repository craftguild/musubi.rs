#!/usr/bin/env python3
"""Adversarial no-follow ownership reader tests over explicitly owned temporary trees."""

from contextlib import contextmanager
import os
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest

from generated_ownership import MARKER, MAX_MARKER
from ownership_io import read_tree
from ownership_test_support import owned_files


@contextmanager
def fixture():
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary).resolve()
        files = owned_files({"src/lib.rs": b"//! fixture\n"})
        for path, raw in files.items():
            output = root / path
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_bytes(raw)
        yield root, files


class ReaderTests(unittest.TestCase):
    def test_wide_tree_keeps_file_descriptors_bounded_by_depth(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            files = owned_files({f"owner_{index}/nested/value": b"" for index in range(512)})
            for path, raw in files.items():
                path = root / path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw)
            child = subprocess.run([sys.executable, __file__, "--wide-child", str(root)], timeout=10, check=False)
            self.assertEqual(child.returncode, 0)

    def test_owned_bytes_are_retained_exactly(self):
        with fixture() as (root, files):
            self.assertEqual(read_tree(root), files)

    def test_unknown_empty_special_and_sparse_entries(self):
        for change in ("extra", "empty", "fifo", "symlink", "marker", "non-utf8"):
            with self.subTest(change=change), fixture() as (root, _):
                if change == "extra":
                    (root / "extra").write_bytes(b"x")
                elif change == "empty":
                    (root / "empty").mkdir()
                elif change == "fifo":
                    os.mkfifo(root / "fifo")
                elif change == "symlink":
                    (root / "link").symlink_to(root / "src/lib.rs")
                elif change == "marker":
                    with (root / MARKER).open("wb") as output:
                        output.truncate(MAX_MARKER + 1)
                else:
                    try:
                        with open(os.fsencode(root) + b"/invalid-\xff", "wb") as output:
                            output.write(b"")
                    except UnicodeError:
                        continue  # The OS/Python rejected the invalid path before the reader.
                    except OSError as error:
                        import errno
                        if error.errno == errno.EILSEQ:
                            continue
                        raise
                with self.assertRaises((OSError, ValueError, UnicodeError)):
                    read_tree(root)

    def test_symlink_parents_and_root_are_rejected(self):
        with fixture() as (root, _):
            alias = root / "alias"
            alias.symlink_to(root / "src", target_is_directory=True)
            for path in (alias, alias / "nested"):
                with self.assertRaises((OSError, ValueError)):
                    read_tree(path)

    def test_mutation_between_metadata_and_read_is_detected(self):
        for change in ("same-size", "growth", "swap"):
            with self.subTest(change=change), fixture() as (root, _):
                def mutate(relative):
                    if relative != "src/lib.rs":
                        return
                    path = root / relative
                    if change == "same-size":
                        path.write_bytes(b"//! changed\n")
                    elif change == "growth":
                        path.write_bytes(b"//! fixture\nmore")
                    else:
                        path.unlink()
                        path.symlink_to("never-read")
                with self.assertRaises((OSError, ValueError)):
                    read_tree(root, after_open=mutate)


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--wide-child":
        import resource
        _, hard = resource.getrlimit(resource.RLIMIT_NOFILE)
        resource.setrlimit(resource.RLIMIT_NOFILE, (64, hard))
        assert len(read_tree(sys.argv[2])) == 513
    else:
        unittest.main()
