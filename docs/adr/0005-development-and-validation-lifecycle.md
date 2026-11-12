# ADR 0005: Development and validation lifecycle

## Status

Accepted; amended by ADR0014/0016 for manifest-only lifecycle and a single final
aggregate, ADR0017 for semantic ownership, and ADR0020 for implementation-only
Code measurement and shared build/quality execution. Publisher packaging is a
separate operation, not a required local final checkpoint.

## Context

Running all codegen/codec/RPC/Safe suites after every edit slows feedback. Omitting
cross-boundary verification at handoff risks SDK/runtime/actual-EVM drift.
Responsibilities and wire invariants must remain reviewable.

## Decision

Start with focused unit/integration tests, then changed-package format/test/Clippy.
Run the full workspace product/quality aggregate once at a final checkpoint;
repeat only for changed inputs, failures or unresolved concerns. Keep externally
provisioned Anvil checks relevant to adapter changes. Never count a skip as pass.

Use tokei15 Code<=400 for implementation under ADR0020; tests, test-only helpers,
fixtures and design are excluded by actual purpose. Split by responsibility,
never minify. Generated Rust exceptions require complete inventory/hash validation
and canonical isolated reproduction, not a directory name or header.

All crates/modules carry responsibility rustdoc and public APIs useful rustdoc;
public crates warn on missing docs and first-party crates forbid unsafe. Update
fixtures only after focused renderer tests, then immediately exact-check them.
Standalone generated SDK Clippy and whitespace/one-newline EOF checks remain.
Independent empty, fallback/receive, unit-function, event-only and no-custom-error
fixtures expose conditional imports/clients/reexports and strict unknown-revert
behavior without another surface hiding unused code.

`make check` reaches ordinary product tests, format, Clippy, rustdoc, ownership,
exact generation, common Code measurement and staged/unstaged diff checks. Use
`make acceptance` for actual CLI/SBOM/closed metadata/temporary-install evidence.
Do not rerun identical aggregates merely to satisfy two historical command lists.

## Consequences and rejected alternatives

Focused iteration stays short; final checks cover cross-crate drift. More modules
make ownership clearer, and API changes require documentation. Both mandatory full
suites after each edit and package-only final verification are rejected.
