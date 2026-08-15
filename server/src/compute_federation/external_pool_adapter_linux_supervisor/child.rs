use std::ffi::CString;

use super::{
    policy::{SupervisorPolicy, CAPSULE_FD, CHILD_IPC_FD, FIRST_CLOSED_FD, SEED_FD},
    seccomp::{install_seccomp_program, EMPTY_EXEC_PATH},
};

const CLOSE_RANGE_UNSHARE: u32 = 1 << 1;
const LINUX_CAPABILITY_VERSION_3: u32 = 0x2008_0522;
const PR_CAP_AMBIENT: libc::c_int = 47;
const PR_CAP_AMBIENT_CLEAR_ALL: libc::c_ulong = 4;

#[cfg(target_env = "gnu")]
type RlimitResource = libc::__rlimit_resource_t;
#[cfg(not(target_env = "gnu"))]
type RlimitResource = libc::c_int;

#[repr(C)]
#[derive(Default)]
pub(super) struct CloneArgs {
    pub(super) flags: u64,
    pub(super) pidfd: u64,
    pub(super) child_tid: u64,
    pub(super) parent_tid: u64,
    pub(super) exit_signal: u64,
    pub(super) stack: u64,
    pub(super) stack_size: u64,
    pub(super) tls: u64,
    pub(super) set_tid: u64,
    pub(super) set_tid_size: u64,
    pub(super) cgroup: u64,
}

#[repr(C)]
struct CapabilityHeader {
    version: u32,
    pid: i32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CapabilityData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}

pub(super) struct ChildLaunchPlan {
    pub(super) mapping_reader_fd: i32,
    pub(super) null_fd: i32,
    pub(super) stderr_fd: i32,
    pub(super) child_ipc_fd: i32,
    pub(super) capsule_fd: i32,
    pub(super) seed_fd: i32,
    pub(super) argv: Vec<CString>,
    pub(super) scratch_root: CString,
    pub(super) policy: SupervisorPolicy,
    pub(super) seccomp_program: Vec<libc::sock_filter>,
}

impl ChildLaunchPlan {
    /// Runs after `clone3`. Only raw syscalls and inherited fixed buffers are used before exec.
    pub(super) unsafe fn run(&self) -> ! {
        if !wait_for_identity_mapping(self.mapping_reader_fd)
            || libc::setsid() < 0
            || !prepare_private_root(&self.scratch_root)
            || !apply_rlimits(&self.policy)
            || !remap_descriptors(self)
            || !clear_capabilities()
            || libc::prctl(libc::PR_SET_DUMPABLE, 0) != 0
            || libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
            || !install_seccomp_program(&self.seccomp_program)
        {
            libc::_exit(127);
        }

        if !(2..=12).contains(&self.argv.len()) {
            libc::_exit(127);
        }
        let mut argv = [std::ptr::null::<libc::c_char>(); 13];
        for (slot, value) in argv.iter_mut().zip(self.argv.iter()) {
            *slot = value.as_ptr();
        }
        let environment = [std::ptr::null::<libc::c_char>()];
        libc::syscall(
            libc::SYS_execveat,
            CAPSULE_FD,
            EMPTY_EXEC_PATH.as_ptr(),
            argv.as_ptr(),
            environment.as_ptr(),
            libc::AT_EMPTY_PATH,
        );
        libc::_exit(127);
    }
}

unsafe fn wait_for_identity_mapping(fd: i32) -> bool {
    let mut byte = 0_u8;
    let result = loop {
        let read = libc::read(fd, (&mut byte as *mut u8).cast(), 1);
        if read < 0 && errno() == libc::EINTR {
            continue;
        }
        break read;
    };
    libc::close(fd);
    result == 1 && byte == 1
}

