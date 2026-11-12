//! Deadline-bounded subprocess execution with process-group cleanup and bounded output capture.

use std::{path::Path, process::ExitStatus, time::Duration};

#[cfg(unix)]
use std::{
    io::Read,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Instant,
};

#[cfg(unix)]
use rustix::process::Pid;
#[cfg(unix)]
use std::os::unix::process::CommandExt as _;

#[cfg(unix)]
use crate::process_cleanup::{cleanup, cleanup_failure, force_group};

/// Total runtime and combined stdout/stderr limits for one child process tree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessLimits {
    /// End-to-end deadline, including process-group cleanup and pipe-reader completion.
    pub timeout: Duration,
    /// Maximum retained and observed combined stdout/stderr bytes.
    pub output_bytes: usize,
}

/// Normative bounds for Cargo validation launched by generation and doctor.
pub const CARGO_PROCESS_LIMITS: ProcessLimits = ProcessLimits {
    timeout: Duration::from_mins(5),
    output_bytes: 8 * 1024 * 1024,
};

/// Stable diagnostic text when secure host primitives are unavailable.
pub const UNSUPPORTED_HOST_MESSAGE: &str =
    "secure generated-output and process-tree operations are unsupported on this platform";

/// Borrowed subprocess description without shell interpolation.
#[derive(Clone, Copy, Debug)]
pub struct ProcessRequest<'a> {
    /// Executable name or path.
    pub program: &'a str,
    /// Exact argument vector.
    pub arguments: &'a [&'a str],
    /// Child working directory.
    pub current_dir: &'a Path,
    /// Additional path-valued environment variables.
    pub environment: &'a [(&'a str, &'a Path)],
}

/// Successful bounded process output.
#[derive(Debug)]
pub struct ProcessOutput {
    /// Reaped direct-child exit status.
    pub status: ExitStatus,
    /// Captured stdout within the combined bound.
    pub stdout: Vec<u8>,
    /// Captured stderr within the combined bound.
    pub stderr: Vec<u8>,
}

/// Stable bounded-process failure classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessFailure {
    /// The direct child or its required pipes/readers could not be started.
    Start,
    /// Safe process-tree management is unavailable on this platform.
    UnsupportedPlatform,
    /// The end-to-end deadline elapsed.
    Timeout,
    /// Combined output exceeded the configured bound.
    OutputLimit,
    /// The direct child could not be queried or reaped.
    Wait,
    /// A pipe reader failed.
    Read,
    /// Descendant cleanup or pipe-reader completion exceeded the deadline.
    Cleanup,
}

/// Verifies that secure generated-output and process-tree primitives are available.
///
/// # Errors
///
/// Returns [`ProcessFailure::UnsupportedPlatform`] before an operation can mutate the host when
/// the versioned secure host contract cannot be implemented.
pub fn host_capability_preflight() -> Result<(), ProcessFailure> {
    #[cfg(unix)]
    {
        Ok(())
    }
    #[cfg(not(unix))]
    {
        Err(ProcessFailure::UnsupportedPlatform)
    }
}

/// Runs a child in a dedicated process group and cleans every failure path before returning.
///
/// # Errors
///
/// Returns [`ProcessFailure`] for unsupported platforms, spawn, timeout, output, read, wait, or
/// cleanup failures.
pub fn run_bounded_process(
    request: ProcessRequest<'_>,
    limits: ProcessLimits,
) -> Result<ProcessOutput, ProcessFailure> {
    host_capability_preflight()?;
    #[cfg(unix)]
    {
        run_unix(request, limits)
    }
    #[cfg(not(unix))]
    {
        let _ = (request, limits);
        Err(ProcessFailure::UnsupportedPlatform)
    }
}

#[cfg(unix)]
fn run_unix(
    request: ProcessRequest<'_>,
    limits: ProcessLimits,
) -> Result<ProcessOutput, ProcessFailure> {
    let started = Instant::now();
    let deadline = started.checked_add(limits.timeout).unwrap_or(started);
    let reserve = (limits.timeout / 4).min(Duration::from_secs(1));
    let execution_deadline = deadline.checked_sub(reserve).unwrap_or(deadline);
    if Instant::now() >= execution_deadline {
        return Err(ProcessFailure::Timeout);
    }
    let mut child = spawn_child(request)?;
    let group = Pid::from_child(&child);
    let Some(stdout) = child.stdout.take() else {
        return cleanup_failure(&mut child, group, deadline, ProcessFailure::Start);
    };
    let Some(stderr) = child.stderr.take() else {
        return cleanup_failure(&mut child, group, deadline, ProcessFailure::Start);
    };
    let total = Arc::new(AtomicUsize::new(0));
    let exceeded = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel();
    let Ok(stdout_reader) = reader(
        stdout,
        Stream::Stdout,
        sender.clone(),
        Arc::clone(&total),
        Arc::clone(&exceeded),
        limits.output_bytes,
    ) else {
        return cleanup_failure(&mut child, group, deadline, ProcessFailure::Start);
    };
    let Ok(stderr_reader) = reader(
        stderr,
        Stream::Stderr,
        sender,
        total,
        Arc::clone(&exceeded),
        limits.output_bytes,
    ) else {
        let cleanup_result = cleanup(&mut child, group, deadline);
        let reader_result = finish_one(&receiver, stdout_reader, deadline);
        cleanup_result?;
        reader_result?;
        return Err(ProcessFailure::Start);
    };

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(_) => {
                return fail_after_cleanup(
                    &mut child,
                    group,
                    deadline,
                    &receiver,
                    stdout_reader,
                    stderr_reader,
                    ProcessFailure::Wait,
                );
            }
        }
        let failure = if exceeded.load(Ordering::Acquire) {
            Some(ProcessFailure::OutputLimit)
        } else if Instant::now() >= execution_deadline {
            Some(ProcessFailure::Timeout)
        } else {
            None
        };
        if let Some(failure) = failure {
            return fail_after_cleanup(
                &mut child,
                group,
                deadline,
                &receiver,
                stdout_reader,
                stderr_reader,
                failure,
            );
        }
        thread::sleep(Duration::from_millis(10));
    };

    let group_result = force_group(group, deadline);
    let readers = finish_readers(&receiver, stdout_reader, stderr_reader, deadline);
    group_result?;
    let (stdout, stderr) = readers?;
    if exceeded.load(Ordering::Acquire) {
        return Err(ProcessFailure::OutputLimit);
    }
    Ok(ProcessOutput {
        status,
        stdout,
        stderr,
    })
}

