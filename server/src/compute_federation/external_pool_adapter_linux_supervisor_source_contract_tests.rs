const FACADE: &str = include_str!("external_pool_adapter_linux_supervisor.rs");
const POLICY: &str = include_str!("external_pool_adapter_linux_supervisor/policy.rs");
const CGROUP: &str = include_str!("external_pool_adapter_linux_supervisor/cgroup.rs");
const CHILD: &str = include_str!("external_pool_adapter_linux_supervisor/child.rs");
const LAUNCH: &str = include_str!("external_pool_adapter_linux_supervisor/launch.rs");
const LIFECYCLE: &str = include_str!("external_pool_adapter_linux_supervisor/lifecycle.rs");
const SECCOMP: &str = include_str!("external_pool_adapter_linux_supervisor/seccomp.rs");
const SESSION_BOOTSTRAP: &str =
    include_str!("../../external-pool-adapter-session-core/src/bootstrap.rs");
const AUTHENTICATED_RUNTIME_TESTS: &str =
    include_str!("external_pool_adapter_linux_supervisor/authenticated_runtime_tests.rs");
const LINUX_KERNEL_TESTS: &str =
    include_str!("external_pool_adapter_linux_supervisor/linux_tests.rs");
const SESSION_FIXTURE: &str = include_str!("../external_pool_adapter_session_fixture_main.rs");
const CAPSULE_FACADE: &str = include_str!("external_pool_adapter_entrypoint_capsule.rs");
const COMPUTE_MOD: &str = include_str!("mod.rs");

#[test]
fn v261_is_linux_x86_64_private_and_modular() {
    assert!(COMPUTE_MOD.contains(
        "#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\n\
         pub(crate) mod external_pool_adapter_linux_supervisor;"
    ));
    assert!(
        COMPUTE_MOD.contains("mod external_pool_adapter_linux_supervisor_source_contract_tests;")
    );
    for module in [
        "mod cgroup;",
        "mod child;",
        "mod launch;",
        "mod lifecycle;",
        "mod policy;",
        "mod seccomp;",
    ] {
        assert!(FACADE.contains(module));
    }
    assert!(!FACADE.contains("axum"));
    assert!(!FACADE.contains("rusqlite"));
}

#[test]
fn v261_requires_clone3_pidfd_cgroup_and_exact_namespaces_without_fallback() {
    for required in [
        "libc::SYS_clone3",
        "libc::CLONE_PIDFD",
        "CLONE_INTO_CGROUP",
        "libc::CLONE_NEWUSER",
        "libc::CLONE_NEWNS",
        "libc::CLONE_NEWNET",
        "libc::CLONE_NEWIPC",
        "libc::CLONE_NEWUTS",
        "pid_namespace_enabled",
        "fallback_allowed",
    ] {
        assert!(format!("{POLICY}\n{LAUNCH}").contains(required));
    }
    let source = format!("{FACADE}\n{LAUNCH}\n{LIFECYCLE}");
    for forbidden in [
        "std::process::Command",
        "Command::new",
        "libc::fork",
        "libc::clone(",
        "waitpid(",
        "libc::kill(",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden fallback: {forbidden}"
        );
    }
}

#[test]
fn v261_configures_dedicated_cgroup_and_refuses_to_mutate_delegation() {
    for required in [
        "cgroup.controllers",
        "cgroup.subtree_control",
        "pids.max",
        "memory.max",
        "memory.swap.max",
        "memory.oom.group",
        "cpu.max",
        "CGROUP2_SUPER_MAGIC",
        "libc::mkdirat",
        "libc::unlinkat",
        "libc::O_NOFOLLOW",
    ] {
        assert!(CGROUP.contains(required));
    }
    assert!(!CGROUP.contains("+cpu"));
    assert!(!CGROUP.contains("+memory"));
    assert!(!CGROUP.contains("+pids"));
}

#[test]
fn v261_applies_root_fd_identity_resource_and_capability_confinement() {
    for required in [
        "libc::MS_REC | libc::MS_PRIVATE",
        "libc::MS_NODEV | libc::MS_NOSUID | libc::MS_NOEXEC",
        "libc::SYS_pivot_root",
        "libc::MNT_DETACH",
        "libc::RLIMIT_CORE",
        "libc::RLIMIT_NOFILE",
        "libc::RLIMIT_NPROC",
        "libc::RLIMIT_AS",
        "libc::RLIMIT_FSIZE",
        "libc::RLIMIT_STACK",
        "libc::RLIMIT_MEMLOCK",
        "libc::RLIMIT_CPU",
        "libc::PR_CAPBSET_DROP",
        "libc::SYS_capset",
        "libc::PR_SET_DUMPABLE",
        "libc::PR_SET_NO_NEW_PRIVS",
        "libc::SYS_close_range",
        "CLOSE_RANGE_UNSHARE",
        "libc::dup3(plan.child_ipc_fd, CHILD_IPC_FD, 0)",
        "libc::dup3(plan.capsule_fd, CAPSULE_FD, libc::O_CLOEXEC)",
        "libc::dup3(plan.seed_fd, SEED_FD, 0)",
    ] {
        assert!(CHILD.contains(required), "missing confinement: {required}");
    }
    for required in ["setgroups", "uid_map", "gid_map", "O_NOFOLLOW"] {
        assert!(LAUNCH.contains(required));
    }
}

