//! Human and JSON renderers over one stable diagnostic fact model.

use musubi_codegen::{Diagnostic, Severity};

use crate::args::OutputFormat;

pub(crate) fn print(format: OutputFormat, diagnostics: &[Diagnostic]) {
    match format {
        OutputFormat::Human => {
            for diagnostic in diagnostics {
                let location = diagnostic
                    .location
                    .as_ref()
                    .map_or_else(String::new, |value| format!(" [{value}]"));
                eprintln!(
                    "{} {}{}: {}",
                    severity(diagnostic.severity),
                    diagnostic.code.as_str(),
                    location,
                    diagnostic.message
                );
            }
        }
        OutputFormat::Json => {
            let document = serde_json::json!({ "diagnostics": diagnostics });
            eprintln!(
                "{}",
                serde_json::to_string(&document).unwrap_or_else(|_| "{\"diagnostics\":[]}".into())
            );
        }
    }
}

const fn severity(value: Severity) -> &'static str {
    match value {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}
