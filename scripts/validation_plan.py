"""Source-only audit that the final checkpoint reaches ownership, exact generation and regressions."""

from pathlib import Path
import sys

from generated_registry import registered_roots
from source_inventory import read_bytes

REQUIRED = (
    'scripts/python.sh scripts/validation_plan.py',
    'scripts/check-lines.sh',
    'scripts/check-generated.sh',
    'scripts/check-safe-generated.sh "$repo_root"',
    'scripts/test-check-lines.sh',
    'scripts/python.sh scripts/test-validation-plan.py',
    'scripts/python.sh scripts/test-generated-ownership.py',
    'scripts/python.sh scripts/test-ownership-io.py',
    'scripts/python.sh scripts/test-sdk-recovery.py',
    'scripts/test-sdk-fixtures.sh',
    'scripts/python.sh scripts/test-large-semantic-owners.py',
    'scripts/python.sh scripts/test-canonical-abi-budget.py',
    'scripts/test-abi-corpus.sh',
    'scripts/test-event-rebalance-acceptance.sh',
    'scripts/test-typed-data-array-acceptance.sh',
    'scripts/test-typed-data-edge-acceptance.sh',
    'scripts/test-safe-source-provenance.sh',
    'cargo test --locked --workspace "$@"',
    'cargo test --locked -p musubi --all-features "$@"',
    'git diff --check',
    'git diff --cached --check',
)


def validate(root):
    lines = read_bytes(root, "scripts/validate.sh", 64 * 1024).decode("utf-8").splitlines()
    for command in REQUIRED:
        if lines.count(command) != 1:
            raise ValueError(f"final checkpoint must invoke exactly once: {command}")
    if not any(line.startswith("set ") and "e" in line[4:] for line in lines):
        raise ValueError("final checkpoint does not propagate command failures")
    registered_roots(root)  # An empty/missing registry cannot turn an exact gate into zero checks.
    print(f"final checkpoint reachability: {len(REQUIRED)} mandatory commands")


if __name__ == "__main__":
    try:
        validate(Path(__file__).resolve().parent.parent)
    except (OSError, ValueError, RuntimeError, RecursionError) as error:
        print(f"final checkpoint: {error}", file=sys.stderr)
        raise SystemExit(1) from error