unsafe fn prepare_private_root(root: &CString) -> bool {
    if libc::mount(
        std::ptr::null(),
        c"/".as_ptr(),
        std::ptr::null(),
        libc::MS_REC | libc::MS_PRIVATE,
        std::ptr::null(),
    ) != 0
    {
        return false;
    }
    let flags = libc::MS_NODEV | libc::MS_NOSUID | libc::MS_NOEXEC;
    if libc::mount(
        c"tmpfs".as_ptr(),
        root.as_ptr(),
        c"tmpfs".as_ptr(),
        flags,
        c"size=67108864,mode=0700".as_ptr().cast(),
    ) != 0
        || libc::chdir(root.as_ptr()) != 0
        || libc::mkdir(c"oldroot".as_ptr(), 0o700) != 0
        || libc::mkdir(c"tmp".as_ptr(), 0o700) != 0
        || libc::syscall(libc::SYS_pivot_root, c".".as_ptr(), c"oldroot".as_ptr()) != 0
        || libc::chdir(c"/".as_ptr()) != 0
        || libc::umount2(c"/oldroot".as_ptr(), libc::MNT_DETACH) != 0
        || libc::rmdir(c"/oldroot".as_ptr()) != 0
        || libc::chdir(c"/tmp".as_ptr()) != 0
    {
        return false;
    }
    libc::umask(0o077);
    true
}

unsafe fn apply_rlimits(policy: &SupervisorPolicy) -> bool {
    let limits = &policy.confinement.rlimits;
    set_limit(libc::RLIMIT_CORE, limits.core_bytes)
        && set_limit(libc::RLIMIT_NOFILE, limits.nofile)
        && set_limit(libc::RLIMIT_NPROC, limits.nproc)
        && set_limit(libc::RLIMIT_AS, limits.address_space_bytes)
        && set_limit(libc::RLIMIT_FSIZE, limits.file_size_bytes)
        && set_limit(libc::RLIMIT_STACK, limits.stack_bytes)
        && set_limit(libc::RLIMIT_MEMLOCK, limits.memlock_bytes)
        && set_limit(libc::RLIMIT_CPU, limits.cpu_seconds)
}

unsafe fn set_limit(resource: RlimitResource, value: u64) -> bool {
    let value = value as libc::rlim_t;
    let limit = libc::rlimit {
        rlim_cur: value,
        rlim_max: value,
    };
    libc::setrlimit(resource, &limit) == 0
}

unsafe fn remap_descriptors(plan: &ChildLaunchPlan) -> bool {
    if libc::dup3(plan.null_fd, 0, 0) < 0
        || libc::dup3(plan.null_fd, 1, 0) < 0
        || libc::dup3(plan.stderr_fd, 2, 0) < 0
        || libc::dup3(plan.child_ipc_fd, CHILD_IPC_FD, 0) < 0
        || libc::dup3(plan.capsule_fd, CAPSULE_FD, libc::O_CLOEXEC) < 0
        || libc::dup3(plan.seed_fd, SEED_FD, 0) < 0
    {
        return false;
    }
    libc::syscall(
        libc::SYS_close_range,
        FIRST_CLOSED_FD,
        u32::MAX,
        CLOSE_RANGE_UNSHARE,
    ) == 0
}

unsafe fn clear_capabilities() -> bool {
    if libc::setresgid(0, 0, 0) != 0
        || libc::setresuid(0, 0, 0) != 0
        || libc::prctl(PR_CAP_AMBIENT, PR_CAP_AMBIENT_CLEAR_ALL, 0, 0, 0) != 0
    {
        return false;
    }
    for capability in 0..=63 {
        if libc::prctl(libc::PR_CAPBSET_DROP, capability, 0, 0, 0) != 0 && errno() != libc::EINVAL {
            return false;
        }
    }
    let header = CapabilityHeader {
        version: LINUX_CAPABILITY_VERSION_3,
        pid: 0,
    };
    let data = [
        CapabilityData {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        },
        CapabilityData {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        },
    ];
    libc::syscall(libc::SYS_capset, &header, data.as_ptr()) == 0
}

unsafe fn errno() -> i32 {
    *libc::__errno_location()
}
