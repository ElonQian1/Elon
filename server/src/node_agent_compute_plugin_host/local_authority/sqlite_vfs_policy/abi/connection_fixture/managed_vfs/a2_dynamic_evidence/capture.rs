use std::{
    io::Read,
    process::{Child, ChildStderr, ChildStdout, ExitStatus},
    thread::{self, JoinHandle},
};

const MAX_CAPTURED_STDOUT_BYTES: usize = 64 * 1024;
const MAX_CAPTURED_STDERR_BYTES: usize = 64 * 1024;

pub(super) struct CapturedChildOutput {
    pub(super) status: ExitStatus,
    pub(super) stdout: Vec<u8>,
}

pub(super) fn wait_for_bounded_output(
    child: &mut Child,
) -> Result<CapturedChildOutput, &'static str> {
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            abort_child(child);
            return Err("A2_DYNAMIC_CHILD_STDOUT_NOT_PIPED");
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            abort_child(child);
            return Err("A2_DYNAMIC_CHILD_STDERR_NOT_PIPED");
        }
    };
    let stdout_reader = match spawn_stdout_reader(stdout) {
        Ok(reader) => reader,
        Err(error) => {
            abort_child(child);
            return Err(error);
        }
    };
    let stderr_reader = match spawn_stderr_reader(stderr) {
        Ok(reader) => reader,
        Err(error) => {
            abort_child(child);
            let _ = stdout_reader.join();
            return Err(error);
        }
    };

    let status = child.wait();
    if status.is_err() {
        abort_child(child);
    }
    let stdout = join_reader(stdout_reader, "A2_DYNAMIC_CHILD_STDOUT_READER_PANICKED");
    let stderr = join_reader(stderr_reader, "A2_DYNAMIC_CHILD_STDERR_READER_PANICKED");
    let status = status.map_err(|_| "A2_DYNAMIC_CHILD_WAIT_FAILED")?;
    let stdout = stdout?;
    let _stderr = stderr?;
    Ok(CapturedChildOutput { status, stdout })
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

pub(super) fn abort_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
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
