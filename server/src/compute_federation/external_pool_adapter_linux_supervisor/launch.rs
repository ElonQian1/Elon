use std::{
    ffi::CString,
    fs::{self, File, OpenOptions},
    io::Write,
    mem::size_of,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::{fs::MetadataExt, fs::OpenOptionsExt, fs::PermissionsExt},
    },
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use ring::rand::{SecureRandom, SystemRandom};

use crate::compute_federation::external_pool_adapter_supervisor_session::{
    ExternalPoolAdapterChildBootstrap, ExternalPoolAdapterSessionRootArguments,
};

use super::{
    cgroup::{ExternalPoolAdapterSupervisorCgroupParent, SupervisorCgroupLeaf},
    child::{ChildLaunchPlan, CloneArgs},
    lifecycle::{terminate_pidfd_and_reap, ExternalPoolAdapterSupervisorChild},
    policy::SupervisorPolicy,
    seccomp::build_seccomp_program,
};

const REQUIRED_CAPSULE_SEALS: libc::c_int =
    libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
const CAPSULE_MODE: u32 = 0o500;
const CHILD_SOURCE_FD_MINIMUM: i32 = 10;
const CLONE_INTO_CGROUP: u64 = 1 << 33;

pub(crate) trait ExternalPoolAdapterSupervisorCapsule {
    fn retained_sealed_image(&self) -> &File;
}

pub(super) struct SupervisorScratchRoot {
    path: PathBuf,
    path_cstring: CString,
    removed: bool,
}

pub(crate) fn launch_external_pool_adapter_supervisor_child(
    cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
    child_bootstrap: ExternalPoolAdapterChildBootstrap,
    capsule: &impl ExternalPoolAdapterSupervisorCapsule,
) -> Result<ExternalPoolAdapterSupervisorChild> {
    let policy = SupervisorPolicy::load()?;
    require_sealed_capsule(capsule.retained_sealed_image())?;
    let mut cgroup = cgroup_parent.create_leaf(&policy)?;
    let scratch = SupervisorScratchRoot::create()?;
    let seccomp_program = build_seccomp_program(&policy)?;
    let (child_ipc, seed_reader, root_arguments) =
        child_bootstrap.into_supervisor_descriptors()?.into_parts();
    set_blocking(child_ipc.as_raw_fd())?;
    let null = open_dev_null()?;
    let (stderr_reader, stderr_writer) = create_pipe(true)?;
    let (mapping_reader, mapping_writer) = create_pipe(false)?;

    let null_child = duplicate_child_source(null.as_raw_fd())?;
    let stderr_child = duplicate_child_source(stderr_writer.as_raw_fd())?;
    let ipc_child = duplicate_child_source(child_ipc.as_raw_fd())?;
    let capsule_child = duplicate_child_source(capsule.retained_sealed_image().as_raw_fd())?;
    let seed_child = duplicate_child_source(seed_reader.as_raw_fd())?;
    let mapping_child = duplicate_child_source(mapping_reader.as_raw_fd())?;

    let plan = ChildLaunchPlan {
        mapping_reader_fd: mapping_child.as_raw_fd(),
        null_fd: null_child.as_raw_fd(),
        stderr_fd: stderr_child.as_raw_fd(),
        child_ipc_fd: ipc_child.as_raw_fd(),
        capsule_fd: capsule_child.as_raw_fd(),
        seed_fd: seed_child.as_raw_fd(),
        argv: child_launch_argv(&root_arguments)?,
        scratch_root: scratch.path_cstring.clone(),
        policy,
        seccomp_program,
    };
    let mut pidfd_slot = -1_i32;
    let mut clone_args = CloneArgs {
        flags: libc::CLONE_PIDFD as u64
            | CLONE_INTO_CGROUP
            | libc::CLONE_NEWUSER as u64
            | libc::CLONE_NEWNS as u64
            | libc::CLONE_NEWNET as u64
            | libc::CLONE_NEWIPC as u64
            | libc::CLONE_NEWUTS as u64,
        pidfd: (&mut pidfd_slot as *mut i32) as u64,
        exit_signal: libc::SIGCHLD as u64,
        cgroup: cgroup.as_raw_fd() as u64,
        ..CloneArgs::default()
    };
    let result = unsafe {
        libc::syscall(
            libc::SYS_clone3,
            &mut clone_args as *mut CloneArgs,
            size_of::<CloneArgs>(),
        )
    };
    if result < 0 {
        return Err(std::io::Error::last_os_error()).context("clone isolated supervisor child");
    }
    if result == 0 {
        unsafe { plan.run() }
    }
    let child_pid = result as libc::pid_t;
    // A successful clone3 with CLONE_PIDFD atomically initializes this slot. There is no PID
    // fallback: the syscall itself fails if the kernel cannot return the process handle.
    debug_assert!(pidfd_slot >= 0);
    let pidfd = unsafe { OwnedFd::from_raw_fd(pidfd_slot) };

    drop(mapping_reader);
    drop(stderr_writer);
    drop(child_ipc);
    drop(seed_reader);
    drop(null);
    drop(null_child);
    drop(stderr_child);
    drop(ipc_child);
    drop(capsule_child);
    drop(seed_child);
    drop(mapping_child);

    let mapping_result = write_identity_maps(child_pid).and_then(|_| {
        write_exact_byte(mapping_writer.as_raw_fd(), 1).context("release mapped supervisor child")
    });
    drop(mapping_writer);
    if let Err(error) = mapping_result {
        terminate_pidfd_and_reap(pidfd.as_raw_fd())?;
        let _ = cgroup.remove();
        return Err(error);
    }

    Ok(ExternalPoolAdapterSupervisorChild::new(
        child_pid,
        pidfd,
        cgroup,
        scratch,
        stderr_reader,
    ))
}

