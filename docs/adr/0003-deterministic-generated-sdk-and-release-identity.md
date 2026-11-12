# ADR 0003: Deterministic generated SDK and release identity

## Status

Accepted; amended by ADR0011/0012/0014/0016 and ADR0017. ADR0016 replaces the
original CLI-packaged fixed runtime and release ordering with explicit manifest
Git/Path sources and recorded-producer replay. ADR0020 owns shared CLI build and
delivery preparation. The original text is retained in the hashed private archive.

## Context

Reviewable retained SDKs expose drift that build-time-only generation hides.
Uncontrolled generator/runtime combinations and direct directory overwrite risk
semantic changes, partial output and loss of handwritten files.

## Decision

Generate complete crates for consumer review and commit. Canonical IR/order/
format determines output. Render source/descriptors/provenance incrementally
against the effective output budget, without first constructing an unbounded
intermediate buffer. Stage and validate all files before create-only installation
or marker-owned atomic update. Never replace an unowned directory. Check compares
all added/removed/changed bytes read-only, preserving the validated recorded
producer under ADR0016.

The manifest explicitly selects one runtime source for both core/runtime, distinct
from generator observations; enforce the generated ABI guard. Protect exact
module bytes with rustfmt::skip. Generator and exact check own formatting.
Deployable bytecode uses binary assets and include_bytes; interfaces have no
bytecode/deployment surface. Release JSON records independent input/artifact,
kind/typed-data, generator/runtime and unverified coherence identities.

Snapshot/diff separately classify kind changes, typed-data breaking/additive,
EVM wire breaking/additive, Rust source breaking/additive, deployment changes and
provenance-only differences. AST-only API changes may be Rust-breaking without
wire change; bytecode-only changes affect deployment. Same binary and inputs
render identically; different captured build observations may change provenance.

## Consequences and rejected alternatives

Consumer builds need neither solc nor original artifacts. Generator fixes require
explicit regeneration and review. The original requirement to release a runtime
before embedding its exact revision in a released CLI was replaced by ADR0016;
no current release-order gate is inferred from it. Build.rs/proc-macro generation
would obscure review, direct overwrite would lose atomicity, and an implicit
runtime semver range would weaken explicit source identity.
