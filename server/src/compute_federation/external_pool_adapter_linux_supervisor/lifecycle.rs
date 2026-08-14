use std::{
    os::fd::{AsRawFd, OwnedFd},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use zeroize::Zeroizing;

use super::{
    cgroup::SupervisorCgroupLeaf,
    launch::SupervisorScratchRoot,
    policy::{SHUTDOWN_GRACE_MS, STDERR_LIMIT_BYTES},
};

const P_PIDFD: libc::idtype_t = 3;
const PIDFD_WAIT_SLICE: Duration = Duration::from_millis(10);

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct ExternalPoolAdapterSupervisorExit {
    pub(crate) exit_code: Option<i32>,
    pub(crate) signal: Option<i32>,
}

pub(crate) struct ExternalPoolAdapterSupervisorChild {
    pid: libc::pid_t,
    pidfd: OwnedFd,
    cgroup: SupervisorCgroupLeaf,
    scratch: SupervisorScratchRoot,
    stderr: OwnedFd,
    reaped: bool,
}

impl ExternalPoolAdapterSupervisorChild {
    pub(super) fn new(
        pid: libc::pid_t,
        pidfd: OwnedFd,
        cgroup: SupervisorCgroupLeaf,
        scratch: SupervisorScratchRoot,
        stderr: OwnedFd,
    ) -> Self {
        Self {
            pid,
            pidfd,
            cgroup,
            scratch,
            stderr,
            reaped: false,
        }
    }

    pub(crate) fn wait(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<ExternalPoolAdapterSupervisorExit>> {
        if self.reaped {
            bail!("supervisor child was already reaped");
        }
        if !poll_pidfd(self.pidfd.as_raw_fd(), timeout)? {
            return Ok(None);
        }
        let observed = waitid_pidfd(self.pidfd.as_raw_fd())?;
        self.reaped = true;
        self.cleanup_after_reap()?;
        Ok(Some(observed))
    }

    pub(crate) fn terminate(&mut self) -> Result<ExternalPoolAdapterSupervisorExit> {
        if self.reaped {
            bail!("supervisor child was already reaped");
        }
        pidfd_send_signal(self.pidfd.as_raw_fd(), libc::SIGTERM)?;
        if !poll_pidfd(
            self.pidfd.as_raw_fd(),
            Duration::from_millis(SHUTDOWN_GRACE_MS),
        )? {
            pidfd_send_signal(self.pidfd.as_raw_fd(), libc::SIGKILL)?;
            if !poll_pidfd(
                self.pidfd.as_raw_fd(),
                Duration::from_millis(SHUTDOWN_GRACE_MS),
            )? {
                bail!("supervisor child did not terminate after pidfd SIGKILL");
            }
        }
        let observed = waitid_pidfd(self.pidfd.as_raw_fd())?;
        self.reaped = true;
        self.cleanup_after_reap()?;
        Ok(observed)
    }

    pub(crate) fn collect_stderr(&mut self) -> Result<Zeroizing<Vec<u8>>> {
        let mut output = Zeroizing::new(Vec::with_capacity(4096));
        let mut buffer = [0_u8; 4096];
        loop {
            let read = unsafe {
                libc::read(
                    self.stderr.as_raw_fd(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                )
            };
            if read > 0 {
                let read = read as usize;
                if output.len().saturating_add(read) > STDERR_LIMIT_BYTES {
                    if !self.reaped {
                        let _ = self.terminate();
                    }
                    bail!("supervisor stderr exceeded server-fixed bound");
                }
                output.extend_from_slice(&buffer[..read]);
                continue;
            }
            if read == 0 {
                break;
            }
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            if error.raw_os_error() == Some(libc::EAGAIN) {
                break;
            }
            return Err(error).context("collect bounded supervisor stderr");
        }
        Ok(output)
    }

    fn cleanup_after_reap(&mut self) -> Result<()> {
        self.cgroup.remove()?;
        self.scratch.remove()?;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn pid_for_test(&self) -> libc::pid_t {
        self.pid
    }

    #[cfg(test)]
    pub(super) fn cgroup_for_test(&self) -> &SupervisorCgroupLeaf {
        &self.cgroup
    }

    #[cfg(test)]
    pub(super) fn scratch_path_for_test(&self) -> &std::path::Path {
        self.scratch.path_for_test()
    }
}

impl Drop for ExternalPoolAdapterSupervisorChild {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = pidfd_send_signal(self.pidfd.as_raw_fd(), libc::SIGKILL);
            let _ = poll_pidfd(
                self.pidfd.as_raw_fd(),
                Duration::from_millis(SHUTDOWN_GRACE_MS),
            );
            if waitid_pidfd(self.pidfd.as_raw_fd()).is_ok() {
                self.reaped = true;
                let _ = self.cleanup_after_reap();
            }
        }
    }
}

pub(super) fn terminate_pidfd_and_reap(pidfd: i32) -> Result<()> {
    pidfd_send_signal(pidfd, libc::SIGKILL)?;
    if !poll_pidfd(pidfd, Duration::from_millis(SHUTDOWN_GRACE_MS))? {
        bail!("failed launch child did not terminate after pidfd SIGKILL");
    }
    waitid_pidfd(pidfd)?;
    Ok(())
}

fn pidfd_send_signal(pidfd: i32, signal: i32) -> Result<()> {
    if unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            pidfd,
            signal,
            std::ptr::null::<libc::siginfo_t>(),
            0,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error()).context("signal supervisor child by pidfd");
    }
    Ok(())
}

fn poll_pidfd(pidfd: i32, timeout: Duration) -> Result<bool> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let milliseconds = remaining
            .min(PIDFD_WAIT_SLICE)
            .as_millis()
            .min(i32::MAX as u128) as i32;
        let mut descriptor = libc::pollfd {
            fd: pidfd,
            events: libc::POLLIN,
            revents: 0,
        };
        let result = unsafe { libc::poll(&mut descriptor, 1, milliseconds) };
        if result > 0 {
            return Ok(descriptor.revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0);
        }
        if result < 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return Err(error).context("poll supervisor pidfd");
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
    }
}

fn waitid_pidfd(pidfd: i32) -> Result<ExternalPoolAdapterSupervisorExit> {
    let mut information = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
    if unsafe {
        libc::waitid(
            P_PIDFD,
            pidfd as libc::id_t,
            information.as_mut_ptr(),
            libc::WEXITED,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error()).context("reap supervisor child by pidfd");
    }
    let information = unsafe { information.assume_init() };
    let status = unsafe { information.si_status() };
    let (exit_code, signal) = match information.si_code {
        libc::CLD_EXITED => (Some(status), None),
        libc::CLD_KILLED | libc::CLD_DUMPED => (None, Some(status)),
        _ => bail!("supervisor waitid returned a non-terminal state"),
    };
    Ok(ExternalPoolAdapterSupervisorExit { exit_code, signal })
}
