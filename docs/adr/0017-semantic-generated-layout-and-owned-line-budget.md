# ADR 0017: Semantic generated layout and ownership-based size exemption

## Status

Accepted — 2026-09-07. Semantic layout and ownership implementation exist; design
acceptance is distinct from current qualification and independent review. Historical
cutover/evaluation originals are hash-preserved privately and indexed in Beads.
Amends generated layout/ownership portions of ADR0005/0014/0016. Retains ADR0013
I/O, ADR0015 recovery, ADR0016 manifest-only configuration/GeneratorInfo, wire and
runtime ABI. ADR0020 subsequently replaces physical/test-inclusive size counting
with implementation-only tokei15 Code and common canonical reproduction checks.

## Context and historical baseline

Contract-wide types.rs mixed arguments/results/events/errors/tuples/enrichment.
Path-only exemptions could hide handwritten source; the original literal
musubi/generated-v1 marker had no inventory. Split by semantic responsibility and
bind every output byte instead. Historical evaluations do not select the
current build revision or qualify the current source. Protocol baseline is
config-v4/release-v4/snapshot4/GeneratorInfo-v1/runtime ABI2. No production backward
compatibility is promised for the old layout/marker; no wire/business change is
licensed by this decision.

## Semantic layout

```text
src/lib.rs                       # Producer, crate entry and module list.
src/<contract>/mod.rs            # Marker/At/Client, facade and aggregate accessors.
src/<contract>/types.rs          # Existing public reexports.
src/<contract>/operations/...    # One function's Args/Return, codec, descriptor, methods.
src/<contract>/events/...        # One event's type, topic, encode/decode.
src/<contract>/errors/...        # One error's type and codec.
src/<contract>/shared_types/...  # One tuple/struct/enum/UDVT and conversions.
src/<contract>/constructor.rs    # Deployable constructor, when present.
src/<contract>/artifact.rs       # Kind-specific ABI/bytecode/metadata access.
src/<typed-data>/mod.rs          # Authority facade/reexports.
src/<typed-data>/types/...       # One struct and its conversions/hash/domain methods.
src/<typed-data>/descriptors.rs  # Authority graph aggregation.
assets/<contract>/...           # Canonical ABI/binary assets.
```

This is a responsibility contract, not a requirement to emit empty files. Keep
arguments/results together, reuse shared owners, preserve distinct owned tuple
surfaces under ADR0009 and retain empty/sparse/typed-data-only behavior. Kind and
ABI still control constructor/deployment existence. Do not reparse all typed-data
types for each reference. Facades, unions, graph tables and provenance may be large
because aggregation is their purpose; do not move every codec body back into them.
Keep typed builders/typestate and existing marker/client/types/methods/traits,
including contract::types::T and contract::T reexports of the same definition.

Private paths/descriptors derive from canonical semantic owners and keyword-safe
normalization, never enumeration indices, cumulative lines or discovery order.
Operations/events/errors use full canonical signatures; overload suffixes derive
from signatures from the start. Bounded readable stems/digests have pinned domains,
input and truncation golden tests. Shared/typed-data types use verified identity
and authority/type name. Reject Rust namespace and filesystem/case/keyword/length/
short-digest collisions with bounded diagnostics, without sequence fallback.
ABI field/tuple order remains wire authority. Unrelated owner edits retain existing
independent leaf paths/bytes; real reexport/table/asset/provenance/dependency changes
and overload-public-name effects are recorded honestly. Do not copy global hashes/
GeneratorInfo into each leaf. Locality tests use the same recorded producer. No
layout/line-count/file-name override or alternate legacy mode is added.

## Closed ownership marker v2

Keep basename .musubi-generated; accept only a closed object with schema_version
musubi/generated-v2, layout musubi/semantic-layout-v1 and files. Each sorted unique
record has only path, bytes and sha256. Enumerate all final Cargo/Rust/asset/release
files except the marker to avoid self-hash recursion.

Paths are normal slash-separated relative UTF-8, without absolute/empty/dot/parent/
backslash/control components, invalid UTF-8, casefold collisions or existing depth/
component bound violations. Bytes is a nonnegative integer, not bool; SHA256 is
prefix-free lowercase64 hex. Reject duplicate JSON keys/unknown/missing fields or
wrong schema/layout. Marker is at most16MiB and counts in the 10,000-file/256MiB
aggregate limits. Reserve budgets incrementally; do not raise them to fit layout.
Build hashes after final format/EOF normalization, and never change files after
marker construction. The actual tree and directories equal inventory+marker and
their ancestors exactly. Reject unknown files, empty extra directories, symlinks,
FIFOs and other special entries. Cargo locks/targets belong to private validation,
not the retained SDK.

Release-v4 remains package/manifest/runtime/producer/kind/asset authority; the
marker binds its bytes without duplicate semantic metadata. Codegen, incoming-tree
inspection, lifecycle, testkit and recovery use the same bounded ownership rules.
Rust validation lives in codegen without a reverse testkit dependency or weaker
reader. Update checks existing self-consistency without demanding that legitimate
current inputs are unchanged. Check additionally rerenders current inputs with the
validated recorded producer, reporting byte drift without repairing the tree.

