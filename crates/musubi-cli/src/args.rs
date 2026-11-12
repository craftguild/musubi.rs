//! Closed manifest-only consumer CLI.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Deterministic Musubi generation from external ABI and bytecode artifacts.
#[derive(Debug, Parser)]
#[command(name = "musubi", version, propagate_version = true)]
pub(crate) struct Cli {
    /// Closed versioned generator manifest.
    #[arg(long, default_value = "musubi.toml", global = true)]
    pub manifest: PathBuf,
    /// Diagnostic rendering format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Human, global = true)]
    pub format: OutputFormat,
    /// Treat degradation warnings as failure; informational notices remain informational.
    #[arg(long, global = true)]
    pub deny_warnings: bool,
    /// Requested operation.
    #[command(subcommand)]
    pub command: Command,
}

/// CLI operations.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Print the exact embedded `CycloneDX` software bill of materials.
    Sbom,
    /// Exclusively create an editable config-v4 manifest.
    Init,
    /// Create the configured standalone SDK only if its output is absent.
    Generate,
    /// Atomically replace an existing marker-owned SDK at the configured output.
    Update,
    /// Check generated files byte-for-byte without writing.
    Check,
    /// Generate temporarily and validate with bounded `cargo check --locked`.
    Doctor,
    /// Write a canonical compatibility snapshot.
    Snapshot(SnapshotArgs),
    /// Classify two canonical compatibility snapshots.
    Diff(DiffArgs),
}

/// Human or machine-readable diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum OutputFormat {
    /// One concise stable-code diagnostic per line.
    Human,
    /// One JSON document containing the same diagnostic facts.
    Json,
}

/// `snapshot` arguments.
#[derive(Debug, Args)]
pub(crate) struct SnapshotArgs {
    /// Canonical snapshot JSON destination.
    #[arg(long)]
    pub out: PathBuf,
}

/// `diff` arguments.
#[derive(Debug, Args)]
pub(crate) struct DiffArgs {
    /// Older canonical snapshot.
    pub old: PathBuf,
    /// Newer canonical snapshot.
    pub new: PathBuf,
}
