# ADR 0013: Handle-Bound Artifact I/O And Conformance Snapshots

## Status

Accepted. Supersedes ADR 0007. Partially superseded by ADR 0016 only for the Cargo environment and
dependency-source boundary: fresh Cargo home/target, `/` cwd, compiler override removal, and
released-only conformance are no longer required. Private source snapshots, handle-bound input/output,
bounded process-tree control, and unsupported-host rejection remain required. Explicit runtime Path
dependencies are caller-authorized normal Cargo inputs, not recursively authenticated snapshots.

## Context

ADR 0007 made generated-tree replacement and process lifecycle Unix-only, but classified manifest
loading as platform-independent. Compiler artifacts, typed-data schemas, vectors, and generated
crate conformance checks have the same time-of-check/time-of-use requirement as generated-tree
replacement: metadata validation followed by a pathname reopen does not prove that the bytes came
from the validated filesystem object. A symlink or regular-file swap can change the object between
those operations, while a special file or post-metadata growth can bypass blocking and size
assumptions.

Returning a `cargo check` command that points at the inspected source directory has the same flaw.
Cargo reopens that directory after testkit validation has ended.

## Decision

- Secure compiler-input loading, generated-tree inspection and replacement, generated-crate
  conformance snapshots, `doctor`, and staging Cargo validation are Unix-host operations.
- Generated replacement validates the complete existing tree against the manifest's effective
  file, byte, and depth limits both before and after its backup rename. Every backup, install, and
  rollback rename uses the host's atomic no-replace primitive, so a concurrently created target is
  preserved and causes failure instead of being overwritten. Old-tree cleanup is handle-relative
  and applies the same limits; cleanup failure retains the sibling backup and is reported.
- The manifest parent is opened once as a directory capability. The manifest and every referenced
  ABI, bytecode, AST, metadata, storage-layout, typed-data schema, and vector component are opened
  relative to retained directory handles with no-follow semantics. Only normal relative path
  components and regular final files are accepted.
- File kind and initial size are obtained from the opened descriptor. Reads are bounded on that
  same descriptor, so file growth after metadata inspection cannot bypass per-file or aggregate
  limits. Nonblocking open prevents FIFO inputs from hanging validation.
- Generated-crate conformance inspection snapshots all accepted regular-file bytes by the same
  handle-relative bounded traversal. Marker, required-file, and release-manifest validation consume
  only that snapshot and never reopen the source path.
- The testkit Cargo helper materializes validated released-crate bytes into a fresh private
  temporary source directory, uses separate fresh target and Cargo-home directories, and returns a
  guard that must outlive Cargo. Before execution, a closed Cargo-manifest parser requires the exact
  package and released Git/revision dependency identities recorded by the release manifest. Only
  generator-owned source/asset path shapes are accepted; build scripts, implicit binary targets,
  Cargo configuration, workspaces with members, and path dependencies fail closed. Rust source
  scanning normalizes raw identifiers before rejecting `include!`, `include_str!`, environment
  macros, and `path` attributes; only the exact inventoried `include_bytes!` assets are accepted.
  Development
  path identities are rejected because relocating those paths changes their meaning, while
  retaining them would reopen filesystem state outside the snapshot.
- The Cargo helper removes inherited Cargo and compiler wrapper/flags overrides, including
  `RUSTUP_TOOLCHAIN`, from its child command; it sets only its private Cargo home/target, the
  repository's pinned Rust 1.85.0 toolchain, and the Git CLI transport requirement. It invokes
  Cargo from `/` and passes an absolute private manifest path. Neither the inspected/generated path
  nor a temporary-directory ancestor can inject `.cargo/config.toml`. Caller SSH capability
  remains explicit and is not copied into the source snapshot.
- A real top-level `target` directory may be excluded from conformance snapshots only after it has
  been opened without following links and identified from its descriptor. A link or special entry
  named `target` is rejected.
- Native process-group lifecycle and the operation-wide capability preflight from ADR 0007 remain
  required. A host without equivalent safe handle-relative and process-tree primitives fails
  closed; it does not fall back to metadata-plus-pathname logic.
- Windows or other non-Unix support requires equivalent safe directory-handle traversal, bounded
  same-handle reads, and process-tree control, recorded by an ADR that supersedes this one.

## Consequences

Input parsing and public generated-crate conformance loading now share the Unix security boundary
with generation and checking. The accepted bytes remain stable even if their original pathnames
are replaced after open. Released-crate Cargo conformance is isolated from later source-tree
mutation. Cross-platform pure transformations remain possible after trusted bytes have been loaded,
but filesystem-loading entry points fail closed on unsupported hosts.

## Required Verification

- Replace an input pathname with an outside-root symlink after its descriptor is opened and prove
  the loader reads only the already-opened file.
- Grow the same inode after metadata inspection and prove the descriptor-bounded read rejects it.
- Prove FIFO and other special entries fail without blocking.
- Apply the same swap, growth, special-entry, target-link, depth, count, and aggregate-byte cases to
  generated-crate snapshots.
- Race a marker-less directory into an initially absent generated target and prove atomic install
  preserves it; grow a backed-up tree past effective limits and prove cleanup fails explicitly.
- Prove the Cargo helper uses `/` as its current directory plus private source, manifest, Cargo
  home, and target paths whose lifetime is owned by the returned guard, not the inspected source
  tree.

## Rejected Alternatives

### Canonicalize, inspect metadata, then reopen by path

Canonicalization and metadata do not bind the later read to the inspected object.

### Keep the original generated source path for Cargo

Cargo performs later pathname opens, recreating the race after conformance inspection.

### Snapshot development path dependencies automatically

Copying or resolving arbitrary relative dependency trees silently expands the conformance trust
boundary and cannot preserve path meaning in a private isolated directory.
