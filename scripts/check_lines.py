"""Source-only registered SDK ownership and reviewed artifact validation."""

from pathlib import Path
import sys

from artifact_inventory import validate_artifacts
from generated_ownership import MARKER
from generated_registry import REGISTRY, registered_roots, validate_registered
from source_inventory import read_bytes, source_paths

def check(root):
    sources, deleted = source_paths(root)
    entries = registered_roots(root)
    if REGISTRY not in sources or any(manifest not in sources for manifest, _, _, _ in entries):
        raise ValueError("SDK registry or registered manifest is absent from Git source inventory")
    expected_markers = {f"{output}/{MARKER}" for _, output, _, _ in entries}
    discovered_markers = {path for path in sources if Path(path).name == MARKER}
    if discovered_markers != expected_markers:
        raise ValueError("SDK marker registration differs; discovery never grants an exemption")
    owned = validate_registered(root, entries)
    if not owned.keys() <= sources:
        raise ValueError("registered SDK files are ignored or absent from Git source inventory")
    artifacts = validate_artifacts(root, sources)
    if artifacts & owned.keys():
        raise ValueError("non-SDK artifact inventory overlaps a registered SDK")
    authored = 0
    for path in sorted(sources - owned.keys() - artifacts):
        # Read even non-source data: a symlink parent, special entry, or read error cannot hide here.
        read_bytes(root, path)
        authored += 1
    if not authored:
        raise ValueError("authored source inventory is empty")
    result = {
        "authored_files": authored,
        "generated_rust": sum(path.endswith(".rs") for path in owned),
        "generated_files": len(owned),
        "generated_bytes": sum(map(len, owned.values())),
        "artifacts": len(artifacts),
        "tracked_deletions": len(deleted),
    }
    print("source-only ownership: " + ", ".join(f"{key}={value}" for key, value in result.items()))
    return result


def main():
    if len(sys.argv) != 1:
        raise ValueError("check_lines.py takes no arguments; its repository is its script parent")
    check(Path(__file__).resolve().parent.parent)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError, RecursionError) as error:
        print(f"source-only gate: {error}", file=sys.stderr)
        raise SystemExit(1) from error
