"""Validate the shared SDK registry and invoke recorded-generator CLI exact checks for every entry."""

from pathlib import Path
import subprocess
import sys

from generated_registry import registered_roots, validate_registered
from source_inventory import source_paths
from generated_ownership import MARKER


def main():
    root = Path(__file__).resolve().parent.parent
    entries = registered_roots(root)
    sources, _ = source_paths(root)
    markers = {path for path in sources if Path(path).name == MARKER}
    if markers != {f"{output}/{MARKER}" for _, output, _, _ in entries}:
        raise ValueError("SDK registry does not cover the retained SDK markers")
    validate_registered(root, entries)
    for manifest, _, _, _ in entries:
        subprocess.run(["sh", "-eu", "-c",
                        'repo_root=$1; . "$repo_root/scripts/sdk-cli.sh"; sdk_cli --manifest "$1/$2" check',
                        "registered-sdk-check", str(root), manifest], cwd=root, check=True)
    print(f"registered SDK exact checks: {len(entries)}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError, RecursionError, subprocess.SubprocessError) as error:
        print(f"registered SDK check: {error}", file=sys.stderr)
        raise SystemExit(1) from error
