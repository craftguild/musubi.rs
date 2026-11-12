//! Unix process-group signalling and deadline-bounded direct-child reaping.

use std::{
    process::Child,
    thread,
    time::{Duration, Instant},
};

use rustix::{
    io::Errno,
    process::{Pid, Signal, kill_process_group, test_kill_process_group},
};

use crate::ProcessFailure;

pub(crate) trait WaitChild {
    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl WaitChild for Child {
    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        Self::try_wait(self)
    }
}

pub(crate) fn cleanup_failure<T>(
    child: &mut Child,
    group: Pid,
    deadline: Instant,
    failure: ProcessFailure,
) -> Result<T, ProcessFailure> {
    cleanup(child, group, deadline)?;
    Err(failure)
}

pub(crate) fn cleanup(
    child: &mut impl WaitChild,
    group: Pid,
    deadline: Instant,
) -> Result<(), ProcessFailure> {
    let term_result = signal_group(group, Signal::TERM);
    let remaining = deadline.saturating_duration_since(Instant::now());
    let graceful_deadline = Instant::now() + (remaining / 2).min(Duration::from_millis(250));
    let mut wait_failed = false;
    let mut reaped = false;
    while Instant::now() < graceful_deadline {
        match child.try_wait() {
            Ok(Some(_)) => {
                reaped = true;
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(_) => {
                wait_failed = true;
                break;
            }
        }
    }
    let kill_result = signal_group(group, Signal::KILL);
    let reap_result = if reaped {
        Ok(())
    } else {
        reap_child(child, deadline)
    };
    let group_result = wait_group_empty(group, deadline);
    term_result?;
    kill_result?;
    reap_result?;
    group_result?;
    if wait_failed {
        Err(ProcessFailure::Wait)
    } else {
        Ok(())
    }
}

pub(crate) fn force_group(group: Pid, deadline: Instant) -> Result<(), ProcessFailure> {
    let signal_result = signal_group(group, Signal::KILL);
    let group_result = wait_group_empty(group, deadline);
    signal_result?;
    group_result
}

fn signal_group(group: Pid, signal: Signal) -> Result<(), ProcessFailure> {
    match kill_process_group(group, signal) {
        Ok(()) | Err(Errno::SRCH) => Ok(()),
        Err(_) => Err(ProcessFailure::Cleanup),
    }
}

fn reap_child(child: &mut impl WaitChild, deadline: Instant) -> Result<(), ProcessFailure> {
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            Ok(None) => return Err(ProcessFailure::Cleanup),
            Err(_) => return Err(ProcessFailure::Wait),
        }
    }
}

fn wait_group_empty(group: Pid, deadline: Instant) -> Result<(), ProcessFailure> {
    loop {
        match test_kill_process_group(group) {
            Err(Errno::SRCH) => return Ok(()),
            Ok(()) | Err(Errno::PERM) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(5));
            }
            Ok(()) | Err(_) => return Err(ProcessFailure::Cleanup),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        os::unix::process::CommandExt as _,
        process::{Command, Stdio},
    };

    use super::*;
    use rustix::process::test_kill_process;

    struct FailOnce<'a> {
        child: &'a mut Child,
        failed: bool,
    }

    impl WaitChild for FailOnce<'_> {
        fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
            if self.failed {
                self.child.try_wait()
            } else {
                self.failed = true;
                Err(std::io::Error::other("injected wait failure"))
            }
        }
    }

    #[test]
    fn wait_failure_still_kills_the_process_group() {
        let mut child = Command::new("sh")
            .args(["-c", "trap '' TERM; while :; do :; done"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .unwrap();
        let group = Pid::from_child(&child);
        thread::sleep(Duration::from_millis(20));
        let result = {
            let mut failing = FailOnce {
                child: &mut child,
                failed: false,
            };
            cleanup(&mut failing, group, Instant::now() + Duration::from_secs(2))
        };
        assert_eq!(result, Err(ProcessFailure::Wait));
        assert_eq!(test_kill_process(group), Err(Errno::SRCH));
        assert_eq!(test_kill_process_group(group), Err(Errno::SRCH));
    }
}
