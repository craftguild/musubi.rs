//! Redacted parse failures using the same diagnostic facts as command execution.

use crate::{
    args::{Cli, OutputFormat},
    diagnostics,
};
use clap::{CommandFactory as _, Parser as _, error::ErrorKind};
use musubi_codegen::{Diagnostic, DiagnosticCode};
use std::{ffi::OsString, process::ExitCode};

pub(crate) fn parse(arguments: &[OsString]) -> Result<Cli, ExitCode> {
    if arguments.len() == 1 || matches!(&arguments[1..], [flag] if flag == "--help" || flag == "-h")
    {
        return Err(crate::version::exit(crate::version::help(
            &Cli::command().render_help().to_string(),
        )));
    }
    match Cli::try_parse_from(arguments) {
        Ok(cli) => Ok(cli),
        Err(error) if error.kind() == ErrorKind::DisplayVersion => Err(crate::version::exit(
            crate::version::print(presentation(arguments)),
        )),
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            Err(crate::version::exit(error.print()))
        }
        Err(_) => {
            diagnostics::print(
                presentation(arguments),
                &[Diagnostic::error(
                    DiagnosticCode::InvalidInput,
                    "invalid command-line input; use --help for the supported manifest-only commands",
                )],
            );
            Err(ExitCode::from(2))
        }
    }
}

// Only presentation is recovered from invalid argv. Values and parser error text are never echoed.
fn presentation(arguments: &[OsString]) -> OutputFormat {
    let mut format = OutputFormat::Human;
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        if argument == "--" {
            break;
        }
        let value = if argument == "--format" {
            arguments.next().and_then(|value| value.to_str())
        } else {
            argument
                .to_str()
                .and_then(|argument| argument.strip_prefix("--format="))
        };
        match value {
            Some("json") => format = OutputFormat::Json,
            Some("human") => format = OutputFormat::Human,
            _ => {}
        }
    }
    format
}