Testkit replaces fixed three-file assumptions with semantic ownership checks over
every leaf. Hashes do not permit arbitrary code: preserve include/environment/path
macro restrictions, raw-identifier normalization, inventory-confined module/asset
resolution and Rust/release/Cargo agreement. Consistently forged metadata is not
authentication or compiler coherence; unverified external coherence remains.

## Source-only ownership and shared measurement

scripts/generated-manifests.json is the reviewed sorted/unique list of retained
SDK manifests, shared with mandatory exact checks. Resolve output from config-v4
instead of duplicating roots. Do not register optional transient outputs. Python
3.11+ standard-library inspection needs no CLI/Cargo/private runtime/network;
missing interpreters fail rather than auto-install.

Validate manifest/root mapping, complete marker/size/hash/tree, root overlap/
containment/case collisions, source scans/reads and nonempty coverage. Discovering
a marker only detects a missing registration; it never automatically admits an
exemption. Unknown short files within a registered root are ownership violations.
Explicit negative fixtures remain distinct. Enumerate tracked and nonignored
untracked sources without hiding tracked deletion, read failures, upstream pipeline
errors or invalid UTF-8. Keep non-SDK compiler/vendor/Safe/corpus artifacts in
separate exact authority/gate inventories; never forge SDK markers or broadly
exclude out/fixture paths to hide authored source. Report authored, owned Rust and
other artifact counts separately. Source-only checks do not demand today's CLI
producer; current-input agreement comes from exact replay. Shared positive/negative
ownership fixtures keep Rust/Python rules aligned without sibling runtime imports.

ADR0020 owns tokei15 measurement and generic measurement regression tests. Its
Code<=400 implementation-only rule supersedes this ADR's former physical/test
counting. Verified generated exceptions require full ownership plus isolated
canonical regeneration; filenames/headers alone never qualify. Do not split a
large generated semantic owner into arbitrary size shards. Resource, compile,
Clippy/rustdoc and wire/golden requirements still apply.

## Resource, migration and recovery boundaries

Stream ABI structure before allocating a JSON Value tree, charging item/component/
nesting budgets before each body: 4,096 items and 65,536 aggregate components per
contract. Ordinary input, mutable IR and retained SDK inspection share LimitExceeded
behavior; valid large canonical assets retain generated-output byte allowance.
Marker validity never waives resource limits. Layout/marker changes alone retain
config-v4/release-v4/snapshot4/GeneratorInfo-v1/runtime ABI2.

Reject old literal markers without compatibility readers, lazy inventory repair or
production skip options. An explicit cutover preserves old SDK/tools/recovery,
renames the directory away, and generates into the absent normal output without
changing Path placement. Inspect partial output before restoration; never overwrite
an unknown competing entry. Preserve/report both contents on recovery failure.
This is an operator cutover, not a second persistent transaction engine. New-format
updates retain ADR0015 no-replace/backup/durable recovery/cleanup failure semantics;
recovery-v1 framing remains. Preserve old recovery files/tools rather than add a
legacy marker reader. Do not restamp only because committing changed HEAD.

Live downstream migration is separate. Fixed in-repository corpora/private SDK
copies can validate consumer contracts without accessing excluded live checkouts.
A later cutover must review manifest, generated files, snapshot approvals, wrappers,
ownership checker and testkit together; old compile success does not imply new
check/update compatibility.

## Verification and consequences

Preserve baseline/API/wire/input evidence; semantic layout/public compile/sparse
coverage; independent-render/locality/order stability; shared strict marker/tree
cases; resource bounds; update/cancel/race/rollback/durable-cleanup recovery;
Safe/full fixed-corpus generation and goldens; final aggregate and relevant Anvil.
Negative cases include duplicate keys, unknown/self records, case collisions,
legacy markers, short unknown files, empty directories, symlinks/FIFOs/swaps,
read growth/same-size mutation and all file/byte/depth/marker limits. Testkit rejects
forbidden macro/path input even with updated hashes. Large event/tuple/typed-data
owners and near-file-limit output remain covered. Generic counting failure matrices
belong in standards under ADR0020. Independent review and unexecuted host/downstream
qualification are recorded separately, never inferred from implementation reports.

More files and ownership metadata improve navigation and local diffs without new
public modes. First cutover requires explicit review; it does not authenticate
supplier identity or compiler provenance.


## 2026-11-09 publication-fixture amendment

The former captured application corpus and checkout-aware extraction, planner
and live-consumer helpers are retired. The repo-owned
[ABI corpus](../../external-acceptance/abi-corpus-v1/README.md) retains 25 interfaces
and their entry, overload, nesting, array, width and indexed-event coverage, with
at least 133,404 ABI input bytes. Synthetic inputs do not claim compiler or
external-revision provenance. Source inventories bind their actual bytes.

Generate and exactly verify SDK output through the normal CLI, then compile
standalone consumers and run the retained ownership/resource rejection tests.
Compiler-derived event, EIP-712 and Safe vectors remain separate authorities
with their original source, compiler and hash observations. Historical acceptance
cannot qualify the new inputs. No live application checkout is a test dependency.
