# ADR 0007: Secure host operation boundary

## Status

Superseded by [ADR0013](0013-handle-bound-artifact-io.md). Historical reasoning is
preserved; supported current operating systems are Linux/macOS under ADR0020.

## Context and original decision

Checking an entry and reopening its path cannot bind reads/renames to the inspected
object. Cargo validation must also stop descendants retaining pipes after the
direct child exits. Standard-library primitives could not provide this uniformly
without forbidden first-party unsafe code.

The original config-v1 generation/check/doctor and staged Cargo operations required
Unix retained directory handles, safe no-follow wrappers and native process groups.
Capability preflight preceded all writes, regardless of existing output or Cargo
validation selection. Missing primitives failed with stable diagnostics, without
path-based traversal or direct-child-only cleanup fallback. Pure manifest/IR/render/
snapshot/diff library boundaries stayed platform-independent.

## Consequences and rejected alternatives

Unix inspection and cleanup avoided path swaps and external signal commands.
Unsupported hosts failed before mutation. Any future equivalent support would
need safe native directory/process-tree primitives and an accepted decision.
Metadata-then-reopen and child-only timeout handling were rejected because they
could not preserve object identity and deadlines. This historical possibility
adds no Windows support plan or CI requirement.
