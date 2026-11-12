//! Stable, redacted diagnostics shared by CLI renderers.

use serde::Serialize;

/// Stable machine-readable diagnostic code.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub enum DiagnosticCode {
    /// External ABI/bytecode compiler-invocation coherence is outside the trust boundary.
    #[serde(rename = "MSB-I001")]
    ExternalArtifactCoherenceUnverified,
    /// Recorded generator information differs from the checking CLI.
    #[serde(rename = "MSB-I002")]
    GeneratorInfoDiffers,
    /// Optional AST enrichment input is absent.
    #[serde(rename = "MSB-W001")]
    AstUnavailable,
    /// AST data could not uniquely enrich one ABI-compatible user-defined type.
    #[serde(rename = "MSB-W002")]
    AstEnrichmentUnavailable,
    /// A manifest, artifact, or limit validation failed.
    #[serde(rename = "MSB-E001")]
    InvalidInput,
    /// A configured path escaped its manifest root or used a forbidden form.
    #[serde(rename = "MSB-E002")]
    UnsafePath,
    /// Input or output exceeded a configured hard limit.
    #[serde(rename = "MSB-E003")]
    LimitExceeded,
    /// ABI or optional AST data is internally inconsistent.
    #[serde(rename = "MSB-E004")]
    InconsistentArtifact,
    /// Generated output differs from the expected directory.
    #[serde(rename = "MSB-E005")]
    GeneratedDrift,
    /// A subprocess failed, timed out, or exceeded its output bound.
    #[serde(rename = "MSB-E006")]
    ProcessFailure,
    /// Explicit runtime source was missing or invalid.
    #[serde(rename = "MSB-E007")]
    InvalidRuntimeSource,
}

impl DiagnosticCode {
    /// Returns the stable textual code used by human and JSON output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExternalArtifactCoherenceUnverified => "MSB-I001",
            Self::GeneratorInfoDiffers => "MSB-I002",
            Self::AstUnavailable => "MSB-W001",
            Self::AstEnrichmentUnavailable => "MSB-W002",
            Self::InvalidInput => "MSB-E001",
            Self::UnsafePath => "MSB-E002",
            Self::LimitExceeded => "MSB-E003",
            Self::InconsistentArtifact => "MSB-E004",
            Self::GeneratedDrift => "MSB-E005",
            Self::ProcessFailure => "MSB-E006",
            Self::InvalidRuntimeSource => "MSB-E007",
        }
    }
}

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational trust-boundary notice that warning denial never promotes.
    Info,
    /// Recoverable degradation that `--deny-warnings` may promote.
    Warning,
    /// Failure that prevents the requested operation.
    Error,
}

/// One redacted diagnostic fact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    /// Stable code.
    pub code: DiagnosticCode,
    /// Severity before optional warning denial.
    pub severity: Severity,
    /// Message that contains no secret, source body, or absolute path.
    pub message: String,
    /// Optional manifest-root-relative source location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

impl Diagnostic {
    /// Constructs an informational diagnostic.
    #[must_use]
    pub fn info(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Info,
            message: message.into(),
            location: None,
        }
    }

    /// Constructs a warning diagnostic.
    #[must_use]
    pub fn warning(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            message: message.into(),
            location: None,
        }
    }

    /// Constructs an error diagnostic.
    #[must_use]
    pub fn error(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            location: None,
        }
    }

    /// Attaches a sanitized manifest-relative location.
    #[must_use]
    pub fn at(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }
}

/// Ordered collection of diagnostics returned by generation stages.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    /// Creates an empty collection.
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Adds one fact while preserving deterministic stage order.
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.0.push(diagnostic);
    }

    /// Iterates over diagnostics.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter()
    }

    /// Reports whether an error is present, optionally treating warnings as errors.
    #[must_use]
    pub fn failed(&self, deny_warnings: bool) -> bool {
        self.0.iter().any(|diagnostic| {
            diagnostic.severity == Severity::Error
                || (deny_warnings && diagnostic.severity == Severity::Warning)
        })
    }

    /// Consumes the wrapper into its ordered vector.
    #[must_use]
    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.0
    }
}
