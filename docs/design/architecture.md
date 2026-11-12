# Musubi Architecture

## Status

Accepted product contracts are defined here and by the [ADR index](README.md).
[ADR 0020](../adr/0020-craftguild-standards-adoption.md) replaces earlier local build,
physical/test-inclusive size rules and distribution implementation with the fixed
standards v0.6.1 engine. It preserves the protocol and ownership contracts below.
Current qualification and review state live in Beads, independently of accepted
design and archived historical evidence.

The product is Musubi (`musubi.rs`); its executable is `musubi`, CLI package is
`musubi-cli`, and opt-in consumer facade is `musubi`. Root help and human version
use the common banner; machine version remains closed GeneratorInfo. Source
builds and copy-only installation follow ADR 0020, which supersedes the local
Make implementation in ADR 0019. Installation/release configuration does not
imply an actual product release or qualification on another host.

First-party builds, tests, CI and generated SDKs use Rust 1.95.0 with declared
minimum Rust 1.95 under ADR 0020. Its Alloy/Serde adaptation preserves owned
public types, ABI 2 and wire semantics. SDK minimum changes use a preserved-output
cutover through normal generation; recorded historical producers are not edited.

## 1. Purpose

Generate deterministic typed Rust SDKs from reviewed external Solidity compiler
artifacts and independent, Solidity-linked closed EIP-712 schemas. SDKs provide
contract calls, events, errors, transactions, kind-specific artifact identity,
optional deployment, ABI codecs and typed-data hashes.

Solidity parsing, semantic analysis, optimization and compilation remain with
external toolchains. Musubi is not a Solidity compiler.

## 2. Goals

- Own the Rust types for all ABI wire types and remove duplicated ABI/bytecode
  declarations and consumer dependence on `sol!` output.
- Produce complete standalone crates that execute without the original artifacts.
- Recover verified enum, UDVT and named-struct identity from optional AST data.
- Provide offline descriptors, ergonomic backend-connected clients and versioned
  Safe transaction/signature utilities while isolating Alloy.
- Keep deployable and interface artifacts distinct throughout input and provenance.
- Treat typed-data schemas/vectors as an independent authority rather than infer
  EIP-712 semantics from ABI.
- Use one manifest-only workflow with separate initialization, creation, update
  and read-only inspection, and retain the generating CLI's build observations.
- Keep normal validation focused on format, implementation quality and product
  tests; actual publication is a separate operation.

## 3. Non-goals

The product and generator do not run, download, configure or pin solc, solc-js,
Hardhat or Foundry. ADR 0020 permits pinned Foundry acquisition only in CI to
prepare owned local Anvil tests. Do not derive
ABI from bytecode; prove common compiler provenance; implement a general Ethereum
RPC client or Safe Transaction Service; own application finality/quorum/nonce/fee/
signing/broadcast/receipt policy; override generation settings through flags or
environment; or acquire/install an unpinned latest binary. Publisher credentials,
registry operations and actual release publication are separate tasks.

## 4. Trust Boundary

| Input | Requirement | Authority |
| --- | --- | --- |
| ABI JSON | Required for each contract | Selectors, topics, encoding and method surface |
| Artifact kind | Required | `deployable` or `interface` surface |
| Creation bytecode | Required for deployable | Linking, init code and CREATE |
| Runtime bytecode | Optional for deployable | Code identity and template verification |
| AST SourceUnit | Optional | Verified enum/UDVT/struct enrichment |
| Metadata | Optional | Compiler observations and metadata digest |
| Storage layout | Optional for deployable | Layout digest and review information |
| Link/immutable references | Optional for deployable | Linking and runtime masking |

Interfaces accept ABI and optional AST/metadata, and reject creation/runtime
bytecode, storage layout, link and immutable fields. Empty creation bytecode is
not an interface marker. Typed-data entries independently require a closed schema
and Solidity-linked vector bundle; ABI, AST and `internalType` never define their
semantics.

Musubi validates syntax and internal consistency, not ABI/bytecode provenance
equivalence. Contract generation/check/doctor emits informational `MSB-I001` and
records `external_artifact_coherence = "unverified"`. Warning denial does not
promote that notice. Missing AST is a separate warning. An external attestation
never becomes a claim that Musubi verified compilation.

The fixed in-repository external corpus records an observed revision separately
from `artifact_revision_coherence = "unverified"`; it cannot be promoted to
`verified`. Retained root/ABI directory handles, no-follow nonblocking opens and
bounded snapshots bind later schema/digest checks to the same bytes. A live
consumer checkout and its compiler coherence are a separate pipeline's concern.

