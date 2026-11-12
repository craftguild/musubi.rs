# ADR 0016: Lightweight validation and generator build identity

## Status

Accepted — 2026-09-06. Design acceptance is separate from implementation and
qualification. Supersedes ADR0008 and amends distribution/runtime/Cargo-environment/
schema/completion portions of ADR0003/0005/0010/0011/0013/0014. Manifest-only
lifecycle, retained-handle I/O, process deadlines, ADR0015 recovery and wire
contracts remain. ADR0017 subsequently changes layout/ownership; ADR0020 replaces
local build/delivery/measurement implementation with the common engine. Historical
plans/reviews are hash-preserved privately; current qualification is tracked in Beads.

## Context

A bespoke publisher SSH/fresh-checkout/empty-cache/double-release/bundle pipeline
made ordinary development depend on credentials, commit/push and repeated builds.
Removing a script alone could not fix a CLI that required packaged runtime identity.
A normally built CLI should work while recording its actual producer honestly.
Those observations are neither signatures nor binary authentication.

## Decision

### 1. One format, quality and product-test checkpoint

Start with focused tests, then one final aggregate of format-check, implementation
quality, Clippy/rustdoc, exact generation/diff checks and full ordinary product
suites, including relevant externally provisioned Anvil scenarios. Do not repeat
identical suites for historical command lists or count unavailable scenarios as
passing. ADR0020's tokei15 implementation-only Code convention replaces the old
physical/test-inclusive size rule.

Actual registry publication, delivery clean builds, tags, commits and credentials
are separate from local product completion. Normal CLI/SDK compilation remains
required; two cold release builds are not a normal product gate.

### 2. Remove bespoke distribution management from the product

Retire publisher-only package/fetch/build/bundle workflows, hidden commands,
special environment variables/validators and their exclusive tests. Keep no alias,
second operating mode or flag restoring them. Preserve input boundaries, ownership,
Safe artifact checks and codecs. ADR0020 now supplies shared SBOM, installer,
receipt and release declarations without restoring the former private pipeline.

Trust the developer/CI's ordinary Cargo/Git/SSH configuration, credentials, wrappers,
toolchains and reusable caches as a separate boundary. Do not inject a product SSH
capability, force a transport, empty CARGO_HOME or claim that normal acquisition
was isolated/authenticated. Implicit URL/path/source fallback remains forbidden.
Library Git use does not require cargo publish. Never copy secrets, environment
dumps or raw Git/Cargo/SSH errors into provenance or redacted diagnostics.

Trusted acquisition does not weaken untrusted input/output: retain Unix no-follow
handles, bounded rendering/reads, markers, atomic installation, rollback/recovery
and process-tree deadlines. Current operating support is Linux/macOS; no Windows
extension is introduced.

### 3. Manifest-owned explicit runtime

Only closed config-v4 owns package/contract/typed-data/limits/output/runtime. No
CLI/environment override or manifest-supplied generator identity is permitted.
Required runtime is exactly one of:

```toml
[runtime]
source = "git"
url = "https://github.com/craftguild/musubi.rs.git"
revision = "<reviewed full nonzero lowercase 40-digit commit>"
```

```toml
[runtime]
source = "path"
root = "."
```

Git allows ordinary Cargo SSH/HTTPS URLs and SSH users, but no passwords/tokens,
HTTPS usernames, query/fragment or controls. Reject zero/short/branch/tag/dirty/
unknown revisions. Core/runtime use the same source and full revision. Path roots
are explicitly mutable, manifest-relative and placement-dependent; only they
permit `.`/`..`. Reject absolute paths, controls and ambiguous separators. Resolve
fixed core/runtime subpaths, rebase generated Cargo dependencies relative to the
SDK and never write into the runtime root. Temporary compile copies alone may
rebase to absolute paths. Artifact/output confinement is unchanged.

The runtime tree is trusted ordinary Cargo input, not recursively captured or
authenticated by Musubi. Use the generated ABI guard and compilation, not an
inferred equality of runtime/generator versions. Consumer lockfiles own the final
graph. Retire Development/Bundled/Released promotion modes and implicit runtime
sources. The retained Safe SDK uses ordinary explicit Path configuration.

Init exclusively creates a deterministic template with commented runtime examples;
it infers neither source nor revision. A valid runtime and at least one input must
be enabled before generation. Existing-entry protection remains.

### 4. Private Cargo validation with ordinary caches

Materialize validated SDK bytes into private temporary source while keeping the
source guard alive through process completion. Never reopen the original SDK.
Generate/update install only after compilation; doctor never modifies configured
output; exact check starts neither Cargo nor network.

