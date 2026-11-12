# ADR 0014: Manifest-Only Consumer CLI And Publisher-Owned Release Boundary

## Status

Accepted. Amends ADR 0003 and ADR 0005, supersedes ADR 0011's pre-release config-v1/v2
compatibility policy, and scopes ADR 0008 to publisher-side release construction and immutable
generated dependency identity.

Partially superseded by [ADR 0016](0016-lightweight-validation-and-generator-build-identity.md).
Manifest-only configuration, no overrides, init/generate/update ownership, and ADR 0015 recovery
remain. Config-v3, binary-owned runtime selection, publisher gates, packaged-only CLI operation,
old evidence versions, and the rejected alternative "Delete the hardened private SSH release pipeline"
below are historical. ADR 0016 now explicitly adopts that deletion, config-v4 runtime selection,
generator build metadata, and recorded-identity replay during exact check. No old package gate must
be completed before implementing or accepting ADR 0016.

## Context

The initial CLI accepts a versioned manifest but still receives the generated output directory and
runtime distribution identity through command-line flags. A source-built CLI may select a
development path, while a released CLI embeds and enforces a private Git identity. The same
`generate` command both creates a new SDK and replaces a marker-owned SDK.

This is safe but makes the ordinary consumer workflow harder to explain and reproduce:

- the manifest is not the complete persistent generation authority;
- a long invocation must be reconstructed correctly by every developer and CI job;
- `generate` does not communicate whether creation or replacement is intended;
- public runtime-selection flags expose publisher and development concerns to consumers;
- private SSH packaging requirements can be mistaken for prerequisites to obtain and execute an
  already reviewed generator binary.

Per-contract CLI flags would make a one-contract example shorter, but create two configuration
authorities. Their meaning also becomes ambiguous for a manifest containing multiple contracts or
typed-data entries. Warning and ignoring such flags would be fail-open because a successful command
could silently omit the caller's requested change.

Lab02 also cannot prove that caller-supplied ABI, bytecode, AST, metadata, and storage layout came
from one honest compiler invocation. That limitation does not become stronger or weaker based on
how the generator binary is distributed.

Lab02 has not reached its first production release. No external stable CLI, manifest, generated
source, release-manifest, or snapshot compatibility promise has been made. Carrying config-v1/v2
parsers and legacy invocation behavior into the first release would turn temporary implementation
history into permanent product surface without protecting an actual production consumer.

## Decision

### One persistent generation authority

- A versioned closed TOML manifest is the only consumer-controlled generation authority.
- Contract, typed-data, package, limit, and output fields cannot be overridden by CLI flags or
  environment variables.
- This rule is identical for zero, one, or multiple contract and typed-data entries. There is no
  single-contract exception.
- The CLI rejects `--abi`, `--bytecode`, `--creation-bytecode`, `--runtime-bytecode`, `--ast`,
  `--metadata`, `--storage-layout`, `--contract-id`, `--out`, `--force`, `--override`, runtime Git,
  and development-runtime flags on `generate`, `update`, `check`, and `doctor`.
- `--manifest` selects the authority file. Diagnostic presentation such as `--format` and
  `--deny-warnings` is operational policy, not a manifest override.
- A report-producing command may continue to receive its report destination, for example
  `snapshot --out`. That path does not alter the generated SDK configuration.

Runtime dependency identity is not consumer configuration. A reviewed generator binary owns one
immutable runtime identity and emits it into generated Cargo dependencies and provenance. Therefore
the reproducible consumer input is the committed manifest plus the verified generator binary
identity/digest, without a self-referential runtime revision inside the lab02 source manifest.

The generator records the manifest schema identifier and exact manifest SHA-256 in generated
release evidence. Input digests, generator/runtime identity, and
`external_artifact_coherence = "unverified"` remain independently recorded.

### One supported config schema

`musubi/config-v3` is the only supported product manifest schema. It includes explicit artifact
kind, optional typed-data authority, and the required generation section:

```toml
schema_version = "musubi/config-v3"

[package]
name = "example-contracts-sdk"
version = "0.3.0"
edition = "2024"

[generation]
output = "generated/example-contracts-sdk"

[[contracts]]
id = "Token"
kind = "interface"
abi = "artifacts/ERC20.abi.json"
```

- `generation.output` is required, manifest-root-relative, and contains only normal path
  components. Absolute paths, empty components, `.`, `..`, platform separators, controls, and
  symlink traversal are rejected.
- A manifest should live at a stable project ancestor of both inputs and output. Moving an existing
  manifest is preferable to weakening output confinement with parent traversal.
- Runtime source fields are forbidden in a consumer manifest. A released binary uses only its
  embedded canonical private SSH URL and exact revision from ADR 0008.
- Source-checkout development and bundled generation remain explicit library/repository-internal
  modes. They are not public runtime flags and cannot be promoted implicitly to released output.
- Unknown fields fail closed.
- Config-v1/v2 parsers, normalization structs, command branches, fixtures whose only purpose is old
  schema acceptance, and documentation are removed. Encountering either identifier is an
  unsupported-schema error and cannot write any output.
