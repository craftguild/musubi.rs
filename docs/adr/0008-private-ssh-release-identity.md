# ADR 0008: Private SSH Release Identity

## Status

Superseded by [ADR 0016](0016-lightweight-validation-and-generator-build-identity.md).
The decisions below are historical, not pending acceptance gates. Dedicated SSH capability injection,
package construction, and release authorization are no longer required product workflows.

## Context

ADR 0006 required a public HTTPS Git repository for released generator/runtime identity. The
reviewed lab02 source is distributed from a private Git repository instead of crates.io or a public
repository. Treating a development path, an HTTPS fallback, or an scp-like locator as equivalent
would make the dependency identity depend on unrecorded host configuration.

The immutable revision, fresh-clone validation, reproducible release binary, and generated-crate
identity requirements remain unchanged. SSH credentials and host trust are explicit CI/operator
capabilities and are not part of the generated artifact.

## Decision

- Released generation accepts only canonical `ssh://user@host/path` repository locators and a
  reviewed, non-zero, lowercase 40-digit Git revision.
- The historical lab02 distribution locator, expressed in the current Musubi example spelling, is
  `ssh://git@github.com/craftguild/musubi.rs.git`. The scp-like spelling
  `git@github.com:craftguild/musubi.rs.git` is not a canonical release identity.
  This name update does not record a Git remote change; see ADR 0018 for the original observation.
- A released generated `Cargo.toml`, generated release manifest, generated Rust provenance, and
  package provenance retain the exact same SSH locator and revision.
- crates.io publication and public HTTPS access are not release prerequisites.
- Development generation remains an explicitly selected relative path and cannot be promoted to a
  release identity.
- Release validation rejects non-SSH schemes, scp-like syntax, local paths, whitespace or control
  characters, URL passwords/tokens, query strings, fragments, empty user/host/path, placeholder
  locators, zero revisions, uppercase revisions, and branch or tag names.
- Release packaging uses non-interactive SSH with password and keyboard-interactive authentication
  disabled and strict host-key checking enabled. It never falls back to HTTPS or a local path.
- CI/operator environments must provision an ssh-agent or reviewed deploy key, pinned
  `known_hosts`, non-interactive SSH, and repository read permission before packaging or consumer
  dependency resolution. `MUSUBI_RELEASE_KNOWN_HOSTS_FILE` identifies the reviewed host-key file
  by absolute path. An operator may instead identify a deploy key by absolute
  `MUSUBI_RELEASE_SSH_IDENTITY_FILE`; no implicit identity file is loaded.
- Package validation forces Cargo's Git CLI transport through the same SSH/config boundary and
  runs `doctor` for a released generated crate. This proves exact-revision dependency resolution
  and standalone compilation instead of relying only on generated-manifest string inspection.
- Release packaging creates a new absolute temporary `CARGO_HOME` before its first Cargo command
  and uses it for fresh validation and both reproducibility builds. Released generated-crate
  resolution uses a second empty Cargo home. Ambient user Cargo configuration, source replacement,
  wrappers, and caches are outside both boundaries.
- Before the first Cargo invocation, packaging removes inherited `CARGO`, every `CARGO_*`
  override, `RUSTC`, `RUSTDOC`, their flags and wrappers, and `RUSTC_BOOTSTRAP`. It then installs
  only the package-owned Cargo homes, target directories, reproducibility flags, the repository's
  pinned Rust 1.85.0 toolchain, and pinned Git CLI transport required by each step. An inherited
  `RUSTUP_TOOLCHAIN` cannot replace that reviewed toolchain.
- Every packaging Cargo process uses `/` as its current directory and an absolute manifest path.
  Repository, checkout, caller, and temporary-directory ancestor `.cargo/config.toml` files cannot
  enter identity validation, reproducibility builds, consumer checks, or SBOM metadata collection.
- Authentication failures and remote Git/SSH errors are reported only as stable redacted failures.
  Private keys, credentials, agent details, and raw remote errors are not written to diagnostics,
  logs, SBOM, or provenance. The repository locator and revision are non-secret release identity.
- The release gate requires an exact fetch from the canonical locator, revision equality, full
  validation in the fresh checkout, byte-identical fresh/current release binaries, and preserved
  locator/revision in generated dependencies and provenance.
- A generated crate consumed internally by the same source distribution uses the closed
  `bundled` runtime identity and fixed `../musubi-core` and `../musubi-runtime` sibling paths.
  It does not claim a self-referential Git revision. Packaging rejects `development` identity in
  this production dependency, exact-checks it in the fresh checkout, and binds all bundled bytes
  through the enclosing Git revision and source-archive digest. Externally generated release
  crates continue to require the exact private SSH identity.
- Every tracked release input must be a regular file. Packaging rejects tracked symlinks even when
  current and fresh checkouts contain the same link text, because a build can follow host bytes
  that a Git source archive records only as a link.
- Release packaging disables Thin LTO and builds fresh/current binaries in distinct temporary
  target roots before comparing their raw bytes. This prevents temporary build paths from changing
  Mach-O UUID and ad-hoc signature bytes; package provenance records the disabled LTO setting.

## Consequences

- A consumer fresh clone needs the documented SSH capability before Cargo can resolve released SDK
  dependencies.
- A source commit is not a usable release identity until that exact commit is available from the
  recorded SSH locator and the package-release gate succeeds.
- Generated SDK review can distinguish immutable private distribution from development paths
  without relying on machine-local URL rewrite rules.

## Rejected Alternatives

### Keep public HTTPS as a prerequisite

The reviewed source is intentionally distributed from a private repository, so public access is
not part of the product contract.

### Accept scp-like SSH syntax

It is ambiguous to generic URL validators and makes canonical identity comparison harder.

### Rewrite HTTPS or local paths to SSH through Git configuration

The generated manifest would no longer describe the transport actually used, and a fresh clone
could silently resolve a different source.
