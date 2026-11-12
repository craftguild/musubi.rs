# ADR 0019: Make build and local CLI installation

## Status

Accepted — 2026-10-08, for the requested Make build and install interface.

## Decision

- `make` and `make build` compile the release `musubi` executable from package `musubi-cli`
  with `cargo build --locked --release`. Cargo retains ordinary user configuration and toolchain
  selection. The default target directory is `target`.
- `CARGO` and `CARGO_TARGET_DIR` are configurable. `TARGET` selects a target triple and defaults
  to `CARGO_BUILD_TARGET`; the executable is located in the corresponding release directory.
- `make install` copies that prebuilt executable with mode `0755`. It invokes no Cargo command,
  so installation with elevated write permissions does not compile as root.
- The default destination is `/usr/local/bin/musubi`. `PREFIX`, `BINDIR` and `DESTDIR` follow the
  sibling repositories: `BINDIR` must be absolute; `DESTDIR` must be empty or absolute.
  `PREFIX="$HOME/.local"` provides installation into the user's own directory.
- A missing or non-executable build is rejected with guidance to run `make build` using matching
  target settings. A directory named `musubi` at the destination is rejected. Installation stages
  a private file beside the destination, then replaces the executable through a same-directory rename.
- The Makefile is handwritten source and is covered by the mechanical 400-line budget gate.

## Validation

Run the real default and explicit release builds, install into a private prefix, compare installed
bytes/mode and execute human/JSON version output. Also check target/target-directory selection,
DESTDIR, spaces in paths, installation without Cargo, missing builds, invalid install paths and
destination-directory refusal. System-wide installation is performed by the operator.

### Executed checkpoint — 2026-10-08

- Native `make` and `make build` succeeded with GNU Make 3.81, Rust 1.85.0 and
  `aarch64-apple-darwin`. Both produced the release executable. Evidence:
  `target/musubi-make-build.log`.
- Ten installation/interface checks passed, including a real private prefix containing spaces,
  exact installed bytes and mode `0755`, human/JSON version with an empty PATH, replacement of
  existing files/symlinks, DESTDIR, BINDIR, ambient target selection and custom target directories.
  `CARGO=false` was used during installation to prove installation does not build.
- Missing binaries, relative install paths and destination directories were rejected. Symlink
  targets and directory contents were preserved; temporary install files were removed.
  Explicit cross-target build flags were checked through a dry run, not a cross-compilation.
  Evidence: `target/musubi-make-install-verification.log` and `target/musubi-make-verification.json`.
- The handwritten gate covers Makefiles and `.mk` fragments. Its existing 401-line adversarial
  test now exercises each filename variant; the focused 11-test suite and current 312-file gate
  passed. Evidence: `target/musubi-make-budget-focused.log`.
- The rename's full workspace/fixture/Anvil checkpoint is recorded in ADR 0018. This subsequent
  change modifies the Make interface, documentation and source-only budget checks; Rust sources
  remain the tested rename candidate. Generated SDK exact checks, fmt and both diff checks were
  repeated after the Make addition.
- During the work, HEAD advanced externally to `79214c817494b78cd61d58ddbea71b242a936b42`,
  which tracks the renamed crates/manifests. This task performed no staging or committing.
  The added Makefile and this ADR are untracked at handoff; installation used only private prefixes.
