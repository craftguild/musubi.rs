//! Adversarial bounded-process cleanup and output tests.

use std::{
    path::Path,
    time::{Duration, Instant},
};

use rustix::{
    io::Errno,
    process::{Pid, test_kill_process},
};

use crate::{ProcessFailure, ProcessLimits, ProcessRequest, run_bounded_process};

fn request<'a>(arguments: &'a [&'a str]) -> ProcessRequest<'a> {
    ProcessRequest {
        program: "sh",
        arguments,
        current_dir: Path::new("."),
        environment: &[],
    }
}

#[test]
fn bounded_process_times_out_and_reaps_term_ignoring_group() {
    let started = Instant::now();
    let failure = run_bounded_process(
        request(&[
            "-c",
            "trap '' TERM; (trap '' TERM; while :; do sleep 30; done) & wait",
        ]),
        ProcessLimits {
            timeout: Duration::from_millis(300),
            output_bytes: 1024,
        },
    )
    .unwrap_err();
    assert_eq!(failure, ProcessFailure::Timeout);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn bounded_process_kills_grandchild_after_direct_child_exits() {
    let output = run_bounded_process(
        request(&[
            "-c",
            "(trap '' TERM; while :; do sleep 30; done) & printf done; exit 0",
        ]),
        ProcessLimits {
            timeout: Duration::from_secs(2),
            output_bytes: 1024,
        },
    )
    .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"done");
}

#[test]
fn bounded_process_kills_grandchild_that_closed_inherited_pipes() {
    let root = tempfile::tempdir().unwrap();
    let pid_file = root.path().join("pid");
    let output = run_bounded_process(
        ProcessRequest {
            program: "sh",
            arguments: &[
                "-c",
                "(trap '' TERM; exec >/dev/null 2>&1; while :; do :; done) & echo $! > \"$PID_FILE\"; exit 0",
            ],
            current_dir: Path::new("."),
            environment: &[("PID_FILE", &pid_file)],
        },
        ProcessLimits {
            timeout: Duration::from_secs(2),
            output_bytes: 1024,
        },
    )
    .unwrap();
    assert!(output.status.success());
    let raw_pid = std::fs::read_to_string(pid_file)
        .unwrap()
        .trim()
        .parse::<i32>()
        .unwrap();
    let pid = Pid::from_raw(raw_pid).unwrap();
    assert_eq!(test_kill_process(pid), Err(Errno::SRCH));
}

#[test]
fn bounded_process_rechecks_output_after_direct_child_exit() {
    let failure = run_bounded_process(
        request(&["-c", "printf x"]),
        ProcessLimits {
            timeout: Duration::from_secs(2),
            output_bytes: 0,
        },
    )
    .unwrap_err();
    assert_eq!(failure, ProcessFailure::OutputLimit);
}

#[test]
fn bounded_process_terminates_on_combined_output_limit() {
    let failure = run_bounded_process(
        request(&["-c", "while :; do printf x; printf y >&2; done"]),
        ProcessLimits {
            timeout: Duration::from_secs(3),
            output_bytes: 4096,
        },
    )
    .unwrap_err();
    assert_eq!(failure, ProcessFailure::OutputLimit);
}
