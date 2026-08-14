use anyhow::{bail, Context, Result};

use crate::compute_federation::external_pool_adapter_supervisor_session_policy_companion::{
    server_supervisor_session_policy_catalog, ExternalPoolAdapterSupervisorLinuxConfinementPolicy,
};

pub(super) const CHILD_IPC_FD: i32 = 3;
pub(super) const CAPSULE_FD: i32 = 4;
pub(super) const SEED_FD: i32 = 5;
pub(super) const FIRST_CLOSED_FD: u32 = 6;
pub(super) const STDERR_LIMIT_BYTES: usize = 1_048_576;
pub(super) const SHUTDOWN_GRACE_MS: u64 = 5_000;

#[derive(Clone)]
pub(super) struct SupervisorPolicy {
    pub(super) confinement: ExternalPoolAdapterSupervisorLinuxConfinementPolicy,
}

impl SupervisorPolicy {
    pub(super) fn load() -> Result<Self> {
        let (policy, digest) = server_supervisor_session_policy_catalog()
            .context("load server-fixed supervisor/session policy")?;
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("supervisor policy digest is unavailable");
        }
        validate_confinement(&policy.linux_confinement)?;
        Ok(Self {
            confinement: policy.linux_confinement,
        })
    }
}

fn validate_confinement(
    policy: &ExternalPoolAdapterSupervisorLinuxConfinementPolicy,
) -> Result<()> {
    let launch = &policy.launch;
    let identity = &policy.identity;
    let filesystem = &policy.filesystem;
    let cgroup = &policy.cgroup;
    let descriptors = &policy.descriptors;
    let seccomp = &policy.seccomp;
    let shutdown = &policy.shutdown;

    if policy.host_os != "linux"
        || policy.host_arch != "x86_64"
        || launch.primitive != "clone3_v1"
        || !launch.require_clone_pidfd
        || !launch.require_clone_into_cgroup
        || launch.fallback_allowed
        || launch.pid_namespace_enabled
        || !launch.user_namespace_enabled
        || !launch.mount_namespace_enabled
        || !launch.network_namespace_enabled
        || !launch.ipc_namespace_enabled
        || !launch.uts_namespace_enabled
    {
        bail!("supervisor launch policy drifted");
    }
    if identity.mapping != "map_supervisor_euid_egid_to_child_root_v1"
        || !identity.deny_setgroups
        || !identity.clear_all_capability_sets
        || !identity.no_new_privileges
        || identity.dumpable
        || identity.umask != 0o077
        || !identity.create_session
    {
        bail!("supervisor identity policy drifted");
    }
    if filesystem.mount_propagation != "private_recursive_v1"
        || filesystem.root_filesystem != "empty_tmpfs_nodev_nosuid_noexec_pivot_root_v1"
        || !filesystem.pivot_root_required
        || filesystem.proc_mounted
        || filesystem.sys_mounted
        || filesystem.dev_mounted
        || filesystem.working_directory != "private_tmpfs_tmp_v1"
        || filesystem.tmp_mount_flags != ["nodev", "nosuid", "noexec"]
        || filesystem.tmp_mode != 0o700
        || filesystem.tmp_limit_bytes != 67_108_864
    {
        bail!("supervisor filesystem policy drifted");
    }
    if cgroup.hierarchy != "cgroup_v2_dedicated_leaf_v1"
        || !cgroup.dedicated_leaf_required
        || cgroup.pids_max != 1
        || cgroup.memory_max_bytes != 268_435_456
        || cgroup.memory_swap_max_bytes != 0
        || !cgroup.memory_oom_group
        || cgroup.cpu_quota_us != 100_000
        || cgroup.cpu_period_us != 100_000
    {
        bail!("supervisor cgroup policy drifted");
    }
    if descriptors.stdin_fd != 0
        || descriptors.stdout_fd != 1
        || descriptors.stderr_fd != 2
        || descriptors.child_ipc_fd != CHILD_IPC_FD as u64
        || descriptors.capsule_fd != CAPSULE_FD as u64
        || descriptors.seed_fd != SEED_FD as u64
        || descriptors.capsule_fd_cloexec != true
        || descriptors.seed_fd_cloexec
        || descriptors.seed_fd_bytes != 32
        || !descriptors.seed_fd_close_after_read
        || descriptors.close_range_from_fd != FIRST_CLOSED_FD as u64
        || !descriptors.close_range_unshare
        || descriptors.post_exec_open_fds != [0, 1, 2, 3, 5]
        || descriptors.post_seed_open_fds != [0, 1, 2, 3]
        || !descriptors.child_ipc_fd_allowed
        || descriptors.child_network_or_target_fd_allowed
    {
        bail!("supervisor descriptor policy drifted");
    }
    if seccomp.architecture != "x86_64"
        || seccomp.unknown_syscall_action != "kill_process"
        || seccomp.audit_arch_policy != "x86_64_only_kill_other_arch"
        || seccomp.exec_rule != "single_execveat_capsule_fd_4_at_empty_path_v1"
        || !seccomp.deny_new_executable_mappings_after_exec
        || !seccomp.deny_process_creation
        || !seccomp.deny_network_syscalls
        || !seccomp.deny_mount_namespace_capability_keyring_ptrace_bpf_perf_io_uring
        || seccomp.argument_rules
            != [
                "execveat_fd4_empty_path_only",
                "mmap_prot_exec_denied",
                "mprotect_prot_exec_denied",
                "fcntl_dup_denied",
                "ioctl_denied",
            ]
    {
        bail!("supervisor seccomp policy drifted");
    }
    if policy.network_policy
        != "child_newnet_no_interface_no_network_or_target_fd_ipc_fd3_only_server_broker_v1"
        || policy.stderr_limit_bytes != STDERR_LIMIT_BYTES as u64
        || policy.stderr_overflow_policy != "terminate_session_v1"
        || shutdown.process_handle != "pidfd_only_v1"
        || shutdown.initial_signal != "SIGTERM"
        || shutdown.grace_ms != SHUTDOWN_GRACE_MS
        || shutdown.terminal_signal != "SIGKILL"
        || shutdown.reap != "waitid_pidfd_v1"
        || shutdown.pid_fallback_allowed
        || shutdown.descendant_policy != "single_process_cgroup_fail_closed_v1"
    {
        bail!("supervisor lifecycle policy drifted");
    }
    Ok(())
}
