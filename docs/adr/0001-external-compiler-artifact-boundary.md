# ADR 0001: External compiler artifact boundary

## Status

Accepted; amended by ADR0011 for interface artifacts, ADR0014 for manifest-only
configuration and ADR0016 for the current config-v4 schema.

## Context

Proving that ABI and bytecode share compiler provenance would require the
compiler's semantics, optimization, EVM settings, metadata and linking behavior.
Owning that inside Musubi would broaden both implementation and review scope.
Silently presenting arbitrary paired files as verified would also be misleading.

## Decision

Musubi neither implements nor invokes a Solidity compiler. Deployable contracts
require ABI and creation bytecode; interfaces require ABI and reject bytecode.
Optional runtime bytecode, AST, metadata, storage layout and references follow the
artifact-kind contract. ABI alone defines selectors, topics and encoding. AST is
limited to uniquely verified enum/UDVT/struct enrichment.

Validate syntax, size/resource bounds, selector/topic/error collisions, placeholder
and link consistency, underlying ABI/enrichment agreement, canonical digests,
deterministic output and optional runtime template descriptors. These checks do
not establish common compilation provenance. Emit informational
`MSB-I001 EXTERNAL_ARTIFACT_COHERENCE_UNVERIFIED` during generation/check/doctor
and record the unverified state in provenance. Warning denial does not promote
this notice; missing AST remains a separate warning.

## Consequences and rejected alternatives

Compiler selection, artifact publication and coherence attestation stay with the
external pipeline. Malicious or accidentally mismatched artifacts may pass
internal consistency checks. Parsing source syntax alone would not prove optimized
bytecode; invoking solc would violate the tool boundary; omitting the notice would
misstate the guarantee. Future external attestations must remain distinct from
Musubi compiler verification.
