<!-- craftguild:common:start -->
Use Craftguild standards 0.1.0 with the fixed engine in standards/engine.json.
Run `make prime` (or `python3 standards/launcher.py prime`) before adoption.
Read relevant topics with `make prime PRIME_ARGS="--topic NAME"`.
Follow the repo-local craftguild-standardize Skill. Run `make check`.
Update explicitly with `craftguild-standards init [--force]`; follow its dependency revision guidance.
<!-- craftguild:common:end -->

## Shared agent files

`AGENTS.md` is the repository instruction authority. The two shared skills in
`.agents/skills/` cover Beads and the pinned Craftguild entry. Tool-specific
`.claude/`, `.codex/`, `.cursor/` and editor settings are local and ignored.
Beads data and configuration stay entirely local in ignored `.beads/`; do not
publish or synchronize them. Run `bd prime` for the local task workflow.

# Project agent and review rules

These rules apply throughout this repository. Deeper instructions may add
constraints. Read the architecture and Accepted ADRs in `docs/design/` before changing
product contracts. Resolve implementation/design drift in the same change.

## Scope and trust boundaries

Musubi generates Rust SDKs from external ABI, bytecode and optional AST inputs.
The product and generator never run, download, configure or pin solc, Hardhat or
Foundry. The explicit CI-only Foundry exception in ADR 0020 provisions isolated
Anvil tests on the explicitly configured ephemeral CI runners; ordinary Make operations do not acquire it. Actual
consumer migrations require a separate accepted decision; do not modify other
products as a side effect. Use fixed in-repository compatibility fixtures without
accessing an excluded checkout, environment or service.

ABI owns wire encoding. AST only enriches names and Rust type identity. Never
change selectors/topics/encoding through enrichment or claim common compiler
provenance for separately supplied inputs. Generated SDK APIs expose owned types,
not Alloy types, `sol!` output or selector strings. Reject traversal, symlink
escapes and replacement of unowned directories. First-party crates forbid unsafe;
a necessary exception requires an accepted decision before implementation.

## Build and quality ownership

Follow ADR 0020 and the fixed common engine. Run `make setup` explicitly when the
cache is absent, prepare locked dependencies, then read `make prime`. Ordinary
operations never install a toolchain, update a pin or acquire another engine.
Product hooks own SDK/ABI/artifact boundaries; common preparation, compilation,
receipts, installation, release algorithms and measurement remain in standards.
Do not recreate generic build regression matrices here.

Use Rust 1.95.0 for normal builds, tests, CI and generation; all first-party
packages and generated SDKs declare minimum Rust 1.95. Preserve historical
producer observations and use the normal verified SDK lifecycle for cutovers.

Implementation is limited to 400 **Code** lines measured by tokei 15.0.0. Tests,
test-only helpers, fixtures, ADRs and design documents are excluded by actual
purpose. Mixed Rust test regions use the common parser. Generated Rust exceptions
require complete inventory/hash validation and isolated canonical reproduction.
Unmeasured implementation is not a pass. Never edit generated files to repair
checks; change the generator or reviewed input and regenerate explicitly.

## Rust documentation and iteration

Each crate/module explains its responsibility with `//!`; public types, traits,
fields, methods, variants and constants have useful `///` documentation. Document
non-obvious ownership, safety and wire invariants, including private code. Public
crates warn on missing documentation.

Start with focused tests in the changed crate, then package format/test/Clippy.
Run the full aggregate at a final checkpoint rather than after each edit. Inspect
failures before applying formatting or snapshots. After a legitimate fixture
update, immediately run its exact generation check.

## Final validation and evidence

`make check` reaches workspace and facade tests, format, Clippy, rustdoc, strict
SDK/artifact ownership, exact generation, product acceptance fixtures and both Git
diff whitespace checks. `make acceptance` checks the actual selected CLI, closed
GeneratorInfo, exact SBOM and an owned temporary installation. Keep generated
Cargo/test output separate from the verified CLI and retained SDK inventories.

Product scenarios and assertions added or changed by this adoption live in Rust
tests. Scripts only prepare resources/context and invoke commands. The opt-in
CLI artifact test runs explicitly through `make acceptance`; an ignored default
result is not acceptance evidence. Infrastructure tests retain their own language
and adopted frameworks retain their standard test tooling. Do not add a helper
crate or general test runner, or broaden this into unrelated legacy rewrites.

ABI/codec/deployment/Safe changes need their golden and differential checks.
Alloy adapter changes also require explicit Anvil integration. Local Anvil is
externally provisioned. CI alone may prepare the pinned Foundry under ADR 0020;
tests use only owned local Anvil processes, never real RPC/accounts/services.
Record unavailable hosts or services and
their responsible CI job separately from passing checks. Linux/macOS are the
supported operating systems; Windows CI and Windows operation are excluded.

Before handoff inspect `git status --short`, identify important untracked files,
and record current-source evidence and remaining scope in Beads. Acceptance of an
ADR, prior results or a configured `prime` is not current qualification. Do not
create a Markdown task/review ledger. Preserve existing user work and the index;
follow the active goal's restrictions on staging, signing, release and publication.

## Design changes

Record durable decisions in `docs/adr/` and synchronize the architecture. Keep
ADR numbers, status and supersession links. Never delete an accepted decision just
to replace it with an implementation plan. Archived review and evaluation records
remain historical; implementation reports do not become independent review.