- Schema identifiers are not reused with changed meanings. Future format changes require a new
  identifier even though the pre-production v1/v2 readers are deleted.

### Lifecycle commands

The ordinary consumer lifecycle is:

```text
init -> edit/review manifest -> generate -> check -> edit/review manifest -> update -> check
```

The corresponding public command shape is:

```bash
musubi --manifest musubi.toml init
musubi --manifest musubi.toml generate
musubi --manifest musubi.toml check
musubi --manifest musubi.toml update
```

The commands have disjoint mutation authority:

- `init` exclusively creates a config-v3 manifest and never generates an SDK. It refuses an
  existing file, symlink, directory, or special entry and has no force mode. The template is
  clearly marked for editing and contains no fabricated artifact provenance or runtime identity.
  It may contain commented contract templates; `generate` continues to require at least one active
  contract or typed-data entry. The deterministic initial active values are config-v3,
  `generated-contract-sdk`, version `0.1.0`, edition `2024`, and output
  `generated/contract-sdk`; package/output inference from the current directory is not used.
- `generate` creates the output tree only when the destination does not exist. Any existing file,
  directory, symlink, or special entry causes failure without mutation, including an existing
  valid generated SDK.
- `update` replaces only an existing, valid, marker-owned SDK. A missing target, invalid marker,
  foreign file, symlink, or special entry causes failure without mutation. It uses the established
  staging, validation, atomic no-replace rename, backup, rollback, cleanup, and bounded process
  contracts.
- `check` is byte-exact and strictly read-only. It derives the output from the manifest.
- `doctor` validates the manifest and a private temporary generated crate without touching the
  configured output.
- `snapshot`, `diff`, and publisher-internal release validation remain separate governance tools.

`update` means "replace the SDK owned by this generator"; it does not mutate or rewrite the
manifest. There is no generic `--force`, `--override`, or "warning then ignore" path.

### Consumer and publisher distribution boundaries

The product has two explicit roles:

1. The publisher reviews one lab02 revision and runs the existing hardened release construction:
   private SSH source fetch, pinned host keys, isolated Cargo homes, clean/fresh checkout,
   reproducible binary comparison, generated-crate conformance, SBOM, provenance, and checksums.
2. The consumer obtains the already built host binary and its checksum/provenance bundle from a
   reviewed release channel, verifies the selected binary, and runs it against a committed
   config-v3 manifest.

GitHub Actions artifacts may be used for internal release candidates when the workflow run ID,
source revision, target triple, and artifact digest are retained. A production release should use
an immutable retained release asset or artifact registry entry. Merely downloading an unpinned
"latest" artifact is not a reviewed release identity.

The consumer does not need the publisher's deploy key or pinned `known_hosts` merely to execute an
already downloaded binary. The packaged CLI does not refetch its own source. Building the generated
crate still resolves `musubi-core` and `musubi-runtime` through the exact dependency identity
embedded by the binary into the output; ordinary repository access is therefore required. Stronger consumer-side SSH
pinning remains available as deployment policy, but is not implemented as generator configuration.

The publisher-only `validate-release-identity` command may remain hidden for release scripts. It is
not part of the normal consumer lifecycle. Publisher hardening is retained rather than duplicated
inside every consumer invocation.

### External compiler artifact boundary

- ABI remains the sole wire authority; AST remains optional naming/type enrichment.
- Lab02 validates syntax, bounds, kind-specific field combinations, ABI/AST contradictions, codec
  behavior, and deterministic output.
- Lab02 does not run solc/Foundry/Hardhat and does not authenticate common compiler-invocation
  provenance after the fact.
- `MSB-I001` and `external_artifact_coherence = "unverified"` remain visible for contract inputs.
- A producer may publish external attestations, but lab02 does not relabel coherence as verified
  until a future accepted ADR defines a verifier and evidence format.

This limitation is a declared producer/consumer responsibility, not a reason to impose publisher
source-fetch ceremony on ordinary CLI use.

### Pre-production cutover

- There is no compatibility or deprecation period. Config-v1/v2 and the old write CLI are removed
  in the same change that introduces config-v3 and the new lifecycle.
- Existing pre-release generated SDKs are regenerated. The new CLI is not required to check,
  update, compile, or preserve byte equality with output created by the old control plane.
- `generate --out ...` is not reinterpreted as `update`; it is simply not a valid invocation.
- Repository-owned fixtures, Safe generation, external acceptance corpora, release packaging, and
  downstream examples are converted to config-v3. No legacy copy is retained as a supported path.
- An application updates only after a reviewed musubi.rs release revision is available. Its
  manifest, scripts, tracked generated SDK, Cargo lockfile, and release evidence change as one
  reviewed transaction.
- Existing EVM selectors, topics, ABI encoding, strict decoding, EIP-712 hashes, and Safe semantics
  are not intentionally changed by this control-plane cutover. Identical semantics may keep their
  current protocol identifiers; there is simply no shim for old generated artifacts.
