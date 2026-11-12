#!/bin/sh
# Ordinary product tests; make check adds the common format, lint and quality gates.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$repo_root"
export RUSTUP_TOOLCHAIN=${CRAFTGUILD_TOOLCHAIN:-1.95.0}
export RUSTUP_AUTO_INSTALL=0
export CARGO_TARGET_DIR="$repo_root/target/product-tests"
case "${CRAFTGUILD_PROFILE:-debug}" in
  debug) set -- ;;
  release) set -- --release ;;
  *) echo 'Unsupported shared test profile' >&2; exit 2 ;;
esac
scripts/python.sh scripts/validation_plan.py

scripts/check-lines.sh
cargo test --locked --workspace "$@"
cargo test --locked -p musubi --all-features "$@"
scripts/check-generated.sh
scripts/check-safe-generated.sh "$repo_root"
scripts/check-manifest-only-surface.sh
scripts/test-consumer-resolution.sh
scripts/test-generated-rustfmt-boundary.sh
scripts/test-sdk-fixtures.sh
scripts/python.sh scripts/test-large-semantic-owners.py
scripts/python.sh scripts/test-canonical-abi-budget.py
scripts/test-abi-corpus.sh
scripts/test-abi-corpus-manifest.sh
scripts/python.sh scripts/test-sdk-recovery.py
scripts/python.sh scripts/test-generated-ownership.py
scripts/python.sh scripts/test-ownership-io.py
scripts/python.sh scripts/test-validation-plan.py
scripts/test-event-rebalance-acceptance.sh
scripts/test-typed-data-array-acceptance.sh
scripts/test-typed-data-edge-acceptance.sh
cmp safe-v141-sdk-input/safe-v1.4.1.typed-data-schema.json release-fixture/typed-data-schema.json
cmp safe-v141-sdk-input/safe-v1.4.1.typed-data-vectors.json release-fixture/typed-data-vectors.json
scripts/test-compiler-artifact-snapshot.sh
scripts/test-update-transaction.sh
scripts/test-safe-source-provenance.sh
scripts/test-safe-generated.sh
scripts/test-check-lines.sh
git diff --check
git diff --cached --check