#[test]
fn v261_seccomp_kills_unknown_arch_syscalls_and_restricts_exec_memory() {
    for required in [
        "AUDIT_ARCH_X86_64",
        "SECCOMP_RET_KILL_PROCESS",
        "SECCOMP_RET_ALLOW",
        "libc::PR_SET_SECCOMP",
        "libc::SYS_execveat",
        "CAPSULE_FD as u32",
        "libc::AT_EMPTY_PATH as u32",
        "libc::PROT_EXEC as u32",
        "libc::SYS_fcntl",
        "libc::F_GETFD as u32",
        "libc::SYS_poll",
        "BPF_JGT",
        "append_getfd_rule",
        "append_bounded_poll_rule",
        "argument_low_offset(0)",
        "argument_low_offset(1)",
        "argument_low_offset(2)",
        "unsupported supervisor syscall policy entry",
    ] {
        assert!(SECCOMP.contains(required));
    }
    assert!(CHILD.contains("libc::SYS_execveat"));
    assert!(CHILD.contains("CAPSULE_FD"));
    assert!(CHILD.contains("libc::AT_EMPTY_PATH"));
    assert!(!CHILD.contains("execve("));
    for required in [
        "linux_kernel_seccomp_rejects_unapproved_poll_shape",
        "linux_kernel_seccomp_rejects_fcntl_descriptor_duplication",
        "libc::SYS_poll as u32",
        "libc::SYS_fcntl as u32",
        "libc::F_DUPFD_CLOEXEC as u32",
    ] {
        assert!(
            LINUX_KERNEL_TESTS.contains(required),
            "missing V261 kernel seccomp regression {required}"
        );
    }
}

#[test]
fn v261_uses_pidfd_only_bounded_stderr_and_existing_v257_v260_seams() {
    for required in [
        "libc::SYS_pidfd_send_signal",
        "libc::waitid(",
        "P_PIDFD",
        "libc::SIGTERM",
        "libc::SIGKILL",
        "STDERR_LIMIT_BYTES",
        "supervisor stderr exceeded server-fixed bound",
    ] {
        assert!(LIFECYCLE.contains(required));
    }
    assert!(SESSION_BOOTSTRAP.contains("into_supervisor_descriptors"));
    assert!(CAPSULE_FACADE.contains("ExternalPoolAdapterSupervisorCapsule"));
    assert!(CAPSULE_FACADE.contains("retained_sealed_image"));
    assert!(LAUNCH.contains("REQUIRED_CAPSULE_SEALS"));
    assert!(LAUNCH.contains("FD_CLOEXEC"));
}

#[test]
fn v262_passes_only_fixed_root_arguments_and_reuses_fd3_fd5_session_core() {
    for required in [
        "set_blocking(child_ipc.as_raw_fd())?",
        "--elon-session-policy=",
        "--elon-session-profile=",
        "--elon-session-target=",
        "--elon-session-companion=",
        "--elon-session-capsule=",
        "--elon-session-bundle=",
        "argv: [CString; 7]",
        "ExternalPoolAdapterChildBootstrap::adopt_supervisor_descriptors()",
        "host.authenticate()",
        "open_fds(pid), BTreeSet::from([0, 1, 2, 3])",
    ] {
        assert!(
            format!("{LAUNCH}\n{CHILD}\n{AUTHENTICATED_RUNTIME_TESTS}\n{SESSION_FIXTURE}")
                .contains(required),
            "missing V262 authenticated runtime rule {required}"
        );
    }
    assert!(!LAUNCH.contains("credential"));
    assert!(!LAUNCH.contains("config_bytes"));
    assert!(!LAUNCH.contains("std::process::Command"));
}

#[test]
fn v261_has_no_network_secret_activation_market_settlement_or_chain_consumer() {
    let source = format!("{FACADE}\n{POLICY}\n{CGROUP}\n{CHILD}\n{LAUNCH}\n{LIFECYCLE}\n{SECCOMP}");
    for forbidden in [
        "TcpStream",
        "UdpSocket",
        "reqwest",
        "tokio::net",
        "credential_bytes",
        "config_bytes",
        "provider_status",
        "activation_ready = true",
        "route_authority",
        "market_admission",
        "settlement",
        "sui_client",
        "axum::",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden V261 effect: {forbidden}"
        );
    }
}
