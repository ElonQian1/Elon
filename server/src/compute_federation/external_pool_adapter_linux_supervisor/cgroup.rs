use std::{
    ffi::{CStr, CString},
    fs::File,
    io::Read,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

use anyhow::{anyhow, bail, Context, Result};
use ring::rand::{SecureRandom, SystemRandom};

use super::policy::SupervisorPolicy;

const CGROUP2_SUPER_MAGIC: libc::c_long = 0x6367_7270;
const MAX_CONTROL_FILE_BYTES: usize = 4 * 1024;
const REQUIRED_CONTROLLERS: [&str; 3] = ["cpu", "memory", "pids"];

pub(crate) struct ExternalPoolAdapterSupervisorCgroupParent {
    directory: File,
}

pub(super) struct SupervisorCgroupLeaf {
    parent: OwnedFd,
    directory: OwnedFd,
    name: CString,
    removed: bool,
}

impl ExternalPoolAdapterSupervisorCgroupParent {
    pub(crate) fn from_directory(directory: File) -> Result<Self> {
        require_cgroup2(directory.as_raw_fd())?;
        let available = read_control_file(directory.as_raw_fd(), c"cgroup.controllers")?;
        let enabled = read_control_file(directory.as_raw_fd(), c"cgroup.subtree_control")?;
        for controller in REQUIRED_CONTROLLERS {
            if !contains_word(&available, controller) || !contains_word(&enabled, controller) {
                bail!("delegated cgroup parent is missing a required controller");
            }
        }
        Ok(Self { directory })
    }

    pub(super) fn create_leaf(&self, policy: &SupervisorPolicy) -> Result<SupervisorCgroupLeaf> {
        let name = random_leaf_name()?;
        if unsafe { libc::mkdirat(self.directory.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(std::io::Error::last_os_error()).context("create dedicated cgroup leaf");
        }
        let parent = match duplicate_fd(self.directory.as_raw_fd()) {
            Ok(parent) => parent,
            Err(error) => {
                let removed = unsafe {
                    libc::unlinkat(
                        self.directory.as_raw_fd(),
                        name.as_ptr(),
                        libc::AT_REMOVEDIR,
                    )
                };
                if removed != 0 {
                    return Err(error)
                        .context("duplicate failed and dedicated cgroup rollback failed");
                }
                return Err(error);
            }
        };
        let directory = match open_directory_at(self.directory.as_raw_fd(), &name) {
            Ok(directory) => directory,
            Err(error) => {
                let removed = unsafe {
                    libc::unlinkat(
                        self.directory.as_raw_fd(),
                        name.as_ptr(),
                        libc::AT_REMOVEDIR,
                    )
                };
                if removed != 0 {
                    return Err(error).context("open failed and dedicated cgroup rollback failed");
                }
                return Err(error);
            }
        };
        let mut leaf = SupervisorCgroupLeaf {
            parent,
            directory,
            name,
            removed: false,
        };
        if let Err(error) = leaf.configure(policy) {
            if leaf.remove().is_err() {
                return Err(error).context("configure failed and dedicated cgroup rollback failed");
            }
            return Err(error);
        }
        Ok(leaf)
    }
}

impl SupervisorCgroupLeaf {
    pub(super) fn as_raw_fd(&self) -> i32 {
        self.directory.as_raw_fd()
    }

    pub(super) fn name(&self) -> &CStr {
        &self.name
    }

    fn configure(&self, policy: &SupervisorPolicy) -> Result<()> {
        let cgroup = &policy.confinement.cgroup;
        write_control_file(self.as_raw_fd(), c"pids.max", &cgroup.pids_max.to_string())?;
        write_control_file(
            self.as_raw_fd(),
            c"memory.max",
            &cgroup.memory_max_bytes.to_string(),
        )?;
        write_control_file(
            self.as_raw_fd(),
            c"memory.swap.max",
            &cgroup.memory_swap_max_bytes.to_string(),
        )?;
        write_control_file(
            self.as_raw_fd(),
            c"memory.oom.group",
            if cgroup.memory_oom_group { "1" } else { "0" },
        )?;
        write_control_file(
            self.as_raw_fd(),
            c"cpu.max",
            &format!("{} {}", cgroup.cpu_quota_us, cgroup.cpu_period_us),
        )?;
        Ok(())
    }

    pub(super) fn remove(&mut self) -> Result<()> {
        if self.removed {
            return Ok(());
        }
        let result = unsafe {
            libc::unlinkat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                libc::AT_REMOVEDIR,
            )
        };
        if result != 0 {
            return Err(std::io::Error::last_os_error()).context("remove dedicated cgroup leaf");
        }
        self.removed = true;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn read_for_test(&self, name: &CStr) -> Result<String> {
        read_control_file(self.as_raw_fd(), name)
    }
}

impl Drop for SupervisorCgroupLeaf {
    fn drop(&mut self) {
        if self.remove().is_err() {
            tracing::error!(
                target: "security",
                "supervisor cgroup fallback cleanup failed"
            );
        }
    }
}

fn require_cgroup2(fd: i32) -> Result<()> {
    let mut status = std::mem::MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::fstatfs(fd, status.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error()).context("inspect cgroup filesystem");
    }
    let status = unsafe { status.assume_init() };
    if status.f_type as libc::c_long != CGROUP2_SUPER_MAGIC {
        bail!("supervisor requires cgroup v2");
    }
    Ok(())
}

fn random_leaf_name() -> Result<CString> {
    let mut nonce = [0_u8; 16];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| anyhow!("generate cgroup leaf identity"))?;
    CString::new(format!("elon-v261-{}", hex::encode(nonce)))
        .map_err(|_| anyhow!("generate cgroup leaf name"))
}

fn contains_word(value: &str, expected: &str) -> bool {
    value.split_ascii_whitespace().any(|item| item == expected)
}

fn open_directory_at(parent: i32, name: &CStr) -> Result<OwnedFd> {
    let fd = unsafe {
        libc::openat(
            parent,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error()).context("open dedicated cgroup leaf");
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn duplicate_fd(fd: i32) -> Result<OwnedFd> {
    let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 10) };
    if duplicate < 0 {
        return Err(std::io::Error::last_os_error()).context("duplicate cgroup parent handle");
    }
    Ok(unsafe { OwnedFd::from_raw_fd(duplicate) })
}

fn read_control_file(parent: i32, name: &CStr) -> Result<String> {
    let fd = unsafe {
        libc::openat(
            parent,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error()).context("open cgroup control file");
    }
    let mut file = unsafe { File::from_raw_fd(fd) };
    let mut bytes = Vec::with_capacity(256);
    file.by_ref()
        .take((MAX_CONTROL_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .context("read cgroup control file")?;
    if bytes.len() > MAX_CONTROL_FILE_BYTES {
        bail!("cgroup control file exceeded bound");
    }
    String::from_utf8(bytes).context("cgroup control file is not UTF-8")
}

fn write_control_file(parent: i32, name: &CStr, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 128 || !value.is_ascii() {
        bail!("invalid cgroup control value");
    }
    let fd = unsafe {
        libc::openat(
            parent,
            name.as_ptr(),
            libc::O_WRONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error()).context("open cgroup control for write");
    }
    let descriptor = unsafe { OwnedFd::from_raw_fd(fd) };
    let bytes = value.as_bytes();
    let written =
        unsafe { libc::write(descriptor.as_raw_fd(), bytes.as_ptr().cast(), bytes.len()) };
    if written != bytes.len() as isize {
        return Err(std::io::Error::last_os_error()).context("write exact cgroup control value");
    }
    Ok(())
}
