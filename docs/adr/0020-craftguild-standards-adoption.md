# ADR 0020: Shared build execution and Craftguild standards

## Status

Accepted — 2026-11-06, for the requested local standards adoption.
Updated — 2026-11-07, for v0.5.0 integration in the working tree.
Updated — 2026-11-08, for uniform Rust 1.95 and Rust product acceptance tests.
Current qualification and remaining scope are recorded in Beads.
Acceptance of this design does not claim implementation, platform qualification,
independent review or publication.

## Decision

Use normative Craftguild rules 0.1.0 with published execution engine v0.5.0 at
`69ef0ba960e4d61bff3f363c07f8d49c4d53d77e`. This replaces the earlier v0.2.0
integration target; old implementation and results are historical, not v0.5.0
qualification. Follow published standards
[ADR 0006](https://github.com/craftguild/standards.rs/blob/69ef0ba960e4d61bff3f363c07f8d49c4d53d77e/docs/adr/0006-initializer-cli-and-controlled-engine-upgrades.md).
After preserving and explicitly retiring the legacy pin, copied rules, archives
and loaders, run the matching `craftguild-standards init` at the repository root.
It creates reference schema 3 in `standards/engine.json`, the thin launcher/Make
entry, local Skill and common AGENTS block, and caches the full
`craftguild-standards-engine` under `.craftguild/engine/`.

Init preserves product Make/config and Cargo/npm inputs. Complete configuration
schema 2 and align common Rust dependencies to that full HTTPS revision manually.
Include `standards/make/common.mk`; Make/CI and direct launcher operations use the
same fixed engine. Explicit `make setup` restores only its pinned cache after
cloning, without a global initializer or engine. `make prime` reads offline
embedded rules. Ordinary operations do not acquire/update the engine or fall
back to a sibling checkout. Do not retain another pin authority or bundle loader.

The shared engine owns build context, dependency inclusion, SBOM preparation,
input seals, compilation, output verification, receipts, copy-only installation
and Conventional Commits release preparation. Make delegates to that engine.
Product glue must not reconstruct those operations or edit prepared metadata.
Retain focused product development commands and actual product checks.

The latest human decision replaces the former Rust 1.85 compatibility policy
and intervening 1.86 proposal. Normal builds, tests, CI, canonical generation and
all first-party SDK/package requirements use Rust 1.95.0 with declared minimum
Rust 1.95. The fixed standards release retains its own compatibility statement.
Enable `rust_output_layout = "standard"` and remove Rust output overrides.
Default output is `target/<host-triple>/release/musubi`; debug, explicit target
and target-directory selection use `<target-dir>/<target-triple>/<profile>/<file>`
through the same shared resolver. Update product SDK acceptance and documentation
callers without an old-path fallback. Installation retains `PREFIX`, `BINDIR`
and `DESTDIR`, defaults to `/usr/local/bin`, and copies only a matching verified
binary. Toolchains must already exist. No Solidity compiler or Foundry acquisition
is added. ADR 0019's local Make implementation is replaced; Cargo is selected
through the common toolchain and trusted PATH rather than a product compiler wrapper.

The repository lock selects the coherent Alloy 1.0.35 family with core crates
1.2.0 to resolve the private
Serde API incompatibility in the previous 0.12.6 family. The upstream
[1.0.35 release](https://github.com/alloy-rs/alloy/releases/tag/v1.0.35) includes
the Serde fix, and its
[workspace manifest](https://github.com/alloy-rs/alloy/blob/v1.0.35/Cargo.toml)
defines the corresponding core family. Keep the fixed standards/Serde graph,
owned public SDK types, runtime ABI 2 and wire contracts. Do not add a second
Serde version, vendor patch or public Alloy exposure to bypass the failure.
Actual isolated Anvil integration remains required for adapter changes.
Reusable core/runtime support dependencies retain compatible minimum ranges;
consumer lockfiles remain independent and the newer consumer-resolution fixture
must still compile. Repository graph selection does not impose exact support
dependency versions on generated SDK consumers.

Root no-argument/help/version presentation and `sbom` use the common CLI library
before manifest loading, discovery, configuration or subprocesses. Root branding
includes the fixed banner and centered `craftguild/musubi.rs` affiliation.
Subcommand help omits the large banner. Failed output is a command failure.
`--format json --version` remains the closed GeneratorInfo object, with no banner
or SBOM envelope. Adapt common captured identity into the existing product record,
retaining the actual build host separately. Generated SDKs retain their recorded
producer; checking a different current build does not restamp them.

One release group owns the eight workspace package versions and root lockfile.
Their current version remains 0.1.0. The generated Safe SDK's package version,
recorded producer and explicit runtime source remain independent authorities.
No release algorithm is copied into this repository. Root source-only Path
dependencies must not impose redundant exact versions that become inconsistent
when the workspace authority advances. Registry publication is a separate task.
Shared release policy uses breaking/feat → MINOR and fix → PATCH while below
1.0.0, breaking → MAJOR from 1.0.0, and `--major` for an explicit major request.
Use common installer declarations/generation and receipt-backed packaging where
applicable, without adding a downloader or packaging engine here. Actual release
and clean-HEAD delivery packaging remain separate from local integration.

## CI-only Anvil preparation amendment (2026-11-11)

The initial source release requires both existing Alloy and Safe Anvil integration
tests to pass on GitHub-hosted `ubuntu-24.04`, replacing the self-hosted runner.
CI may obtain Foundry `v1.9.0` through the official
[`foundry-rs/foundry-toolchain`](https://github.com/foundry-rs/foundry-toolchain)
Action fixed at `dba66c27052e63476d886f22216e119ba7e615bf`, with `cache: false`.
The version is the official [v1.9.0 release](https://github.com/foundry-rs/foundry/releases/tag/v1.9.0).
This narrowly amends the no-acquisition rule above for CI environment preparation.
The product, generator and ordinary Make operations never acquire Foundry.

The job verifies Anvil's command path and version and explicitly runs the existing
`musubi-alloy --test anvil` and `musubi-safe --test safe_v141_anvil` suites with
`--ignored`, retaining all cases and assertions. Default-suite ignores or a waiting
runner do not satisfy this gate. Tests own their loopback Anvil processes and
synthetic accounts; they never connect to real RPC, accounts or existing services.
No continuously operated runner or additional CI service is introduced.

## Preserved product contracts

Keep config-v4, release-v4, generated-v2, semantic-layout-v1, GeneratorInfo-v1 and
generated-runtime ABI 2. ABI remains the wire authority; optional AST enriches
names and Rust type identity only. Preserve Alloy isolation, reviewed Safe inputs,
closed schemas, resource budgets, retained-handle I/O, process deadlines, atomic
update/recovery and redacted diagnostics. No legacy readers or production metadata
overrides are introduced.

The registered retained SDK remains bound to its manifest, Cargo identity,
release record and complete closed ownership inventory. Common generated-size
exceptions additionally require current-input canonical reproduction into an
isolated empty directory. Use its recorded validated producer to reproduce every
byte; do not copy the retained output as reproduction, mask metadata, modify a
marker to repair it or substitute the current CLI's producer.

The new SDK minimum is a real generation-input change. Before its cutover,
validate the complete old SDK and its exact recorded-producer replay, then
preserve all old bytes and provenance. Use the normal manifest-owned generation
or update lifecycle. If strict current admission rejects the old minimum, move
the validated old tree aside and use normal create-only generation at its
original placement. Validate complete new ownership and exact replay afterward.
Do not hand-edit generated Cargo, Rust, release records or markers, or restamp
only to make the current check pass.

## Quality and verification ownership

The common implementation-only Code convention replaces physical/test-inclusive
400-line rules in AGENTS.md, ADR 0016/0017 and active guides. Tests, test-only
helpers, fixtures, ADRs and design documents are excluded by their actual role.
Mixed Rust files require the shared syntax-aware separation. Missing or unmeasured
implementation is a failure. Generated exceptions remain conditional on complete
ownership, hashes and canonical reproduction.

Standards owns reusable build, metadata observation, entry, install, release and
measurement regression tests. Remove duplicate consumer build suites, including
the repeated Git/cache build-identity matrix and generic validation-CWD harness.
Do not move product implementations, named fixtures or renamed copies into
standards. Retain actual builds, real CLI/machine-output/SBOM acceptance and product
ABI, ownership, artifact, generation, consumer, golden and resource tests.

Product scenarios and assertions added or changed by this adoption use Rust's
ordinary test tooling in the owning component. The CLI artifact acceptance is an
opt-in Rust integration test of the verified engine-built executable. Its script
only verifies the common receipt, selects its immutable preparation/toolchain/
target/profile and invokes Cargo. Use a separate test target directory so Cargo
cannot replace the delivered executable. `make acceptance` explicitly runs the
test; default-suite ignored results do not qualify acceptance. Retain the copied
offline CLI, closed GeneratorInfo, actual host, exact SBOM, output-failure,
no-write and temporary installation assertions. Retire the bespoke Python
acceptance report instead of introducing another report or testing framework.
Existing infrastructure tests retain their own implementation language; adopted
frameworks retain their standard tooling. Unrelated legacy scenarios are not a
repository-wide rewrite requirement. No runtime command or helper crate is added.

Existing source-only SDK ownership and reviewed-artifact validation remain product
checks; replace only their generic physical-line measurement. Keep the relevant
Anvil jobs. The user's subsequent scope decision removes Windows CI and Windows
from Musubi's operating and acceptance criteria. Supported build/runtime hosts
are Linux and macOS; standards has no Windows support plan. A native Windows
inventory-path investigation is consequently outside this adoption's remaining
work, rather than a required fix or successful qualification. Defensive rejection
of unsupported hosts may remain in product code. Unix cross-compilation does not
establish native Windows support. Report unavailable and failing supported-host
checks separately from local passing checks.

## Documentation, licensing and operational boundaries

Public prose and agent guidance use English. Adopt the shared README outline and
documentation index while retaining ADR numbers, statuses and supersession.
Preserve evaluation originals and original history with hashes in the ignored
private archive; public builds and tests never read that archive or review state.
Keep first-party dual licensing and all third-party rights and notices.

## Local integration completion

The local completion criteria are:

| Criterion | Completion evidence |
| --- | --- |
| I1 — Fixed identity | v0.5.0 reference, managed entry/Skill/common block and common-crate full revision agree. |
| I2 — Connected operations | Make/CI and complete product configuration use the fixed engine; legacy common files/loaders are retired. |
| I3 — Standalone workflow | Isolated intended public source supports explicit setup, offline prime and actual build without siblings, private archives or a global engine. |
| I4 — Actual artifacts | Rust 1.95 release/debug artifacts use standard paths; exact SBOM, offline CLI/GeneratorInfo and temporary copy-only install succeed. |
| I5 — Product quality | Final `make check`, actual builds, applicable acceptance and complete retained-SDK ownership/canonical reproduction succeed; preserve ABI and relevant Alloy/Anvil checks. |
| I6 — Maintained delivery | Documentation, licenses, dependencies, implementation Code measurement and declared installer/release settings match. |
| I7 — Current evidence | Record final source/index basis, pin, commands, profiles, hosts and deferred qualification; required missing/failing local gates block completion. |
| I8 — Execution boundary | Local completion leaves real release, clean-HEAD packaging, staging, commits and publication for separate authorization. |

Init/prime or the earlier partial v0.2.0 build is not integration completion.
Use existing product checks and one final-source pass; add tests only for a
concrete uncovered product connection risk. Do not reproduce common failure
matrices. Report unavailable additional supported-host qualification separately;
Windows host/CI is excluded by the existing scope decision. A source export may
include intended uncommitted work, with its Git identity reported honestly.

## Authorization and technical provenance

Build and release runs are transient technical operations. Immutable receipts and
Git provide their provenance. They add no persistent business entity, state
transition table, database migration or Header-Detail aggregate. Existing SDK
transaction and recovery boundaries remain unchanged.

This decision defines the integration contract; Beads tracks current
execution scope and ordered product milestones. Maintaining this ADR or the goal
prompt does not itself resume, pause or certify implementation. Authorized local
implementation and validation do not stage,
commit, tag, push, publish, run an actual release preview/application, change
product versions or release notes, install into a user's/system directory, or
operate existing services. Necessary fixture commits use isolated temporary
OpenPGP keys and full-revision signature verification. A real signing or
verification failure stops the entire adoption goal. The actual excluded product
checkout and environment are outside the investigation and test scope.

## Published v0.6.0 adoption amendment (2026-11-11)

The user authorized the standards authentication release and explicit adoption
for this initial CLI delivery. Upgrade through the published v0.6.0 initializer
to full revision `72cf076962dc8401b2e4d1d84933c8a818ff390c`, then align the
common Rust Git declarations and lockfile with that same revision. This replaces
the v0.5.0 pin in the current identity/completion criteria above; earlier decisions,
results and actual generator-producer records keep their original provenance.
Normative baseline 0.1.0, reference schema 3, configuration schema 2 and both
compatibility protocols remain unchanged. Rust 1.95.0 and all product gates stay
required. Init preserves product recipes, configuration and local task state.

The shared release removes its implicit aggregate command deadline while retaining
explicit watchdogs, cancellation and owned-child cleanup. This does not establish
a diagnosis for an earlier hosted-runner shutdown. Requalify the current product
source, actual artifacts and native CI rather than reuse older-source passes.
Ship the CLI for Linux x86_64 GNU and macOS arm64 through CI-built Release assets,
with target checksums and SBOMs, a generated installer and README instructions.
Verify real download/install/uninstall in owned temporary destinations.

The public CLI installer retains the default curl transport. CLI delivery does
not publish npm/crates registry packages or qualify additional SDK/OS payloads.

The active release request separately authorizes signed commits/tags, pushes and
GitHub publication. Preserve the original local-integration boundary as history;
this amendment does not claim any unexecuted qualification or introduce a business
lifecycle, transition-history table or Header-Detail model.

## 2026-11-11 runner and license amendment

The owner authorized WarpBuild runners for every workflow, including native CLI
delivery and installed-asset acceptance. Linux uses Ubuntu 24.04 x86_64 and
macOS uses macOS 26 arm64 on ephemeral WarpBuild VMs. This replaces earlier
GitHub-hosted-only restrictions; explicit tool provisioning, isolated processes,
product gates and deadlines remain in force. No local service is provisioned by
this decision. First-party code is now MIT OR Apache-2.0, at the recipient's
option; third-party licenses and historical producer observations retain their
original scope.

## 2026-11-11 renamed standards release

The owner authorized adoption of published `craftguild/standards.rs` v0.6.1 at
`6852b210a9a614920bc5be2187f2705f46db978d`. The actual published initializer re-created the
managed entry after its previous owned bytes were verified and archived.
The engine and common Cargo crates use that same full revision. Product
configuration and instructions remain intact. No old repository-name fallback
is provided. The normative baseline stays 0.1.0; product Rust stays 1.95.

## 2026-11-13 standards v0.6.2 release preparation

Use published standards v0.6.2 at `be63eefb44a850a01f4cc065c66fff2fe558ee18` through the verified initializer.
The fixed engine reference and common Cargo Git dependencies select that same full revision, with rules baseline 0.1.0 and product Rust 1.95.
This amendment aligns the shared release preparation across CraftGuild products while preserving earlier decisions and historical producer observations.
Declare independent Cargo path-consumer lockfiles in `releases.cargo.additional_lockfiles` so normal release preparation synchronizes their selected local package versions.