## 5. Configuration And Input Files

Only closed TOML `musubi/config-v4` is accepted. Earlier pre-production schemas
have no reader, normalization or legacy command branch. One persistent manifest
owns package, contracts, typed-data, limits, runtime and output:

```toml
schema_version = "musubi/config-v4"

[package]
name = "example-contracts-sdk"
version = "0.3.0"
edition = "2024"

[generation]
output = "generated/example-contracts-sdk"

[runtime]
source = "path"
root = "../musubi-source"

[[contracts]]
id = "Token"
kind = "interface"
abi = "artifacts/ERC20.abi.json"

[[contracts]]
id = "ExampleContract"
kind = "deployable"
abi = "artifacts/ExampleContract.abi.json"
creation_bytecode = "artifacts/ExampleContract.creation.json"

[[typed_data]]
id = "Policy"
rust_module = "policy_typed_data"
authority = "closed-schema-v1"
schema = "typed-data/Policy.schema.json"
vectors = "typed-data/Policy.vectors.json"
```

`generation.output` is manifest-relative with normal path components. Reject
absolute paths, empty/`.`/`..` components, backslashes, controls and symlinks. Put
the manifest at a common ancestor of inputs/output. Required `[runtime]` is a
closed Git/Path union, independent of generator identity:

- Git records a Cargo-compatible SSH/HTTPS URL and a full, nonzero, lowercase
  40-digit commit. Reject credentials/passwords, HTTPS usernames, query/fragment,
  controls, branches, tags, short hashes, dirty and unknown revisions. SSH users
  are allowed. Both support crates use the same URL and revision.
- Path records a relative root resolving fixed `crates/musubi-core` and
  `crates/musubi-runtime`. Only this explicitly trusted Cargo source permits `.`
  and `..`; artifact/output confinement is unchanged. Reject absolute paths,
  controls and ambiguous platform separators. Generated Cargo paths are rebased
  relative to the SDK; temporary compile copies alone may use absolute rebasing.
- Path is mutable and placement-dependent. Ordinary Cargo configuration, runtime
  sources and caches are trusted separately, not recursively authenticated by a
  source snapshot. Runtime package versions are not inferred from the generator.
  Generated-runtime ABI checks and actual compilation establish compatibility.

No package/contract/typed-data/limit/output/runtime override exists in flags or
environment, regardless of entry count. `--manifest`, `--format human|json`,
`--deny-warnings` and report destinations control selection/presentation only.
Reject ambiguous or unsupported flags rather than ignore them.

`init` exclusively creates a deterministic template without an SDK or inferred
runtime. `generate` only installs an absent output. `update` atomically replaces
only a valid marker-owned SDK. `check` rerenders read-only with the recorded
validated producer and compares every byte; a different checking CLI is merely
informational. `doctor` uses a private inspection tree. No `--force` or generation
field override bypasses ownership.

ABI files are JSON arrays. Bytecode is a fully linked `0x` hex JSON string or a
closed solc-compatible `object`/`linkReferences` object; runtime objects may also
contain `immutableReferences`. Reject unresolved placeholders in raw hex. AST is
one SourceUnit object. Resolve artifacts from the retained manifest directory;
apply type checks, bounded reads and no-follow traversal to the same descriptor.
Unknown fields fail closed.

### 5.1 Typed-data schema v1

```json
{
  "schema_version": "musubi/typed-data-schema-v1",
  "types": [
    {
      "name": "Message",
      "fields": [
        { "name": "sender", "type": "address" },
        { "name": "values", "type": "uint256[]" }
      ]
    },
    {
      "name": "EIP712Domain",
      "fields": [{ "name": "chainId", "type": "uint256" }]
    }
  ],
  "roots": [
    { "kind": "struct_hash", "type": "Message" },
    { "kind": "typed_data", "type": "Message", "domain": "EIP712Domain" }
  ]
}
```

Only struct field order is semantic; the `types` array order is not. Roots are a
closed `struct_hash`/domain-bound `typed_data` union. Different domains for one
primary type are allowed, duplicate kind/type/domain combinations are rejected,
and every declared type must be reachable from a root/domain. No supplied hash or
precomputed encodeType field is accepted.

### 5.2 Typed-data vector bundle v1

