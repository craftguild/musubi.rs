<h1 align="center">musubi.rs</h1>

<p align="center">
  <a href="https://github.com/Essk/ai-contribution-level">
    <img src="https://raw.githubusercontent.com/Essk/ai-contribution-level/main/badges/level-3.svg" alt="AI Contribution">
  </a>
</p>

musubi.rs generates typed Rust SDK crates from externally compiled Ethereum
artifacts and reviewed EIP-712 schemas. ABI defines the wire format; optional
Solidity AST data adds Rust names and type identity. The generated API uses
owned Rust values, keeping Alloy behind the runtime and optional adapters.
Release binaries cover Linux x86_64 GNU and macOS arm64; libraries and other
build targets are consumed from source.

> [!WARNING]
> This project has not reached version 1.0 and may be unstable.
> Its behavior and specifications may change, and upgrades may require manual migration.
> Our team uses it in internal production systems to refine its design and implementation, but it may not meet your requirements.
> Please keep these limitations in mind when using it.

## Quick start

Release binaries are available for Linux x86_64 GNU and macOS arm64 from
[GitHub Releases](https://github.com/craftguild/musubi.rs/releases/tag/v0.1.0).
The installer needs POSIX sh, curl, and `sha256sum` or `shasum`. It verifies the
downloaded checksum and installs to `~/.local/bin` by default.

Install the CLI, then create an editable manifest:

```sh
curl -fsSL https://raw.githubusercontent.com/craftguild/musubi.rs/v0.1.0/scripts/install.sh | sh -s -- --version 0.1.0
BIN="$HOME/.local/bin/musubi"
"$BIN" init
```

`init` exclusively creates `musubi.toml`; it does not overwrite an existing
manifest. Set its exported artifact paths, output and explicit Git/Path runtime
source, then run `"$BIN" --manifest musubi.toml generate`. musubi.rs never runs or
downloads a Solidity compiler and never guesses a runtime revision. See the
[architecture](docs/design/architecture.md) for manifest and artifact examples.

Pass `--bindir /absolute/bin` after `--version 0.1.0` to choose another directory.
Use `--uninstall` instead of `--version 0.1.0` to remove the command; pass the same
`--bindir` for a custom destination. Add that directory to PATH for bare commands.
Each release includes target-specific `SHA256SUMS` files and a CLI SBOM.

## System requirements

Linux or macOS with Unix directory handles and process groups. Windows is outside
the support criteria. Generated SDKs require Rust **1.95+**. Generation, update
and `doctor` need Cargo and the explicitly selected runtime dependencies.

The generator needs read access to input artifacts and write access to its owned
output parent. Offline help, version, embedded SBOM and exact `check` need no
network service. The external compiler pipeline owns ABI/bytecode coherence;
musubi.rs records it as `unverified`.

## Build requirements

Use Rust/Cargo **1.95.0**, a native linker, Git, Make, Python **3.11+** and curl.
The repository pins standards **v0.6.1**; explicit setup restores its verified
engine on macOS arm64 or Linux x86_64 GNU. Normal builds/checks do not obtain an
engine, compiler or dependencies. No global standards command is needed.

Checks also use rustfmt, Clippy, tokei **15.0.0**, ripgrep (`rg`), `jq` and the locked downstream
consumer fixture. Prepare that fixture before the full check:

```sh
cargo +1.95.0 fetch --locked --manifest-path tests/fixtures/consumer-resolution/Cargo.toml
make check
make acceptance PROFILE=debug
make acceptance PROFILE=release
```

Local Anvil integration uses an externally provisioned `anvil`. CI alone prepares
the pinned Foundry toolchain for isolated Anvil tests; the product, generator and
ordinary Make operations do not acquire it. [Development guidance](docs/guides/development.md)
describes this CI exception, normal validation and SDK replay.

Build the CLI from source after preparing the build tools:

```sh
make setup
cargo +1.95.0 fetch --locked
make
```

## Usage

```sh
musubi --manifest musubi.toml generate
musubi --manifest musubi.toml check
musubi --manifest musubi.toml update
musubi --manifest musubi.toml doctor
musubi --manifest musubi.toml snapshot --out current.json
musubi --format json diff previous.json current.json
musubi --format json --version
musubi sbom > musubi.cdx.json
```

`generate` creates an absent output. `check` is read-only and replays its recorded
producer exactly. `update` replaces only valid owned output, and `doctor`
compiles a private inspection copy. CLI flags select the manifest/presentation;
the closed config-v4 manifest owns generation settings.

The `musubi` facade has optional `alloy` and `safe` features. Generated crates
expose operations, clients, events, errors and a typed `GeneratorInfo`.
`musubi-safe` implements the reviewed Safe **1.4.1** contract.
See [documentation](docs/README.md) for ownership, recovery and API decisions.

Build output is `target/<target-triple>/<debug|release>/musubi`.
Use matching `PROFILE`, `TARGET` and `CARGO_TARGET_DIR` selectors for build/install.
`make install BINDIR=/absolute/bin` only copies a verified existing artifact.
The [adoption ADR](docs/adr/0020-craftguild-standards-adoption.md) defines shared
release/installer ownership; local configuration does not publish a release.

## Known limitations

- Supported closed formats are config-v4, release-v4, generated-v2 with
  semantic-layout-v1, GeneratorInfo-v1 and generated-runtime ABI 2. Older formats
  require explicit migration.
- Separately supplied ABI and bytecode are not proven coherent by valid syntax.
- Safe support is limited to 1.4.1. Finality, nonces, fees, signing policy and
  governance remain application/backend responsibilities.
- Path runtimes are mutable and depend on placement. Build observations and
  ownership hashes are not authentication.
- Untested hosts, registry publication and live consumer migrations require
  separate qualification.

## Community

Use [project issues](https://github.com/craftguild/musubi.rs/issues) for ordinary
bugs and proposals. Start with [AGENTS.md](AGENTS.md) and the
[documentation index](docs/README.md) before changing ABI or ownership contracts.
Beads owns current implementation and review evidence.

## Security

Private vulnerability-reporting instructions have not yet been published.
Do not post credentials or sensitive exploit details in public issues.

## License

First-party code is licensed under either the [MIT License](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option.
[NOTICE.md](NOTICE.md) records the separate provenance and terms of retained data.
