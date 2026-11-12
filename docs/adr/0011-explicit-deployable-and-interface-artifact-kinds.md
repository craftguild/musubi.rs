# ADR 0011: Explicit Deployable And Interface Artifact Kinds

## Status

Accepted for explicit artifact-kind semantics. Its config-v1/v2 compatibility policy is superseded
by ADR 0014 before the first production release.
ADR 0016 subsequently replaces config-v3 with config-v4 and removes the private-SSH release gate;
the kind-specific ABI/deployment and wire invariants in this ADR remain authoritative.

## Context

`musubi/config-v1` requires creation bytecode for every contract and the renderer consequently
emits constructor, deployment, and creation-artifact APIs for every module. That is correct for the
original deployable-contract boundary, but it cannot represent reviewed Solidity interfaces.
Supplying empty creation bytecode would make a non-deployable interface look like a deployable
artifact and would weaken both the generated API and compatibility evidence.

A generated crate can contain a concrete deployable artifact and multiple
checked interfaces. The distinction must therefore be explicit in input, IR, output,
snapshot, diff, release, and provenance rather than inferred from bytecode contents.

## Decision

### Versioned manifest schema

- The meaning of `musubi/config-v1` is unchanged. Every v1 contract is deployable, requires
  non-empty creation bytecode, and retains the existing generated surface.
- `musubi/config-v2` is a new closed schema. Every `[[contracts]]` entry requires exactly one
  `kind = "deployable"` or `kind = "interface"`. Missing, unknown, or misspelled kinds and unknown
  fields fail closed.
- A v2 deployable requires ABI and non-empty creation bytecode. Runtime bytecode, SourceUnit AST,
  compiler metadata, and storage layout remain optional.
- A v2 interface requires ABI and forbids creation bytecode, runtime bytecode, storage layout, link
  references, and immutable references. A SourceUnit AST and compiler metadata are allowed only as
  enrichment and provenance inputs. Empty bytecode is not an interface marker and is rejected when
  supplied.
- ABI remains the only authority for selectors, topics, error selectors, and ABI encoding. AST
  remains optional enrichment and cannot change wire identity.

The parser represents v1 and v2 as distinct closed wire schemas and normalizes them into an IR
whose artifact kind is always explicit. It does not deserialize both versions into one struct with
version-dependent optional-field interpretation.

### Kind-specific generated surface

- Deployable modules retain creation assets, `ContractArtifact`, `artifact`, `init_code`,
  `deployment`, linking, and runtime-template verification surfaces.
- Interface modules contain canonical ABI, calls, transactions, events, errors, and offline ABI
  value codecs only. They do not contain creation/runtime assets and do not expose
  `ContractArtifact`, creation hashes, constructor, deployment, init-code, linking, or runtime
  verification APIs.
- A multi-contract generated crate may contain both kinds. Shared package/runtime identity is
  emitted once, while each module exposes only its kind-valid API.
- Sparse interface ABIs remain valid. Surface and import reservation are derived only from ABI
  items that exist.
- Compile-fail fixtures prove that an interface module has no deployment or creation-artifact API;
  string inspection alone is not sufficient.

### Compatibility and release evidence

- Artifact kind is included in canonical IR, release manifest, generated Rust provenance,
  compatibility snapshot, diff output, SBOM properties, and package provenance.
- A deployable/interface change is always reported explicitly as `artifact_kind_changed`, is Rust
  source breaking, and changes deployment identity. It is never hidden as provenance-only or as an
  ordinary bytecode addition/removal.
- Snapshot schema v2 stores artifact kind and the exact non-zero creation/runtime template byte
  lengths used to bound every link and immutable range. Reading an old v1 snapshot normalizes its
  contracts to deployable without changing the old snapshot bytes. Because v1 committed only the
  template hashes, a range-free v1 deployment retains an explicit unknown length for comparison;
  any v1 deployment containing a link or immutable range fails closed because its real bound
  cannot be reconstructed. New snapshots use v2 and must always carry exact lengths.
- Deterministic ordering uses `(contract id, artifact kind)` only after unique contract IDs have
  been enforced; kind changes cannot create a second contract with the same ID.
- Release generation continues to require one reviewed canonical SSH locator and exact revision
  for both generator and runtime. Artifact kind does not introduce a development-path fallback.

## Required Verification

- Config-v1 ABI types, calls, events, errors, deployment behavior, and artifact-byte fixtures retain
  their prior behavior and deployable surface. Generated provenance constants plus newly written
  release, snapshot, SBOM, and package provenance evidence record the normalized explicit
  `deployable` kind required by this decision.
- Config-v2 rejects missing/unknown kinds and every forbidden kind/field combination.
- Deployable entries reject empty creation bytecode.
- Interface entries generate calls, transactions, events, errors, overloads, tuples, nested and
  fixed/dynamic arrays, and strict codecs without any deployment surface.
- Mixed-kind crates generate, check, doctor, test, Clippy, and rustdoc standalone.
- Snapshot and diff fixtures cover both directions of a kind change.
- Snapshot mutation fixtures reject zero, oversized, absent, and actual-template-out-of-bounds
  lengths, plus length-free v1 snapshots containing bytecode ranges.
- Release manifest, SBOM, and provenance agree on every contract kind.

## Consequences

Consumer repositories can use real reviewed interface compiler artifacts without manufacturing
bytecode or publishing a misleading deployment API. Config v1 remains a stable compatibility
boundary, while config v2 makes all new artifact intent reviewable.

## Rejected Alternatives

### Infer interface from empty creation bytecode

This preserves an ambiguous schema, makes accidental empty deployables indistinguishable from
interfaces, and encourages the compatibility hack this decision is intended to remove.

### Make creation bytecode optional in config v1

That silently changes an accepted schema and makes old manifests acquire new semantics.

### Permit runtime bytecode or storage layout on interfaces

An interface owns neither deployed code nor storage. Preserving those fields would imply an
artifact identity that the interface cannot establish.
