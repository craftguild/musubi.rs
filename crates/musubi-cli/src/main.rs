//! Command-line entry point for deterministic SDK generation and validation.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod args;
#[cfg(test)]
mod args_tests;
mod compatibility_output;
mod diagnostics;
mod doctor;
mod parse;
mod run;
mod snapshot_io;
mod version;

fn main() -> std::process::ExitCode {
    match parse::parse(&std::env::args_os().collect::<Vec<_>>()) {
        Ok(cli) => run::execute(&cli),
        Err(code) => code,
    }
}