```json
{
  "schema_version": "musubi/typed-data-vectors-v1",
  "evidence": {
    "solidity_source_path": "test/MessageHash.t.sol",
    "solidity_source_sha256": "0x<64 lowercase hex>",
    "artifact_target": "test/MessageHash.t.sol:MessageHashTest",
    "artifact_sha256": "0x<64 lowercase hex>",
    "compiler_version": "0.8.30+commit.73712a01",
    "compiler_settings_sha256": "0x<64 lowercase hex>",
    "vector_command": "forge test --match-test testMessageVector"
  },
  "vectors": [
    {
      "root": "Message",
      "domain_type": "EIP712Domain",
      "value": { "sender": "0x<40 lowercase hex>", "values": ["1", "2"] },
      "domain": { "chainId": "1" },
      "type_string": "Message(address sender,uint256[] values)",
      "type_hash": "0x<64 lowercase hex>",
      "struct_hash": "0x<64 lowercase hex>",
      "domain_type_string": "EIP712Domain(uint256 chainId)",
      "domain_type_hash": "0x<64 lowercase hex>",
      "domain_separator": "0x<64 lowercase hex>",
      "digest": "0x<64 lowercase hex>"
    }
  ]
}
```

Integers are canonical decimal strings; address/bytes are lowercase `0x` hex;
bools, strings, arrays and exact-field structs use their corresponding JSON
forms. Fixed bytes/arrays match declared length. Struct-hash roots omit all six
domain/digest fields; domain-bound roots require them. Every declared root has a
vector. Recompute canonical type strings/hashes, struct hashes, separators and
digests exactly. Reject unknown/duplicate/missing fields, uppercase hex, leading
zero decimals, negative zero and out-of-width integers. Evidence paths and
`source-path:Contract` targets use normal relative components and valid Solidity
contract identifiers, without controls or platform-specific separators.

## 6. Processing Pipeline

```text
closed manifest + actual generator observations
  -> bounded input loading and kind/ABI/bytecode/typed-data validation
  -> canonical wire IR + optional AST enrichment + independent typed-data IR
  -> name/collision checks -> bounded Rust and asset rendering
  -> private staged Cargo validation using ordinary trusted configuration/cache
  -> create-only install or marker-owned atomic update
```

ABI IR alone determines selectors/topics/error selectors and codec layout. AST
may replace a developer-facing type only after unique mapping to ABI. Missing,
unknown or ambiguous enrichment warns and falls back; a contradictory underlying
wire type fails.

Wire tuple identity uses canonical ABI types without names/internalType/AST. Owned
identity includes exact ABI names, normalized Rust names, ordered fields, array
shape, nested owned identities and uniquely verified enrichment. Encode once as
length-prefixed, tagged, domain-separated `owned-tuple-surface/v1` bytes, within
existing budgets, never JSON/debug text. Fallback depends only on ABI surface;
`internalType` alone is not trusted. Anonymous names are `Tuple` plus the first
16 Keccak-256 bytes as 32 lowercase hex digits. Reuse identical owners in a
contract; digest or normalized namespace collisions fail with `MSB-E004`, without
order-dependent suffixes. Wire behavior does not change.

Typed-data IR exclusively determines EIP-712 graphs and hashes. Append referenced
types lexicographically after the primary type, preserving each struct's field
order. ABI codecs and typed-data authority remain distinct. Canonical item,
component and link ordering makes the same binary/manifest/input/runtime produce
identical bytes. Different build observations may legitimately change provenance.

## 7. Workspace Boundaries

| Crate | Responsibility |
| --- | --- |
| `musubi-core` | Owned values, descriptors, kinds, digests and compatibility |
| `musubi-runtime` | ABI/typed-data codecs, operations, deployment, backend SPI and ABI guard |
| `musubi-codegen` | Bounded loading, IR, enrichment, rendering, snapshots/diff, staging and process runner |
| `musubi-cli` | Manifest-only orchestration and diagnostics |
| `musubi-alloy` | Provider/wallet adaptation to runtime SPI |
| `musubi-safe` | Version markers, transactions, signatures and reviewed registry |
| `musubi-testkit` | Backend conformance and generated-crate inspection/compilation |
| `musubi` | Opt-in component facade |

No dependency cycles. Core depends on neither Alloy nor an async runtime. Runtime
may privately use Alloy dynamic ABI; its public signatures/rustdoc and generated
SDKs expose only owned types. The Alloy adapter may expose its own provider types.

## 8. Owned Public Type System

