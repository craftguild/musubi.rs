# ADR 0009: Separate wire shape and owned tuple surface identity

## Status

Accepted.

## Context

Different structs can lower to the same `(bool,int24,int24)` ABI while exposing
should_place versus should_rebalance fields. The old four-byte wire-derived
Tuple name safely rejected these distinct surfaces with MSB-E004, but could not
generate a legitimate full ABI. Optional SourceUnit AST may omit imported types.
Requiring AST, reducing the ABI, trusting internalType or editing output is not
an acceptable workaround.

## Decision

Wire identity uses canonical ABI types only, excluding names/internalType/AST,
and determines selectors/topics/codec and EVM compatibility. Owned identity is a
recursive ordered Rust surface independent of ABI traversal: canonical primitive,
array kind/length/element owner, uniquely verified enrichment, exact ABI field
names, normalized Rust fields and nested owned identities. Exact names also appear
in rustdoc. Missing/unknown/ambiguous AST still has an ABI-surface fallback;
internalType alone cannot establish named identity.

Encode once within resource limits as tagged length-prefixed bytes with
`owned-tuple-surface/v1` separation, never JSON/debug text. Anonymous names are
Tuple plus first16 Keccak-256 bytes in lowercase32 hex. Identical owners share one
type per contract. Short digest or normalized field/function/event/error/enrichment
namespace collisions fail MSB-E004; no counters or traversal-order fallback.
Keep selectors/topics/descriptors/codec bytes unchanged.

## Compatibility and verification

Changed names/enrichment can be Rust-breaking without a wire change. The first
pre-production cutover offers no old anonymous-name shim and alone does not change
the runtime ABI guard. Preserve full reviewed artifact inventories.

Verify same-wire/different-field and verified-enrichment/fallback coexistence,
owner reuse, recursive tuple/array distinction and traversal-independent names/
source. Raw-input provenance differences remain legitimate. Independent selector/
calldata/return goldens, byte differential, real namespace collision rejection
and complete repo-owned ABI-corpus generate/check/doctor stay required.

## Consequences and rejected alternatives

Longer names correctly separate public surface from wire identity and work without
complete imported AST. Merging same-wire structs loses field names; internalType
trust imports unverified identity; reducing the ABI hides drift; sequence suffixes
make unrelated traversal changes rename types. All are rejected.

## 2026-11-09 publication-fixture amendment

The [ADR0017 amendment](0017-semantic-generated-layout-and-owned-line-budget.md#2026-11-09-publication-fixture-amendment)
transfers the full-corpus oracle to repo-owned interface samples. Independent
owned-tuple selector/calldata/return goldens and real deployable fixtures retain
their separate wire and deployment coverage.
