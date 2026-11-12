//! Command coordination over codegen's read, IR, render, and output boundaries.

use std::process::ExitCode;

use musubi_codegen::{
    Diagnostic, DiagnosticCode, GeneratedTree, GenerationIr, OutputTarget, Severity,
    WriteOperation, build_ir, build_snapshot, check_generated, diff_snapshots, generate,
    load_inputs, parse_snapshot, render, snapshot_json,
};

use crate::{
    args::{Cli, Command, DiffArgs, SnapshotArgs},
    compatibility_output, diagnostics, snapshot_io,
};

pub(crate) fn execute(cli: &Cli) -> ExitCode {
    let result = match &cli.command {
        Command::Sbom => return crate::version::exit(crate::version::sbom()),
        Command::Init => musubi_codegen::init_manifest(&cli.manifest)
            .map(|()| Vec::new())
            .map_err(|error| error.diagnostics.into_vec()),
        Command::Generate => run_generate(cli, WriteOperation::Create),
        Command::Update => run_generate(cli, WriteOperation::Update),
        Command::Check => run_check(cli),
        Command::Doctor => run_doctor(cli),
        Command::Snapshot(arguments) => run_snapshot(cli, arguments),
        Command::Diff(arguments) => run_diff(cli, arguments),
    };
    match result {
        Ok(diagnostics) => {
            diagnostics::print(cli.format, &diagnostics);
            ExitCode::SUCCESS
        }
        Err(diagnostics) => {
            diagnostics::print(cli.format, &diagnostics);
            ExitCode::FAILURE
        }
    }
}

fn run_snapshot(cli: &Cli, arguments: &SnapshotArgs) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
    let (ir, _, notices) = load_ir(cli)?;
    let bytes = snapshot_json(&build_snapshot(&ir, crate::version::generator()))
        .map_err(|_| snapshot_failure())?;
    snapshot_io::write(&arguments.out, &bytes).map_err(|_| snapshot_failure())?;
    Ok(notices)
}

fn run_diff(cli: &Cli, arguments: &DiffArgs) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
    let old = snapshot_io::read(&arguments.old).map_err(|_| snapshot_failure())?;
    let new = snapshot_io::read(&arguments.new).map_err(|_| snapshot_failure())?;
    let old = parse_snapshot(&old).map_err(|_| snapshot_failure())?;
    let new = parse_snapshot(&new).map_err(|_| snapshot_failure())?;
    compatibility_output::print(cli.format, &diff_snapshots(&old, &new));
    Ok(Vec::new())
}

fn run_doctor(cli: &Cli) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
    let (tree, output, notices) = pipeline(cli)?;
    crate::doctor::run(&output, &tree)?;
    Ok(notices)
}

fn run_generate(cli: &Cli, operation: WriteOperation) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
    let (tree, output, notices) = pipeline(cli)?;
    generate(&tree, &output, operation).map_err(|error| error.diagnostics.into_vec())?;
    Ok(notices)
}

fn run_check(cli: &Cli) -> Result<Vec<Diagnostic>, Vec<Diagnostic>> {
    let (ir, output, mut notices) = load_ir(cli)?;
    let report = check_generated(&ir, crate::version::generator(), &output)
        .map_err(|error| error.diagnostics.into_vec())?;
    notices.extend(report.notices);
    for (kind, paths) in [
        ("added", report.added),
        ("removed", report.removed),
        ("changed", report.changed),
    ] {
        for path in paths {
            notices.push(
                Diagnostic::error(
                    DiagnosticCode::GeneratedDrift,
                    format!("generated file {kind}"),
                )
                .at(path.to_string_lossy()),
            );
        }
    }
    if notices
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        Err(notices)
    } else {
        Ok(notices)
    }
}

fn pipeline(cli: &Cli) -> Result<(GeneratedTree, OutputTarget, Vec<Diagnostic>), Vec<Diagnostic>> {
    let (ir, output, notices) = load_ir(cli)?;
    let tree =
        render(&ir, crate::version::generator()).map_err(|error| single(error.diagnostic))?;
    Ok((tree, output, notices))
}

fn load_ir(cli: &Cli) -> Result<(GenerationIr, OutputTarget, Vec<Diagnostic>), Vec<Diagnostic>> {
    let inputs = load_inputs(&cli.manifest).map_err(|error| error.diagnostics.into_vec())?;
    let output = inputs.output.clone();
    let ir = build_ir(inputs).map_err(|error| error.diagnostics.into_vec())?;
    let notices = ir.diagnostics.clone();
    if notices.failed(cli.deny_warnings) {
        return Err(notices.into_vec());
    }
    Ok((ir, output, notices.into_vec()))
}

fn single(diagnostic: Diagnostic) -> Vec<Diagnostic> {
    vec![diagnostic]
}

fn snapshot_failure() -> Vec<Diagnostic> {
    single(Diagnostic::error(
        DiagnosticCode::InvalidInput,
        "compatibility snapshot operation failed",
    ))
}
