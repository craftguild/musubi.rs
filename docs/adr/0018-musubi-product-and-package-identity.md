# ADR 0018: Musubi product, CLI and package identity

## Status

Accepted — 2026-10-08, following the user's choice of `musubi.rs` and request for a complete rename.
This amends names in the accepted design; its encoding, trust, lifecycle and resource contracts remain in force.

## Decision

- Product identity is **Musubi**, displayed as **musubi.rs**. The standalone executable is `musubi`;
  its Cargo package and source directory are `musubi-cli`. It is not a Cargo subcommand.
- The consumer facade is `musubi`. Components are `musubi-core`, `musubi-runtime`, `musubi-codegen`,
  `musubi-alloy`, `musubi-safe` and `musubi-testkit`. The retained generated package is
  `musubi-safe-v141-generated`. Rust imports use the corresponding underscore spelling.
- Default configuration is `musubi.toml`; generated ownership and provenance use `.musubi-generated`
  and `musubi-release.json`. Project-owned schema and hash-domain namespaces use `musubi/`.
  Existing structure/version numbers are retained: config-v4, release-v4, generated-v2,
  semantic-layout-v1, generator-info-v1 and generated-runtime ABI 2.
- Diagnostic codes use `MSB-`; repository/environment integration names use `MUSUBI_`.
  Product parsers support only the Musubi identities. Old CLI/crate aliases and dual schema readers
  are not added to this pre-production rename.
- Root help, invocation without arguments and human version output share one `musubi.rs` ASCII banner.
  It uses Small Slant and embeds the generated asset without a build/runtime
  dependency. ADR 0020 replaces the original Node.js generation with the fixed
  standards engine's renderer and font data.
  Version JSON remains the closed GeneratorInfo object without human presentation text;
  command diagnostics and generated metadata remain machine readable.
- Generated source is regenerated from reviewed input artifacts, not patched. ABI, bytecode,
  Solidity evidence, selectors, topics, encodings and typed-data values remain byte-identical.
  Project-owned acceptance envelopes are regenerated while retaining their compiler-derived payloads.
- Current source, tests, scripts, CI, documentation and examples adopt Musubi names. Historical
  command/type examples are updated to the new spelling; historical revisions, judgments and logs
  retain their original attribution and do not become evidence for this rename.

## Migration boundaries

The previous identities were `cargo-evm-sdk`, `evm-sdk-*` / `evm_sdk_*`, `evm-sdk.toml`,
`.evm-sdk-generated`, `evm-sdk-release.json`, the `evm-sdk/` namespace, `EVS-` and `EVM_SDK_`.
They may appear only as explicit migration history or isolated legacy-fixture input handling.
The observed pre-rename source remote was `ssh://git@github.com/craftguild/evm-sdk.rs.git`.
Musubi URLs in updated examples do not assert that the remote repository was renamed or fetched.
Consumers must update manifests, dependencies/imports and regenerate an SDK with Musubi together.
Legacy SDKs are retained before replacement and are not accepted as current owned output.

Live application repositories are outside this decision. A private-copy acceptance adapter
may translate captured legacy inputs, recording that translation rather than claiming unchanged
input identity. Live consumer cutover is a separate task requiring its own Accepted ADR.
No repository remote, domain registration, publication, stage, commit or push is part of this rename.

## Validation

Focused CLI/version and codec/codegen tests precede retained SDK regeneration and immediate exact check.
The final checkpoint includes the full validation pipeline, branded help/human/JSON version,
standalone generation/consumer compilation, goldens, ownership/recovery, related Anvil tests,
400-line inspection and a scan for unintentional old names. Pinned compiler/vendor bytes and sibling
repository states are compared with the protected pre-rename inventory.

### Executed rename checkpoint — 2026-10-08

- Baseline: clean HEAD `d1c74d6433e6a2c48cd3fea4affe84c23413dab6`.
  This task performed no staging or committing.
- `scripts/validate.sh` passed, including workspace fmt/test/Clippy/rustdoc, all fixture/consumer
  checks, canonical ABI budget regression, goldens, ownership/recovery and both diff checks.
  Evidence: `target/musubi-final-validation.log`.
- Both ignored Anvil integrations passed when explicitly invoked: Alloy deployment/read/write/event
  round trip and every supported Safe signature form. Evidence: `target/musubi-anvil-alloy.log`
  and `target/musubi-anvil-safe.log`.
- Node.js FIGlet Small Slant generation and root/no-argument help, both human version flags,
  version JSON and all seven subcommand help pages were checked. Version remains capability-free.
- All 361 externally locked dependencies and 38 pinned compiler/vendor files are unchanged.
  The six generated Safe ABI/bytecode assets are byte-identical. Ten schema/vector/provenance
  envelopes preserve semantic payloads; only schema labels and two Safe vector-command spellings
  changed. All 389 non-SDK baseline files resolve at their renamed paths.
- The fixed semantic-owner digest vector was independently recomputed for the Musubi hash domain.
  The validation-runner fixture's stale script count was corrected to include the existing canonical
  ABI regression; directory/configuration behavior and failure propagation checks passed.
- Safe SDK and original ABI corpus envelopes were regenerated and immediately checked, with original
  output retained under the ignored `target/musubi-rename-baseline-049ri6ox` baseline directory.
  Original ABI, bytecode and compiler-derived corpus payloads were not edited.
- The macOS filesystem rejects non-UTF-8 names; that build-state case was skipped and is assigned
  to Linux CI job `test`. Windows-only fail-closed checks belong to CI job `unsupported-host`.
  Live consumer migration and live SSH/HTTPS acquisition were outside this checkpoint.

The subsequent Make build/install addition is recorded separately in ADR 0019.