- If generated-runtime API, release JSON, snapshot JSON, or another serialized contract changes,
  assign the appropriate new exact schema/ABI version and remove the old reader. Do not keep a
  dual-reader solely for pre-production history.

### Exact cutover evidence formats

- The sole release JSON schema is `musubi/release-v3`. It adds required `manifest_schema` and
  exact-byte `manifest_sha256`, also exposed as generated Rust root constants. The root
  `runtime_identity()` records the same binary/library-owned identity even for typed-data-only SDKs.
- The sole snapshot JSON `schema_version` is `3`. It records the manifest schema/digest and requires
  exact nonzero bytecode lengths; no reader infers module names, artifact kinds, or unknown lengths.
  Manifest-only changes are package-level provenance changes, independent of EVM wire categories.
- Package evidence uses the sole `musubi/package-provenance-v2` schema, including the exact target,
  manifest/runtime identities, preserved contract/typed-data evidence, and checksum-bound subjects.
  The CycloneDX schema is unchanged; its standard property list additionally carries manifest identity.
- The core/runtime generated-runtime interface and its semantics remain unchanged. These new
  generated root provenance accessors use existing core types, so generated ABI identifier `1`
  remains exact; there is no bridge for earlier generated control-plane evidence.

## Required Verification

- CLI parsing rejects every generation-field/runtime override for single- and multi-contract
  manifests, including aliases and environment-based injection attempts.
- `init` uses exclusive creation and leaves existing regular files, directories, symlinks, FIFOs,
  and raced destinations unchanged.
- `generate` succeeds only for an absent output and never converts an existing target into an
  update.
- `update` succeeds only for a valid marker-owned output and keeps the old SDK recoverable across
  every staged write, validation, rename, cleanup, signal, and rollback failure. ADR 0015 defines
  complete file-byte recovery after installation has succeeded but backup cleanup fails.
- `check` and pre-installation rejection leave the manifest, output, snapshots, lockfiles, and
  unrelated files byte-identical. Failed installation restores the prior SDK, or preserves its
  backup if rollback fails. A post-installation cleanup failure may leave the new SDK live with
  complete old SDK recovery evidence; it is still reported as failure, never silently as success.
- Config-v3 unknown fields, unsafe paths, and forbidden runtime fields fail before filesystem
  mutation. Config-v1/v2 are rejected as unsupported and have no generation path.
- Config-v3 generation remains byte-identical across repeated runs and a fresh checkout.
- Release evidence contains the config-v3 schema, manifest digest, exact binary-owned runtime
  identity, all input digests, and the unchanged unverified-coherence notice.
- The packaged binary can run `init` without private SSH credentials. Released `generate`,
  `update`, and `doctor` use normal Cargo dependency resolution and fail redacted if the required
  private dependency cannot be resolved; they never fall back to a path or alternate URL.
- Publisher package validation retains private SSH, fresh-checkout, reproducible-build, checksum,
  SBOM, provenance, bundled-crate, sparse-ABI, typed-data, and Anvil/Safe gates.
- Human and JSON diagnostics report the same stable facts without absolute paths, artifact bodies,
  credentials, keys, or raw remote errors.

## Consequences

- The normal workflow is shorter and identical for one or many contracts.
- A committed manifest plus a pinned generator release fully describes how an SDK is regenerated.
- First creation and deliberate replacement are distinguishable in command names and tests.
- The first production release exposes one manifest schema and one lifecycle instead of carrying
  pre-release compatibility branches.
- Updating the generator/runtime release requires an explicit binary pin and generated-output
  review; it does not require a self-referential revision in the manifest.
- Existing consumer repositories must migrate their manifest location when the output is not below
  the current manifest root.
- Hardened release construction remains operationally substantial, but it is paid once by the
  publisher instead of presented as ordinary generator setup.
- Lab02's inability to prove external compiler coherence remains explicit and unchanged.

## Rejected Alternatives

### Permit overrides for a single-contract manifest

This creates a second authority and makes a command copied from a one-contract project unsafe when
the manifest later gains another entry.

### Warn and ignore overrides for a multi-contract manifest

A successful exit would falsely imply that the requested values were used. Ambiguity must be an
error, not a warning.

### Add a generic force or override flag

It collapses creation and replacement back into one command and can overwrite a target whose
ownership was not deliberately established.

### Retain config-v1/v2 readers or a legacy CLI subcommand

There is no production consumer requiring them. Retention would multiply parser, documentation,
test, and security surface at the exact point the first stable contract should be made small.

### Remove the manifest and support only direct artifact flags

This optimizes a demo invocation at the cost of reproducible multi-contract, typed-data, limit,
output configuration and generator/runtime identity evidence.

### Delete the hardened private SSH release pipeline

The strict source/reproducibility checks are valuable at the publisher boundary. The problem is
their placement in the consumer mental model, not their existence.

### Claim compiler artifact coherence from matching fields or hashes

Without owning or independently reproducing the compiler invocation, lab02 cannot establish that
claim.
