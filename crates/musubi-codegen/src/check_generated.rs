//! Retained-snapshot check that replays only valid recorded generator metadata and compares all bytes.

use musubi_core::GeneratorInfo;

use crate::{CheckReport, GenerateError, GenerationIr, OutputTarget};

/// Checks current inputs and renderer using the existing SDK's recorded generator information.
///
/// Reads the actual tree once through retained directory handles. All expected Rust, JSON, assets,
/// and inventory bytes are compared; no fields are masked. A different checking generator is an
/// informational observation, not authentication, and this function never updates the recording.
///
/// # Errors
///
/// Rejects invalid metadata, unsafe output, unsupported hosts, or invalid current rendering inputs.
pub fn check_generated(
    ir: &GenerationIr,
    current: &GeneratorInfo,
    output: &OutputTarget,
) -> Result<CheckReport, GenerateError> {
    crate::host_capability_preflight()
        .map_err(|_| crate::output::io_error(crate::UNSUPPORTED_HOST_MESSAGE))?;
    current
        .validate()
        .map_err(|_| crate::output::io_error("checking generator metadata is invalid"))?;
    crate::render_validation::validate(ir, current)?;
    #[cfg(unix)]
    {
        let retained = output.resolve()?;
        let Some((parent, name)) = retained.existing() else {
            return Ok(crate::output::missing_report(&crate::render(ir, current)?));
        };
        let Some(actual) = crate::output_fs::read_tree_at(parent.descriptor(), name, ir.limits)?
        else {
            return Ok(crate::output::missing_report(&crate::render(ir, current)?));
        };
        let marker = actual
            .get(std::path::Path::new(crate::GENERATED_MARKER))
            .ok_or_else(|| crate::output::unsafe_error("existing generation marker is missing"))?;
        crate::parse_ownership_marker(marker, ir.limits)
            .map_err(crate::output_fs::ownership_error)?;
        let release = actual
            .get(std::path::Path::new("musubi-release.json"))
            .ok_or_else(invalid_metadata)?;
        let recorded = recorded_generator(release)?;
        let expected = crate::render(ir, &recorded)?;
        let mut report = crate::output::compare(&expected, &actual)?;
        if &recorded != current {
            let recorded = serde_json::to_string(&recorded).map_err(|_| invalid_metadata())?;
            let current = serde_json::to_string(current).map_err(|_| invalid_metadata())?;
            report.notices.push(crate::Diagnostic::info(
                crate::DiagnosticCode::GeneratorInfoDiffers,
                format!("recorded generator differs from checking CLI; recorded={recorded}; current={current}"),
            ));
        }
        Ok(report)
    }
    #[cfg(not(unix))]
    {
        let _ = output;
        Err(crate::output::io_error(crate::UNSUPPORTED_HOST_MESSAGE))
    }
}

#[cfg(unix)]
fn recorded_generator(bytes: &[u8]) -> Result<GeneratorInfo, GenerateError> {
    let release = crate::closed_json::parse(bytes).map_err(|()| invalid_metadata())?;
    if release
        .get("schema_version")
        .and_then(serde_json::Value::as_str)
        != Some("musubi/release-v4")
    {
        return Err(invalid_metadata());
    }
    let value = release.get("generator").ok_or_else(invalid_metadata)?;
    let generator: GeneratorInfo =
        serde_json::from_value(value.clone()).map_err(|_| invalid_metadata())?;
    generator.validate().map_err(|_| invalid_metadata())?;
    Ok(generator)
}

#[cfg(unix)]
fn invalid_metadata() -> GenerateError {
    crate::output::io_error(
        "recorded generator metadata is missing, invalid, or uses an unsupported schema",
    )
}
