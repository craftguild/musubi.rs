//! Private generated-crate validation using bounded locked Cargo processes.

use musubi_codegen::{
    CargoValidationFailure, Diagnostic, DiagnosticCode, GeneratedTree, OutputTarget,
    ProcessFailure, UNSUPPORTED_HOST_MESSAGE,
};

pub(crate) fn run(output: &OutputTarget, tree: &GeneratedTree) -> Result<(), Vec<Diagnostic>> {
    match musubi_codegen::doctor(tree, output) {
        Ok(()) => Ok(()),
        Err(CargoValidationFailure::Preparation) => {
            Err(single("doctor private Cargo validation cannot be prepared"))
        }
        Err(CargoValidationFailure::Process(failure)) => Err(process_failure(failure)),
        Err(CargoValidationFailure::Status) => Err(single("doctor Cargo command failed")),
    }
}

fn process_failure(failure: ProcessFailure) -> Vec<Diagnostic> {
    let message = match failure {
        ProcessFailure::Start => "doctor Cargo command could not start",
        ProcessFailure::UnsupportedPlatform => UNSUPPORTED_HOST_MESSAGE,
        ProcessFailure::Timeout => "doctor Cargo command exceeded 300 seconds and was terminated",
        ProcessFailure::OutputLimit => {
            "doctor Cargo output exceeded 8 MiB and the process was terminated"
        }
        ProcessFailure::Wait => "doctor Cargo process could not be reaped",
        ProcessFailure::Read => "doctor Cargo output could not be read",
        ProcessFailure::Cleanup => "doctor Cargo process tree could not be cleaned before deadline",
    };
    single(message)
}

fn single(message: &str) -> Vec<Diagnostic> {
    vec![Diagnostic::error(DiagnosticCode::ProcessFailure, message)]
}

#[cfg(all(test, not(unix)))]
mod tests {
    use super::*;

    #[test]
    fn unsupported_host_failure_has_the_stable_redacted_doctor_diagnostic() {
        let error = process_failure(ProcessFailure::UnsupportedPlatform);
        assert_eq!(error[0].code, DiagnosticCode::ProcessFailure);
        assert_eq!(error[0].message, UNSUPPORTED_HOST_MESSAGE);
    }
}