#[cfg(unix)]
fn spawn_child(request: ProcessRequest<'_>) -> Result<Child, ProcessFailure> {
    let mut command = Command::new(request.program);
    command
        .args(request.arguments)
        .current_dir(request.current_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    for (name, value) in request.environment {
        command.env(name, value);
    }
    command.spawn().map_err(|_| ProcessFailure::Start)
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum Stream {
    Stdout,
    Stderr,
}

#[cfg(unix)]
type ReadResult = (Stream, Result<Vec<u8>, ()>);

#[cfg(unix)]
fn reader<R: Read + Send + 'static>(
    mut pipe: R,
    stream: Stream,
    sender: mpsc::Sender<ReadResult>,
    total: Arc<AtomicUsize>,
    exceeded: Arc<AtomicBool>,
    limit: usize,
) -> Result<thread::JoinHandle<()>, ()> {
    thread::Builder::new()
        .name("musubi-output-reader".to_owned())
        .spawn(move || {
            let mut retained = Vec::new();
            let mut buffer = [0_u8; 8192];
            let result = loop {
                let count = match pipe.read(&mut buffer) {
                    Ok(0) => break Ok(retained),
                    Ok(count) => count,
                    Err(_) => break Err(()),
                };
                let previous = total.fetch_add(count, Ordering::AcqRel);
                let keep = limit.saturating_sub(previous).min(count);
                retained.extend_from_slice(&buffer[..keep]);
                if previous.saturating_add(count) > limit {
                    exceeded.store(true, Ordering::Release);
                }
            };
            let _ = sender.send((stream, result));
        })
        .map_err(|_| ())
}

#[cfg(unix)]
fn finish_readers(
    receiver: &mpsc::Receiver<ReadResult>,
    stdout_reader: thread::JoinHandle<()>,
    stderr_reader: thread::JoinHandle<()>,
    deadline: Instant,
) -> Result<(Vec<u8>, Vec<u8>), ProcessFailure> {
    let mut stdout = None;
    let mut stderr = None;
    let mut read_failed = false;
    while stdout.is_none() || stderr.is_none() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(ProcessFailure::Cleanup);
        }
        match receiver.recv_timeout(remaining.min(Duration::from_millis(20))) {
            Ok((stream, result)) => {
                let bytes = result.unwrap_or_else(|()| {
                    read_failed = true;
                    Vec::new()
                });
                match stream {
                    Stream::Stdout => stdout = Some(bytes),
                    Stream::Stderr => stderr = Some(bytes),
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(ProcessFailure::Cleanup);
            }
        }
    }
    stdout_reader.join().map_err(|_| ProcessFailure::Cleanup)?;
    stderr_reader.join().map_err(|_| ProcessFailure::Cleanup)?;
    if read_failed {
        Err(ProcessFailure::Read)
    } else {
        Ok((stdout.unwrap_or_default(), stderr.unwrap_or_default()))
    }
}

#[cfg(unix)]
fn finish_one(
    receiver: &mpsc::Receiver<ReadResult>,
    reader: thread::JoinHandle<()>,
    deadline: Instant,
) -> Result<(), ProcessFailure> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(ProcessFailure::Cleanup);
    }
    let (_, result) = receiver
        .recv_timeout(remaining)
        .map_err(|_| ProcessFailure::Cleanup)?;
    result.map_err(|()| ProcessFailure::Read)?;
    reader.join().map_err(|_| ProcessFailure::Cleanup)
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn fail_after_cleanup<T>(
    child: &mut Child,
    group: Pid,
    deadline: Instant,
    receiver: &mpsc::Receiver<ReadResult>,
    stdout_reader: thread::JoinHandle<()>,
    stderr_reader: thread::JoinHandle<()>,
    failure: ProcessFailure,
) -> Result<T, ProcessFailure> {
    let cleanup_result = cleanup(child, group, deadline);
    let reader_result = finish_readers(receiver, stdout_reader, stderr_reader, deadline);
    cleanup_result?;
    reader_result?;
    Err(failure)
}
