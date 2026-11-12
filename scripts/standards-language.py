#!/usr/bin/env python3
"""Run the product's ordinary language gates using the shared operation context."""

import ast
import os
from pathlib import Path
import subprocess
import sys

from source_inventory import read_bytes, source_paths

ROOT = Path(__file__).resolve().parent.parent


def run(argv, environment=None):
    subprocess.run(argv, cwd=ROOT, env=environment, check=True, timeout=1800)


def cargo(operation, *arguments):
    toolchain = os.environ.get("CRAFTGUILD_TOOLCHAIN") or "1.95.0"
    profile = os.environ.get("CRAFTGUILD_PROFILE", "debug")
    if profile not in ("debug", "release"):
        raise ValueError("Unsupported shared operation profile")
    environment = dict(os.environ, CARGO_TARGET_DIR=str(ROOT / "target/product-tests"),
                       RUSTUP_AUTO_INSTALL="0")
    if operation == "doc":
        environment["RUSTDOCFLAGS"] = (environment.get("RUSTDOCFLAGS", "") + " -D warnings").strip()
    command = ["cargo", f"+{toolchain}", operation, "--locked", "--workspace", *arguments]
    if profile == "release":
        command.insert(4, "--release")
    run(command, environment)


def main(action):
    if action == "format":
        toolchain = os.environ.get("CRAFTGUILD_TOOLCHAIN") or "1.95.0"
        run(["cargo", f"+{toolchain}", "fmt", "--all", "--", "--check"])
    elif action == "lint":
        cargo("clippy", "--all-targets", "--", "-D", "warnings")
        cargo("doc", "--no-deps")
    elif action == "tooling":
        paths, _ = source_paths(ROOT)
        for name in sorted(paths):
            if name.endswith(".py") and not name.startswith("standards/"):
                ast.parse(read_bytes(ROOT, name).decode(), filename=name)
            elif name.endswith(".sh") and not name.startswith("standards/"):
                shell = "bash" if b"bash" in read_bytes(ROOT, name).splitlines()[0] else "sh"
                run([shell, "-n", name])
    else:
        raise ValueError("Expected format, lint or tooling")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Expected one language gate")
    main(sys.argv[1])
