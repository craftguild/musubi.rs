//! Minimal product-validation entry point for generated-crate conformance inspection.

use std::{path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let mut arguments = std::env::args_os();
    let _program = arguments.next();
    let Some(root) = arguments.next().map(PathBuf::from) else {
        eprintln!("generated crate path is required");
        return ExitCode::from(2);
    };
    if arguments.next().is_some() {
        eprintln!("exactly one generated crate path is required");
        return ExitCode::from(2);
    }
    if musubi_testkit::inspect_generated_crate(&root).is_err() {
        eprintln!("generated crate validation failed");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
