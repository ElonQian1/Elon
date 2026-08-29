use std::{
    io::Read,
    process::{Child, ChildStderr, ChildStdout, ExitStatus},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use super::child::DynamicChildFailure;

const MAX_CAPTURED_STDOUT_BYTES: usize = 64 * 1024;
const MAX_CAPTURED_STDERR_BYTES: usize = 64 * 1024;
const CHILD_WAIT_TIMEOUT: Duration = Duration::from_secs(120);
const ABORT_CONFIRM_TIMEOUT: Duration = Duration::from_secs(5);
const CHILD_POLL_INTERVAL: Duration = Duration::from_millis(10);

pub(super) struct CapturedChildOutput {
    pub(super) status: ExitStatus,
    pub(super) stdout: Vec<u8>,
}

pub(super) fn wait_for_bounded_output(
    child: &mut Child,
) -> Result<CapturedChildOutput, DynamicChildFailure> {
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            return Err(abort_child(child, "A2_DYNAMIC_CHILD_STDOUT_NOT_PIPED"));
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            return Err(abort_child(child, "A2_DYNAMIC_CHILD_STDERR_NOT_PIPED"));
        }
    };
    let stdout_reader = match spawn_stdout_reader(stdout) {
        Ok(reader) => reader,
        Err(error) => {
            return Err(abort_child(child, error));
        }
    };
    let stderr_reader = match spawn_stderr_reader(stderr) {
        Ok(reader) => reader,
        Err(error) => {
            let failure = abort_child(child, error);
            if failure.exit_confirmed() {
                let _ = stdout_reader.join();
            }
            return Err(failure);
        }
    };

    let status = match wait_for_child_exit(child) {
        Ok(status) => status,
        Err(failure) => {
            if failure.exit_confirmed() {
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
            }
            return Err(failure);
        }
    };
    let stdout = join_reader(stdout_reader, "A2_DYNAMIC_CHILD_STDOUT_READER_PANICKED")
        .map_err(DynamicChildFailure::exited)?;
    let _stderr = join_reader(stderr_reader, "A2_DYNAMIC_CHILD_STDERR_READER_PANICKED")
        .map_err(DynamicChildFailure::exited)?;
    Ok(CapturedChildOutput { status, stdout })
}

fn wait_for_child_exit(child: &mut Child) -> Result<ExitStatus, DynamicChildFailure> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started.elapsed() < CHILD_WAIT_TIMEOUT => {
                thread::sleep(CHILD_POLL_INTERVAL);
            }
            Ok(None) => {
                return Err(abort_child_with_detail(
                    child,
                    "A2_DYNAMIC_CHILD_WAIT_TIMEOUT",
                    "child did not exit within the bounded wait".to_owned(),
                ));
            }
            Err(error) => {
                return Err(abort_child_with_detail(
                    child,
                    "A2_DYNAMIC_CHILD_WAIT_FAILED",
                    format!("initial try_wait failed: {error}"),
                ));
            }
        }
    }
}

fn spawn_stdout_reader(
    stdout: ChildStdout,
) -> Result<JoinHandle<Result<Vec<u8>, &'static str>>, &'static str> {
    thread::Builder::new()
        .name("a2-dynamic-stdout".to_owned())
        .spawn(move || {
            drain_capped(
                stdout,
                MAX_CAPTURED_STDOUT_BYTES,
                "A2_DYNAMIC_CHILD_STDOUT_TOO_LARGE",
                "A2_DYNAMIC_CHILD_STDOUT_READ_FAILED",
            )
        })
        .map_err(|_| "A2_DYNAMIC_CHILD_STDOUT_READER_SPAWN_FAILED")
}

fn spawn_stderr_reader(
    stderr: ChildStderr,
) -> Result<JoinHandle<Result<Vec<u8>, &'static str>>, &'static str> {
    thread::Builder::new()
        .name("a2-dynamic-stderr".to_owned())
        .spawn(move || {
            drain_capped(
                stderr,
                MAX_CAPTURED_STDERR_BYTES,
                "A2_DYNAMIC_CHILD_STDERR_TOO_LARGE",
                "A2_DYNAMIC_CHILD_STDERR_READ_FAILED",
            )
        })
        .map_err(|_| "A2_DYNAMIC_CHILD_STDERR_READER_SPAWN_FAILED")
}

fn drain_capped(
    mut reader: impl Read,
    limit: usize,
    too_large: &'static str,
    read_failed: &'static str,
) -> Result<Vec<u8>, &'static str> {
    let mut captured = Vec::with_capacity(limit.min(8 * 1024));
    let mut buffer = [0u8; 8 * 1024];
    let mut exceeded = false;
    loop {
        let read = reader.read(&mut buffer).map_err(|_| read_failed)?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(captured.len());
        let retained = read.min(remaining);
        captured.extend_from_slice(&buffer[..retained]);
        exceeded |= retained != read;
    }
    if exceeded {
        Err(too_large)
    } else {
        Ok(captured)
    }
}

fn join_reader(
    reader: JoinHandle<Result<Vec<u8>, &'static str>>,
    panic_error: &'static str,
) -> Result<Vec<u8>, &'static str> {
    reader.join().map_err(|_| panic_error)?
}

pub(super) fn abort_child(child: &mut Child, code: &'static str) -> DynamicChildFailure {
    abort_child_with_detail(child, code, "child capture aborted".to_owned())
}

fn abort_child_with_detail(
    child: &mut Child,
    code: &'static str,
    cause: String,
) -> DynamicChildFailure {
    let mut details = vec![cause];
    match child.try_wait() {
        Ok(Some(status)) => {
            details.push(format!("already exited with {status}"));
            return DynamicChildFailure::exited_with_detail(code, details.join("; "));
        }
        Ok(None) => {}
        Err(error) => details.push(format!("pre-kill try_wait failed: {error}")),
    }
    if let Err(error) = child.kill() {
        details.push(format!("kill failed: {error}"));
    }
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                details.push(format!("exit confirmed with {status}"));
                return DynamicChildFailure::exited_with_detail(code, details.join("; "));
            }
            Ok(None) if started.elapsed() < ABORT_CONFIRM_TIMEOUT => {
                thread::sleep(CHILD_POLL_INTERVAL);
            }
            Ok(None) => {
                details.push("exit was not observed before abort timeout".to_owned());
                return DynamicChildFailure::exit_unconfirmed(code, details.join("; "));
            }
            Err(error) => {
                details.push(format!("post-kill try_wait failed: {error}"));
                return DynamicChildFailure::exit_unconfirmed(code, details.join("; "));
            }
        }
    }
}

#[cfg(test)]
pub(super) fn drain_capped_for_test(input: &[u8], limit: usize) -> Result<Vec<u8>, &'static str> {
    drain_capped(
        input,
        limit,
        "A2_DYNAMIC_CHILD_TEST_TOO_LARGE",
        "A2_DYNAMIC_CHILD_TEST_READ_FAILED",
    )
}