impl SupervisorScratchRoot {
    fn create() -> Result<Self> {
        let mut nonce = [0_u8; 16];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| anyhow!("generate supervisor scratch identity"))?;
        let path = std::env::temp_dir().join(format!(
            "elon-v261-supervisor-{}-{}",
            std::process::id(),
            hex::encode(nonce)
        ));
        fs::create_dir(&path).context("create supervisor scratch mountpoint")?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
            .context("set supervisor scratch permissions")?;
        let path_cstring = path_to_cstring(&path)?;
        Ok(Self {
            path,
            path_cstring,
            removed: false,
        })
    }

    pub(super) fn remove(&mut self) -> Result<()> {
        if self.removed {
            return Ok(());
        }
        fs::remove_dir(&self.path).context("remove supervisor scratch mountpoint")?;
        self.removed = true;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn path_for_test(&self) -> &Path {
        &self.path
    }
}

impl Drop for SupervisorScratchRoot {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

fn require_sealed_capsule(file: &File) -> Result<()> {
    let metadata = file.metadata().context("inspect retained capsule")?;
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
    let seals = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) };
    if !metadata.is_file()
        || metadata.mode() & 0o777 != CAPSULE_MODE
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.nlink() != 0
        || flags < 0
        || flags & libc::FD_CLOEXEC == 0
        || seals != REQUIRED_CAPSULE_SEALS
    {
        bail!("supervisor rejected untrusted capsule descriptor");
    }
    Ok(())
}

fn open_dev_null() -> Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let file = options
        .open("/dev/null")
        .context("open supervisor null device")?;
    if file.metadata()?.mode() & libc::S_IFMT != libc::S_IFCHR {
        bail!("supervisor null device is not a character device");
    }
    Ok(file)
}

fn create_pipe(nonblocking: bool) -> Result<(OwnedFd, OwnedFd)> {
    let mut descriptors = [-1; 2];
    let flags = libc::O_CLOEXEC | if nonblocking { libc::O_NONBLOCK } else { 0 };
    if unsafe { libc::pipe2(descriptors.as_mut_ptr(), flags) } != 0 {
        return Err(std::io::Error::last_os_error()).context("create supervisor pipe");
    }
    Ok((unsafe { OwnedFd::from_raw_fd(descriptors[0]) }, unsafe {
        OwnedFd::from_raw_fd(descriptors[1])
    }))
}

fn duplicate_child_source(fd: i32) -> Result<OwnedFd> {
    let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, CHILD_SOURCE_FD_MINIMUM) };
    if duplicate < 0 {
        return Err(std::io::Error::last_os_error()).context("duplicate child launch descriptor");
    }
    Ok(unsafe { OwnedFd::from_raw_fd(duplicate) })
}

fn set_blocking(fd: i32) -> Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags & !libc::O_NONBLOCK) } != 0 {
        return Err(std::io::Error::last_os_error())
            .context("make inherited child session socket blocking");
    }
    Ok(())
}

fn child_launch_argv(roots: &ExternalPoolAdapterSessionRootArguments) -> Result<[CString; 7]> {
    let [policy, profile, target, companion, capsule, bundle] = roots.values();
    Ok([
        CString::new("elon-external-pool-adapter")?,
        digest_argument("policy", policy)?,
        digest_argument("profile", profile)?,
        digest_argument("target", target)?,
        digest_argument("companion", companion)?,
        digest_argument("capsule", capsule)?,
        digest_argument("bundle", bundle)?,
    ])
}

fn digest_argument(label: &str, digest: &str) -> Result<CString> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        bail!("supervisor launch root digest is invalid");
    }
    CString::new(format!("--elon-session-{label}={digest}"))
        .context("supervisor launch root argument contains NUL")
}

fn write_identity_maps(pid: libc::pid_t) -> Result<()> {
    if pid <= 0 {
        bail!("invalid supervisor child identity");
    }
    let host_uid = unsafe { libc::geteuid() };
    let host_gid = unsafe { libc::getegid() };
    write_proc_map(pid, "setgroups", "deny\n")?;
    write_proc_map(pid, "uid_map", &format!("0 {host_uid} 1\n"))?;
    write_proc_map(pid, "gid_map", &format!("0 {host_gid} 1\n"))?;
    Ok(())
}

fn write_proc_map(pid: libc::pid_t, name: &str, value: &str) -> Result<()> {
    if !matches!(name, "setgroups" | "uid_map" | "gid_map") {
        bail!("invalid supervisor identity map");
    }
    let path = PathBuf::from(format!("/proc/{pid}/{name}"));
    let mut options = OpenOptions::new();
    options
        .write(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let mut file = options
        .open(&path)
        .context("open supervisor identity map")?;
    file.write_all(value.as_bytes())
        .context("write supervisor identity map")?;
    Ok(())
}

fn write_exact_byte(fd: i32, value: u8) -> Result<()> {
    let written = unsafe { libc::write(fd, (&value as *const u8).cast(), 1) };
    if written != 1 {
        return Err(std::io::Error::last_os_error()).context("write supervisor sync byte");
    }
    Ok(())
}

#[cfg(unix)]
fn path_to_cstring(path: &Path) -> Result<CString> {
    use std::os::unix::ffi::OsStrExt;
    CString::new(path.as_os_str().as_bytes()).context("supervisor scratch path contains NUL")
}
