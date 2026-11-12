//! Human and JSON rendering of one compatibility report model.

use musubi_codegen::{CompatibilityCategory, CompatibilityReport};

use crate::args::OutputFormat;

pub(crate) fn print(format: OutputFormat, report: &CompatibilityReport) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string(report).unwrap_or_else(|_| "{\"changes\":[]}".into())
            );
        }
        OutputFormat::Human if report.is_empty() => println!("compatible: no classified changes"),
        OutputFormat::Human => {
            for change in &report.changes {
                let scope = change.contract.as_deref().unwrap_or("<package>");
                println!("{}: {scope}", category(change.category));
            }
        }
    }
}

const fn category(category: CompatibilityCategory) -> &'static str {
    match category {
        CompatibilityCategory::ArtifactKindChanged => "artifact-kind-changed",
        CompatibilityCategory::TypedDataBreaking => "typed-data-breaking",
        CompatibilityCategory::TypedDataAdditive => "typed-data-additive",
        CompatibilityCategory::EvmWireBreaking => "evm-wire-breaking",
        CompatibilityCategory::EvmWireAdditive => "evm-wire-additive",
        CompatibilityCategory::RustSourceBreaking => "rust-source-breaking",
        CompatibilityCategory::RustSourceAdditive => "rust-source-additive",
        CompatibilityCategory::DeploymentIdentityChanged => "deployment-identity-changed",
        CompatibilityCategory::ProvenanceOnly => "provenance-only",
    }
}