Owned values include Address, Bytes, FixedBytes<N>, Hash, Uint/Int, UFixed/Fixed,
FunctionPointer, artifact/bytecode types, requests, logs, receipts and typed-data
descriptors. Integer constructors reject out-of-width values up to 256 bits;
fixed-point decimal precision follows Solidity's inclusive 0..80. Dynamic arrays
use Vec, fixed arrays use `[T; N]`, and tuples use the identity above. Indexed
dynamic event values remain typed topic hashes. Verified AST enrichment supplies
closed enums, transparent UDVT newtypes and named structs; encoding still follows
ABI descriptors.

## 9. Generated SDK Surface

Each SDK is a complete Cargo crate with source, canonical ABI, binary bytecode
assets and release provenance. Large assets use `include_bytes!`, not Rust arrays.
Contracts expose markers, unbound operations and bound clients. Read operations
encode/call/decode/estimate; writes additionally construct unsigned transactions
and send only through TransactionExecutor. Explicit target addresses and ABI
mutability are enforced before backend/RPC execution; native value is payable-only.
Keys, nonce, fees, retries and finality belong to the executor.

Deployable constructors require exact library-address coverage and construct init
code. Missing/nonpayable constructors reject nonzero value. Deployment accepts
CREATE requests only, without a target address. Runtime template verification
accounts for links/immutables and revalidates all range bounds and non-overlap,
even for publicly constructed/deserialized descriptors; no last-write-wins or
masking false matches. Interfaces expose calls/transactions/events/errors and
selector-free value codecs, without deployment APIs or bytecode assets.

Typed-data modules expose owned structs, canonical strings/hashes, nested/array
hashing, domain separators and final digests. Events offer strict decoding and a
deterministic metadata-free Log encoder from the same ABI authority.

SDKs expose raw/canonical ABI SHA-256, creation/runtime template Keccak, optional
metadata/AST/layout digests, reference inventory, kinds, generator observations,
ABI version, explicit runtime source, unverified coherence and typed-data schema/
vector/type/evidence identities. `generator() -> &'static GeneratorInfo` returns
the recorded literals without RPC or recapturing Git/environment at SDK build time.

### 9.1 Semantic file ownership

ADR 0017 groups each operation's Args/Return, codec, descriptor and methods;
events/errors/shared types/typed-data structs each have their own semantic owner.
Keep public facades and `types` reexports. Paths derive from canonical owners,
not global indices or accumulated line counts. Independent owner changes retain
unrelated leaf paths/bytes; genuine naming/dependency changes propagate honestly.
Do not duplicate global provenance in every leaf or shard a large owner by size.

Only closed `.musubi-generated` JSON `musubi/generated-v2` is accepted. Top-level
fields are schema_version, `layout = "musubi/semantic-layout-v1"`, files. Sorted,
unique records contain only normal root-relative UTF-8 path, nonnegative integer
bytes and prefix-free lowercase 64-digit SHA-256. Reject duplicate keys/unknown
fields, invalid path/depth/component sizes and casefold collisions. Inventory
covers every final Cargo/Rust/asset/release file except the marker itself; tree
and ancestor directories match exactly. Reject extra files/empty directories,
symlinks and special entries. Marker maximum is 16 MiB within total budgets.
Never place Cargo locks/targets/caches inside the owned SDK.

Release-v4 owns package/runtime/generator/input identity; the marker binds those
files without duplicating authority. Codegen, testkit and recovery share bounded
ownership rules. Testkit scans all leaves and rejects forbidden macros, environment
references and path escapes, allowing only inventory-bound asset includes. Hash
consistency alone does not authorize arbitrary source. `update` validates existing
self-consistency without demanding unchanged current inputs; `check` additionally
rerenders from current inputs and the recorded producer. Neither is authentication.

Old literal markers have no compatibility reader. Explicit migration preserves
old SDK/recovery tools, renames the SDK aside and generates into the absent output
with the same placement meaning. Never overwrite an unknown competing entry or
hand-edit a marker. New-format updates retain ADR 0015 recovery behavior; old
recovery evidence is not deleted. Live consumer cutovers are separate tasks.

## 10. Runtime SPI And Alloy Adapter

Object-safe boxed futures separate ReadBackend (chain ID, call, estimate, code,
storage, logs and receipt) from TransactionExecutor (completion, signing,
broadcast and pending/receipt policy). Unbound descriptors need no backend; bound
clients share cheap-clone handles. Preserve error sources internally without
printing credentials, headers or keys. Alloy provider/wallet/network types stay
in the adapter, outside core/runtime/SDK public APIs.

## 11. Safe Adapter

