# Development and validation

Use Rust/Cargo 1.95.0, rustfmt, Clippy, a native linker, Make, Git, Python 3.11+,
curl and tokei 15.0.0. Source-build engine hosts are macOS arm64 and Linux x86_64
GNU. Runtime Linux/macOS support is distinct from host qualification.

Explicitly prepare the pinned standards v0.6.1 engine and locked dependencies:

```sh
make setup
cargo +1.95.0 fetch --locked
cargo +1.95.0 fetch --locked --manifest-path tests/fixtures/consumer-resolution/Cargo.toml
make prime
make build
```

Normal operations use the existing engine/tool/dependency cache without downloads
or updates. `PROFILE=debug`, `TARGET` and `CARGO_TARGET_DIR` select the artifact;
outputs follow `<target-dir>/<target-triple>/<debug|release>/musubi`.

## Product checks

```sh
make check
make acceptance PROFILE=debug
make acceptance PROFILE=release
make installer INSTALLER_ARGS=--check
```

The ordinary check runs product tests, format, Clippy, rustdoc, complete SDK
ownership and exact recorded-producer replay. Local Anvil integration needs
externally provisioned Anvil. CI uses GitHub-hosted `ubuntu-24.04` and
`foundry-rs/foundry-toolchain` at `dba66c27052e63476d886f22216e119ba7e615bf`
with Foundry `v1.9.0` and `cache: false`. This CI preparation is the only Foundry
acquisition exception; the product, generator and ordinary Make operations do not
acquire it. The job checks `command -v anvil` and `anvil --version`, then runs:

```sh
cargo +1.95.0 test --offline --locked -p musubi-alloy --test anvil -- --ignored
cargo +1.95.0 test --offline --locked -p musubi-safe --test safe_v141_anvil -- --ignored
```

Both suites use owned loopback Anvil processes and synthetic test accounts, with
no real RPC, accounts or existing services. A skipped or waiting job is not a pass.

Acceptance explicitly runs Rust against the verified built command with an empty
runtime PATH, temporary HOME and temporary copy-only install. An ignored ordinary
Cargo result is not acceptance.

`make install BINDIR=/absolute/bin` copies a matching verified artifact without
building. Release/package configuration is preparation for a later authorized
publication; it is not evidence of an existing release. See
[ADR 0020](../adr/0020-craftguild-standards-adoption.md).

## Generated output

Do not hand-edit an SDK or restamp its recorded producer. Use the normal
manifest-only generate/check/update/doctor lifecycle. Preserve the complete closed
inventory and canonical byte reproduction. A change in generator requirements may
need an explicit new generated output; it does not rewrite historical compiler
observations. The [architecture](../design/architecture.md) owns those contracts.

Record source, toolchain/host/profile, actual commands, failures and unrun scope
in Beads. A local pass does not establish hosted CI, another OS, independent
review or clean-HEAD distribution qualification.
