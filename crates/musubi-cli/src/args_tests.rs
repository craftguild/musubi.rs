//! Consumer command parsing and complete rejection of generation override surfaces.

use super::args::{Cli, Command};
use clap::{CommandFactory as _, Parser as _};

#[test]
fn lifecycle_has_no_generated_sdk_configuration_arguments() {
    for command in ["init", "generate", "update", "check", "doctor"] {
        assert!(
            Cli::try_parse_from(["musubi", command]).is_ok(),
            "missing {command}"
        );
        for flag in [
            "--abi",
            "--bytecode",
            "--creation-bytecode",
            "--runtime-bytecode",
            "--ast",
            "--metadata",
            "--storage-layout",
            "--contract-id",
            "--out",
            "--force",
            "--override",
            "--development-runtime-path",
            "--bundled-runtime",
            "--runtime-git-url",
            "--runtime-git-revision",
            "--package-name",
            "--package-version",
            "--typed-data",
            "--limits",
            "--output",
        ] {
            assert!(
                Cli::try_parse_from(["musubi", command, flag, "injected"]).is_err(),
                "accepted {command} {flag}"
            );
            assert!(Cli::try_parse_from(["musubi", &format!("{flag}=injected"), command]).is_err());
        }
    }
}

#[test]
fn help_lists_the_lifecycle_and_rejects_removed_publisher_validation() {
    assert!(Cli::try_parse_from(["musubi", "validate-release-identity"]).is_err());
    let help = Cli::command().render_long_help().to_string();
    for command in [
        "init", "generate", "update", "check", "doctor", "snapshot", "diff",
    ] {
        assert!(help.contains(command));
    }
    assert!(!help.contains("validate-release-identity"));
    for name in ["init", "generate", "update", "check", "doctor"] {
        let mut cli = Cli::command();
        let command = cli.find_subcommand_mut(name).unwrap();
        for argument in command.get_arguments() {
            assert!(matches!(
                argument.get_id().as_str(),
                "help" | "version" | "manifest" | "format" | "deny_warnings"
            ));
        }
    }
}

#[test]
fn report_destination_and_presentation_controls_remain_available() {
    let cli = Cli::try_parse_from([
        "musubi",
        "--manifest",
        "chosen.toml",
        "--format",
        "json",
        "--deny-warnings",
        "snapshot",
        "--out",
        "report.json",
    ])
    .unwrap();
    assert!(matches!(cli.command, Command::Snapshot(_)));
}