Sealed SafeVersionSpec markers and version-specific signature unions prevent
cross-version misuse. Only SafeV141 is implemented: EIP-712 EOA, eth_sign EOA,
approved hash and EIP-1271 signatures; CALL/DELEGATECALL, SafeTx hashing,
approveHash and execTransaction. Reject signature counts exceeding static blob
bounds before sorting. The packer validates declared owner order, duplicates,
threshold, marker 0/1 canonical address words, approved-hash zero s, dynamic
offsets/bounds/padding and trailing data. PackedSignatures has no raw constructor:
layout alone proves neither EOA owner recovery nor contract-signature validity.
Cryptographic validity remains an on-chain check.

Wire methods/events/errors use checked generated ABI; hashes use the independent
closed schema/Solidity vectors. Reviewed pins are safe-smart-account
`bf943f80fec5ac647159d26161446ac5d716a294` and safe-deployments
`5bb0ebd7150a777f39bec4733e4d799c4b637b49`. SHA-512-pinned npm artifacts retain
Safe.sol exactly matching SHA-256-pinned source at the immutable account revision
URL. Registry-v2 binds source revision/digest and requires every canonical singleton,
proxy-factory and fallback-handler name/address/runtime hash/upstream digest to
match reviewed compile-time pins. An address or VERSION() alone is insufficient.

Explicit asset updates validate registry/fixtures/inputs/SDK in a repository-local
same-filesystem staging tree before a single no-replace rename transaction. Check
device identity, roll back all installed paths in reverse order after failure,
continue after an individual rollback error and retain/report every unrestored
backup. Do not progressively copy staged files into production paths. Application
owner/threshold/module/guard/fallback policy is outside this generic adapter.

## 12. Diagnostics And Limits

Stable diagnostic code, severity, redacted message and optional source-relative
location carry the same facts in human and JSON output. User input failures do
not panic. Manifests may lower hard limits, never raise or remove them:

| Resource | Hard limit |
| --- | ---: |
| manifest | 1 MiB |
| aggregate input | 128 MiB |
| one ABI file | 8 MiB |
| one creation/runtime bytecode file | 2 MiB |
| one AST file | 32 MiB |
| one metadata/storage layout file | 16 MiB |
| contracts | 256 |
| one typed-data schema/vector file | 8 MiB / 16 MiB |
| typed-data entries / types / roots / vectors per entry | 256 / 4,096 / 4,096 / 4,096 |
| aggregate typed-data fields per entry | 65,536 |
| ABI items per contract | 4,096 |
| aggregate ABI components per contract | 65,536 |
| cumulative ABI fixed-array element product per type path | 65,536 |
| tuple/array nesting | 64 |
| cumulative typed-data fixed-array element product per type path | 65,536 |
| AST nodes per contract input | 500,000 |
| UTF-8 identifier length | 256 bytes |
| generated files | 10,000 |
| aggregate generated output | 256 MiB |
| generated ownership marker | 16 MiB (including the marker in total files/bytes) |

ABI structural preflight streams items/fields without first retaining a JSON
value tree, charging items/components before their bodies and sharing component
budgets across a contract. Loading, mutable IR and retained SDK inspection use the
same preflight and LimitExceeded result. Retained pretty canonical ABI uses the
generated-output allowance, preserving valid larger canonical bytes. Unique-key
JSON and semantic validation follow; ownership hashes waive no resource budget.

ABI/snapshot/runtime validation applies the cumulative fixed-array cap before
allocation. Typed-data preparation builds one bounded name index and memoizes
canonical hashes. Struct/array paths share the 64-edge and cumulative 65,536-element
budgets. Hash 32-byte words incrementally rather than clone graphs or concatenate
nested buffers. Type strings, root evidence, descriptors and every tree insertion
charge effective output bytes/files/depth before accumulating complete output.

Cargo inspection/staging has a 300-second whole-process-tree deadline and 8 MiB
combined output limit. Child, readers and process group share the deadline; stop
remaining descendants even after direct-child exit. Every timeout/read/wait failure
stops/reaps the group before returning. Unsupported capabilities fail before any
write; no path-based or direct-child-only fallback. Linux/macOS are supported;
Windows operation/CI is excluded.

Init is exclusive-create, including races. Generate refuses every existing entry
before mutation. Update stages, closes/fsyncs as applicable, backs up and atomically
installs with no-replace renames. Revalidate retained trees with effective file,
byte, marker and depth limits, including after backup. Rollback preserves unknown
entries and retains/reports backups on failure. Cleanup uses the same bounded
handle-relative traversal. Before recursive deletion persist every old SDK byte
in one exclusive sibling recovery file. Partial cleanup or final unlink failure
retains complete recovery evidence and reports failure; the new SDK may remain
live after installation. Recovery extracts only into a new directory, never over
an existing SDK. Do not repoll completed child processes.

