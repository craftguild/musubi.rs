# ADR 0012: Closed Typed-Data Authority

## Status

Accepted.

## Context

ABI describes contract calldata, return values, events, errors, and tuple layout. It does not
preserve the EIP-712 type graph, primary type, struct names, field names, field order, dependency
ordering, or domain type. Reconstructing those semantics from ABI or unverified `internalType`
would create a different and unauditable signing boundary.

Applications may use inline `sol!` structs for typed-data hashes, while Safe
1.4.1 requires version-specific hashing. Replacing those handwritten bindings
requires an explicit authority independent from ordinary ABI generation, checked
against Solidity-produced evidence.

## Decision

### Authority and schema

- Config v2 may contain `[[typed_data]]` entries. Each entry requires a unique `id`, a unique Rust
  module, `authority = "closed-schema-v1"`, a manifest-relative closed schema file, and a
  manifest-relative golden-vector bundle.
- The schema identifier is `musubi/typed-data-schema-v1`. It declares named struct types in source
  field order, one or more root types, and an explicit named domain type for roots that expose a
  final digest. ABI, `internalType`, and AST are not typed-data authority.
- Atomic types are `address`, `bool`, `string`, `bytes`, `bytes1` through `bytes32`, and explicit
  `uint8` through `uint256` or `int8` through `int256` in eight-bit increments. Named structs and
  fixed/dynamic arrays of supported types are allowed. Aliases, tuples, functions, mappings,
  fixed/ufixed, implicit `uint`/`int`, unresolved references, recursive cycles, zero-length fixed
  arrays, typed-data fixed-array type paths whose cumulative element product exceeds the versioned
  limit, and
  unknown fields fail closed.
- Type and field names must be valid Solidity identifiers. Duplicate names, normalized Rust name
  collisions, duplicate roots, unused unreachable types, and ambiguous module collisions with
  contract modules fail closed.
- `encodeType` is built from the declared primary type followed by its transitively referenced
  struct types in lexicographic type-name order. Field order within every struct is preserved.
  The schema never accepts a precomputed type string or type hash as authority.

### Mandatory Solidity-linked golden evidence

- Every typed-data entry requires an `musubi/typed-data-vectors-v1` bundle with at least one
  positive vector per root and per declared domain shape.
- A vector records typed owned values and the independently obtained canonical type string, type
  hash, struct hash, domain separator when applicable, and final `\x19\x01` digest when applicable.
  Lab02 recomputes every stage and rejects any mismatch.
- The bundle also records the reviewed Solidity source path and SHA-256, Foundry artifact target,
  artifact SHA-256, compiler version/settings digest, and vector-generation command identity.
  These are unverified external provenance, not a claim that lab02 compiled Solidity.
- Source paths are normalized relative paths. Artifact targets use the exact relative
  `source-path:Contract` form; absolute paths, traversal, platform separators, controls, and
  malformed Solidity contract identifiers fail closed before release evidence is built.
- Compiler identity is a bounded canonical `major.minor.patch` value, optionally prefixed by
  `solc ` and optionally suffixed by lowercase `+commit.` plus eight hexadecimal digits. Paths,
  controls, arbitrary build banners, and credential-bearing text are not accepted as versions.
- Consumer extraction must obtain compiler evidence and typed-data provenance from the same
  reviewed build and must run the Solidity golden harness before updating vectors. Lab02 preserves
  and hashes the evidence but does not run Foundry or elevate external coherence above
  `unverified`.
- Release manifest, snapshot, SBOM, and provenance include schema digest, vector digest, roots,
  canonical type strings/type hashes, the normalized artifact target, and external Solidity
  evidence digests. Public generated-crate inspection parses every root/domain type string as a
  bounded closed EIP-712 graph, reconstructs canonical `encodeType`, and revalidates Solidity
  identifiers and Keccak type hashes before packaging copies that evidence into SBOM or
  provenance. Truncated definitions, trailing text, aliases, unresolved/unreachable dependencies,
  noncanonical dependency order, and self-consistent hashes over malformed text fail closed. Raw
  source, absolute paths, and raw artifact bodies are not emitted.
- Validation prepares one bounded named-type index and memoizes type hashes across all roots and
  vectors. Recursive struct/array hashing streams words into Keccak, and generated source is
  accumulated against the effective output-byte limit so repeated shared DAGs cannot bypass the
  resource model before Cargo subprocess bounds apply.

### Generated owned API

For each declared type, generated source provides an owned Rust struct and:

- `TYPE_STRING` and `type_hash` for roots and referenced types;
- strict `struct_hash` with recursive struct and array hashing;
- dynamic `string` and `bytes` hashing and canonical 32-byte atomic encoding;
- strict signed/unsigned width construction using lab02-owned integer types.

For every declared domain/root pair it additionally provides the owned domain type,
`domain_separator`, and final typed-data `digest`. Generated public APIs contain no Alloy or
`sol!` types. The implementation may use a private codec internally.

ABI tuple/value encoding remains a separate ABI-authoritative surface. Generated argument/value
types may expose selector-free `abi_encode` and strict `abi_decode` methods derived from ABI
descriptors. Those methods must not call, share authority with, or infer EIP-712 semantics.

### Compatibility

- Typed-data schema and roots are included in snapshot schema v2.
- Changes to type name, field name, field order, field type, dependency graph, root, or domain are
  `typed_data_breaking` unless they add a new unreachable-independent root/type without changing an
  existing root. Additions are `typed_data_additive` only when all existing canonical strings and
  hashes remain byte-identical.
- Vector/provenance changes with identical schema and computed outputs are provenance-only.
- Typed-data compatibility is reported independently from EVM ABI wire compatibility.

### Safe 1.4.1

The Safe adapter consumes the same typed-data engine and a reviewed Safe schema/vector bundle.
Safe call/event/error encoding is generated from a checked ABI artifact. Handwritten type strings,
selector strings, and parallel ABI encoders are removed after differential vectors prove identical
SafeTx hashes, `approveHash`, `getTransactionHash`, `execTransaction`, and execution events.
Safe-specific owner ordering, signature packing, threshold, EIP-1271, approved-hash, and `eth_sign`
policy remains in `musubi-safe`.

## Required Verification

- Solidity and Rust share golden vectors for canonical strings, type hashes, nested/array struct
  hashes, domain separators, and final digests.
- Negative tests cover wrong field/type/dependency order, cross-chain and verifying-contract
  mismatch, malformed/narrow integers, array order/length, missing/extra values, and schema cycles.
- ABI-value encoding and typed-data hashing tests prove their outputs differ where the standards
  require and cannot be invoked through the wrong API.
- Generated typed-data crates pass standalone check, test, Clippy, and rustdoc without Alloy in the
  public surface.
- Snapshot/diff and release evidence cover typed-data additions, breakage, and provenance-only
  vector refreshes.
- Mutated root/domain names, type strings, or type hashes fail public generated-crate inspection
  and the package gate.

## Consequences

EIP-712 signing semantics become an explicit reviewed input rather than an incidental feature of
ABI generation. Consumers can remove inline `sol!` hashing without replacing it with another
handwritten encoder, and Safe can share the generic wire-correct engine while retaining its
version-specific security policy.

## Rejected Alternatives

### Infer typed data from ABI or `internalType`

Neither contains the complete EIP-712 authority and both permit silent field/type-graph drift.

### Treat a canonical type string as the only input

A string alone does not provide a closed value schema, collision-safe Rust surface, or structured
compatibility diff.

### Keep Safe hashing as the generic model

Safe's domain and signature rules are version-specific and do not cover other application roots.