Use ordinary Cargo settings/cache, with repository or manifest root as trusted
configuration base and an explicit absolute private manifest. Do not force cwd=/
or clear the whole environment. Honor explicitly selected home/target caches,
reject overlap with the SDK and choose reusable targets outside it by default.
Private lockfiles and cache effects never mutate the consumer/owned SDK inventory.
Retain bounded output, a 300-second whole-group deadline and cleanup/reaping.
Relocated inspection explicitly carries conceptual SDK and trusted configuration
roots so Path retains its meaning.

### 5. Capture actual CLI build observations

Normally built CLI metadata must not depend on publishing, tags, special secrets
or commit/push. ADR0020 makes common prepared builds the supported source route;
capture once at build time, not from runtime cwd/Git/environment. The closed
GeneratorInfo is:

| Field | Meaning |
| --- | --- |
| schema_version | musubi/generator-info-v1 |
| version | Generator package version |
| revision | Full commit, full commit-dirty or unknown |
| git_commit | Full unsuffixed commit or null |
| dirty | false/true when known, otherwise null |
| build.rustc | Actual compiler version |
| build.host | Actual build host triple |
| build.target | Executable target triple |
| build.profile | Captured profile |

Known commit and status must both exist to assert clean/dirty. Include staged,
unstaged, nonignored untracked and submodule changes across the repository, not
only CLI paths. Linked worktrees/detached HEAD work. Unavailable Git/HEAD/status
gives coupled nulls and unknown, without adopting an enclosing consumer's HEAD.
Refresh observations after source/index/HEAD changes without mandatory cargo clean
or recompiling all dependencies. Git subprocesses are bounded and failures redact
raw output. Shared human/SBOM observations remain independent; the product adapter
preserves this closed, coupled SHA1 record and the separately captured actual host.

Human and JSON version work offline before manifest/runtime/Cargo/network. No
capture timestamp, username, hostname, absolute path or environment dump is added.
The shared human presentation may omit the separately retained host field.

### 6. Record the actual SDK producer

Every SDK exposes generator() returning literal GeneratorInfo, equal to generating
CLI JSON and release JSON. Do not recapture at SDK build/runtime. Unknown remains
honest unknown; pure renderer tests may inject explicit fixture identity, but no
production override fabricates provenance. Runtime source and producer remain
separate, as do manifest/input hashes, wire goldens and unverified coherence.
Different dirty sources may share a display; this is not authentication.

### 7. Determinism, exact check and self-reference

Same binary/manifest/input/runtime yields identical bytes. Different compiler/
host/profile/dirty observations may legitimately differ. No generated timestamps
are added. Generate/update always stamp the current CLI. Check snapshots and
validates the retained producer, rerenders current inputs with it, compares every
byte and reports a different checking producer informationally with both identities.

Reject inconsistent Rust/JSON metadata, invalid/missing fields/schema and actual
input/runtime/renderer/file drift. Never mask fields, hand-edit metadata or restamp
SDKs just because committing changed HEAD. Fully consistent hostile metadata is
not authenticated. Generator-only snapshot differences are provenance-only.

### 8. Pre-production clean break

Only config-v4, release-v4, snapshot4 and GeneratorInfo-v1 remain; generated-runtime
ABI becomes2 for the public provenance contract. EVM selectors/encoding/EIP-712/
Safe semantics do not change. Retire old readers, packaged-runtime branches,
hidden commands, old variables and compatibility shims; small rejection tests and
historical ADRs may remain. Old SDK migration is explicit preserved-output
cutover into an absent destination, never markerless overwrite. Safe/corpus/fixtures
use normal identity rules; live consumer migration is a separate task.

## Acceptance and deferred work

Require ordinary CLI/path-runtime init/generate/update/check/doctor, closed schema
and override rejection, runtime/producer independence, offline literal metadata,
recorded replay and drift rejection, cache reuse, all product ownership/resource/
process/recovery/Safe/wire tests and one final aggregate. Generic Git/build-state
regression matrices moved to standards under ADR0020; retain actual product
adaptation/artifact evidence instead of a duplicate consumer matrix. Fixed corpus
acceptance never authorizes modifying a live consumer checkout.

Actual registry permissions/order, clean-HEAD delivery build, release/tag/signing,
retained assets and publication remain separate authorized operations. Shared
SBOM/installer/release configuration under ADR0020 is current integration work,
not evidence those operations happened. Do not add empty pipelines or treat
publication credentials as a missing local product implementation. Unexecuted
hosts/remote credential acquisition remain unqualified, not passing.