Generated module declarations use rustfmt::skip to protect exact bytes from
consumer formatting. The renderer owns trailing whitespace/one-newline EOF;
exact check rejects drift. Independent sparse fixtures (empty, fallback/receive,
unit function, event-only and no custom errors) receive standalone Clippy and
whitespace checks. Conditional imports/clients/error decoders/name reservations
follow only the actually generated surface.

Testkit opens the retained SDK once and inspects a bounded no-follow memory
snapshot. Compile a private materialization while its guard remains alive; never
reopen the original by path. Normalize raw identifiers before macro/path checks.
Relocated inspection explicitly carries conceptual SDK and trusted configuration
roots, rebasing Path only within the temporary copy. Ordinary Cargo settings and
reusable caches are separate trusted inputs. Keep lock/target outputs outside the
SDK; never claim the runtime dependency tree was captured or authenticated.

## 13. Compatibility And Release Identity

Closed snapshot schema 4 records canonical ABI, Rust enrichment, kind/deployment
identity and actual nonzero template lengths, typed-data authority, exact manifest
schema/hash, generator/runtime ABI and independent provenance. Validate references
against actual template length, without an old inferred-length reader. Diff
separately reports EVM wire, Rust source and typed-data breaking/additive changes,
artifact_kind_changed, deployment identity and provenance-only changes. A kind
change is always Rust-breaking and deployment-changing. AST/name/owned-identity
changes may be Rust-breaking without a wire change; bytecode-only changes affect
deployment. Metadata-only differences remain provenance-only.

Only release-v4, GeneratorInfo-v1 and runtime ABI2 are accepted. Manifest/input
hashes, runtime Git/Path source, generator observations and unverified coherence
remain distinct. Reusable core/runtime support dependencies use compatible
minimum ranges; the repository and consumer lockfiles own their exact graphs.

Capture package version, full Git commit/dirty, rustc, actual host/target/profile
at CLI build time. Git/HEAD/status unavailable gives honest unknown; known dirty
includes stage, worktree, nonignored untracked and submodule changes. Both human
and closed JSON version work offline before manifest/runtime resolution. SDKs
retain literal observations; generate/update stamp the current CLI, while check
validates and replays the retained producer without restamping, masking metadata
or creating a self-referential commit cycle. Consistent rewritten metadata is not
authenticated; identical dirty displays need not identify the same source bytes.

### 13.1 Validation and deferred distribution

`make check` connects common Code measurement, format, Clippy/rustdoc and all
ordinary product tests, exact SDK replay and artifact/ownership/resource gates.
Focused iteration precedes one final aggregate. Retain explicit Anvil jobs on
GitHub-hosted runners with ADR 0020's pinned CI-only preparation, reporting skips
separately. Local Anvil remains externally provisioned. Shared tokei15 Code measurement
covers implementation only; dedicated tests/helpers/fixtures/design are excluded.
Source-only Python3.11+ ownership validation still needs no Cargo, private runtime
or network and rejects missing/empty/unknown registries and failed scans. Its
registry is also used by exact CLI checks. Non-SDK compiler/vendor artifacts have
separate exact inventories and authorities, not path-only size exemptions.

New or changed product scenarios use Rust tests; scripts orchestrate commands and
resources. Actual CLI acceptance explicitly runs its Rust artifact test against
the verified executable, with test compilation in a separate target directory.
Default ignored results do not qualify acceptance. Infrastructure tests and
adopted frameworks retain their own language/tooling; unrelated legacy scenarios
are not rewritten solely by this adoption.

ADR0020 introduces shared SBOM/receipts/installer and release declarations; it does
not restore the old bespoke publisher/SSH pipeline. Actual registry publication,
clean-HEAD delivery packaging, release/tag/signing and asset retention are separate
operations. Normal CLI/SDK compilation remains mandatory. Local integration is
not blocked merely by deferred publication credentials, but a failed required
local gate blocks completion. Additional hosts, live remote acquisition and
full external campaigns require their own honest qualification evidence.

## 14. Documentation Contract

Document every public module/type/trait/method/field/error, including generated
ABI signatures, mutability, selectors/topics and enrichment. Never embed source
contents, absolute paths or secrets in public diagnostics/provenance. Record new
durable decisions as ADRs and update this architecture in the same change.
