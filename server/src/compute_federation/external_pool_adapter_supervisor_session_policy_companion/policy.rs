use anyhow::Result;

use super::*;

pub(super) const SUPERVISOR_SESSION_POLICY_V1_ID: &str =
    "external_pool_adapter_supervisor_session_policy_v1";
pub(super) const SUPERVISOR_SESSION_POLICY_V1_REVISION: u64 = 1;
pub(super) const SUPERVISOR_SESSION_POLICY_V2_ID: &str =
    "external_pool_adapter_supervisor_session_policy_v2";
pub(super) const SUPERVISOR_SESSION_POLICY_V2_REVISION: u64 = 2;

pub(crate) fn server_supervisor_session_policy_catalog(
) -> Result<(ExternalPoolAdapterSupervisorSessionPolicy, String)> {
    let policy = policy_for_validation();
    validate_supervisor_session_policy(&policy)?;
    let digest = supervisor_session_policy_digest(&policy)?;
    Ok((policy, digest))
}

/// Frozen V259 catalog used only to validate and reproduce historical V1 evidence.
pub(crate) fn historical_supervisor_session_policy_v1_catalog(
) -> Result<(ExternalPoolAdapterSupervisorSessionPolicy, String)> {
    let policy = policy_v1_for_validation();
    validate_embedded_supervisor_session_policy_shape(&policy)?;
    let digest = supervisor_session_policy_digest(&policy)?;
    Ok((policy, digest))
}

pub(super) fn policy_for_validation() -> ExternalPoolAdapterSupervisorSessionPolicy {
    policy_v2_for_validation()
}

pub(super) fn policy_v1_for_validation() -> ExternalPoolAdapterSupervisorSessionPolicy {
    ExternalPoolAdapterSupervisorSessionPolicy {
        policy_id: SUPERVISOR_SESSION_POLICY_V1_ID.into(),
        policy_revision: SUPERVISOR_SESSION_POLICY_V1_REVISION,
        supervisor_owner: "server_authenticated_supervisor_v1".into(),
        compatibility: "v255_intent_preserved_v259_dual_frame_realization_v1".into(),
        wire: wire_policy(),
        crypto: crypto_policy(),
        state: state_policy(),
        linux_confinement: linux_confinement_policy_v1(),
    }
}

pub(super) fn policy_v2_for_validation() -> ExternalPoolAdapterSupervisorSessionPolicy {
    let mut policy = policy_v1_for_validation();
    policy.policy_id = SUPERVISOR_SESSION_POLICY_V2_ID.into();
    policy.policy_revision = SUPERVISOR_SESSION_POLICY_V2_REVISION;
    policy.compatibility = "v255_intent_preserved_v267_post_exec_dumpable_launch_capsule_v2".into();
    let identity = &mut policy.linux_confinement.identity;
    identity.exec_transition_ptrace_guard = Some("yama_ptrace_scope_2_or_stricter_v2".into());
    let seccomp = &mut policy.linux_confinement.seccomp;
    insert_before(
        &mut seccomp.bootstrap_allowed_syscalls,
        "prlimit64",
        "prctl",
    );
    insert_before(&mut seccomp.runtime_allowed_syscalls, "prlimit64", "prctl");
    seccomp
        .argument_rules
        .push("prctl_dumpable_set_zero_or_get_only".into());
    seccomp.exec_rule = "single_execveat_derived_launch_capsule_fd_4_at_empty_path_v2".into();
    policy
}

fn insert_before(values: &mut Vec<String>, before: &str, value: &str) {
    let index = values
        .iter()
        .position(|entry| entry == before)
        .unwrap_or(values.len());
    values.insert(index, value.into());
}

fn wire_policy() -> ExternalPoolAdapterSupervisorSessionWirePolicy {
    ExternalPoolAdapterSupervisorSessionWirePolicy {
        transport: "anonymous_child_socketpair_seqpacket_v1".into(),
        protocol_id: "elon.external_pool_adapter.sidecar.v1".into(),
        protocol_revision: 1,
        legacy_launch_profile_framing: "u32_be_length_prefixed_utf8_jcs_v1".into(),
        framing: "elon_external_pool_sidecar_dual_frame_v1".into(),
        frame_magic_ascii: "ELSP".into(),
        frame_header_bytes: 20,
        frame_mac_bytes: 32,
        header_field_order: [
            "magic",
            "version_u8",
            "kind_u8",
            "flags_u16be",
            "sequence_u64be",
            "payload_length_u32be",
        ]
        .map(str::to_string)
        .into(),
        frame_kind_control: 1,
        frame_kind_config: 2,
        frame_kind_credential: 3,
        control_encoding: "strict_utf8_rfc8785_jcs_v1".into(),
        binary_encoding: "raw_exact_bytes_v1".into(),
        max_control_payload_bytes: 1_048_576,
        max_config_payload_bytes: 1_048_576,
        max_credential_payload_bytes: 65_536,
        max_frames_per_direction: 1_048_576,
        unknown_kind_policy: "fail_closed_shutdown_v1".into(),
    }
}

fn crypto_policy() -> ExternalPoolAdapterSupervisorSessionCryptoPolicy {
    ExternalPoolAdapterSupervisorSessionCryptoPolicy {
        seed_policy: "host_os_csprng_memory_only_inherited_seed_fd_v1".into(),
        seed_bytes: 32,
        nonce_policy: "host_and_child_unique_csprng_nonce_v1".into(),
        nonce_bytes: 32,
        kdf: "hkdf_sha256_extract_expand_v1".into(),
        kdf_salt: "sha256_protocol_policy_profile_target_host_nonce_child_nonce_v1".into(),
        kdf_context: "direction_and_full_root_transcript_v1".into(),
        directional_key_bytes: 32,
        mac: "hmac_sha256_32_v1".into(),
        mac_coverage: "domain_separator_header_and_payload_v1".into(),
        sequence_policy: "independent_directional_u64_from_1_exact_increment_no_replay_v1".into(),
        transcript_policy: "policy_profile_target_capsule_and_both_nonces_v1".into(),
        key_custody: "memory_only_zeroize_no_clone_serde_debug_log_db_http_v1".into(),
    }
}

fn state_policy() -> ExternalPoolAdapterSupervisorSessionStatePolicy {
    ExternalPoolAdapterSupervisorSessionStatePolicy {
        state_machine: "bootstrap_hello_authenticated_config_credential_no_work_shutdown_reaped_v1"
            .into(),
        host_states: [
            "bootstrap",
            "hello",
            "authenticated",
            "config",
            "credential",
            "no_work",
            "shutdown",
            "reaped",
        ]
        .map(str::to_string)
        .into(),
        child_states: [
            "bootstrap",
            "hello",
            "authenticated",
            "config",
            "credential",
            "no_work",
            "shutdown",
        ]
        .map(str::to_string)
        .into(),
        terminal_states: ["shutdown", "reaped"].map(str::to_string).into(),
        invalid_transition_policy: "fail_closed_authenticated_shutdown_v1".into(),
        authentication_failure_policy: "constant_time_fail_closed_no_oracle_v1".into(),
        startup_timeout_ms: 10_000,
        handshake_timeout_ms: 5_000,
        sensitive_delivery_timeout_ms: 5_000,
        probe_timeout_ms: 15_000,
        shutdown_timeout_ms: 5_000,
        reap_timeout_ms: 5_000,
    }
}

fn linux_confinement_policy_v1() -> ExternalPoolAdapterSupervisorLinuxConfinementPolicy {
    ExternalPoolAdapterSupervisorLinuxConfinementPolicy {
        host_os: "linux".into(),
        host_arch: "x86_64".into(),
        launch: ExternalPoolAdapterSupervisorLinuxLaunchPolicy {
            primitive: "clone3_v1".into(),
            require_clone_pidfd: true,
            require_clone_into_cgroup: true,
            fallback_allowed: false,
            pid_namespace_enabled: false,
            user_namespace_enabled: true,
            mount_namespace_enabled: true,
            network_namespace_enabled: true,
            ipc_namespace_enabled: true,
            uts_namespace_enabled: true,
        },
        identity: ExternalPoolAdapterSupervisorLinuxIdentityPolicy {
            mapping: "map_supervisor_euid_egid_to_child_root_v1".into(),
            deny_setgroups: true,
            clear_all_capability_sets: true,
            no_new_privileges: true,
            dumpable: false,
            exec_transition_ptrace_guard: None,
            umask: 0o077,
            create_session: true,
        },
        filesystem: ExternalPoolAdapterSupervisorLinuxFilesystemPolicy {
            mount_propagation: "private_recursive_v1".into(),
            root_filesystem: "empty_tmpfs_nodev_nosuid_noexec_pivot_root_v1".into(),
            pivot_root_required: true,
            proc_mounted: false,
            sys_mounted: false,
            dev_mounted: false,
            working_directory: "private_tmpfs_tmp_v1".into(),
            tmp_mount_flags: ["nodev", "nosuid", "noexec"].map(str::to_string).into(),
            tmp_mode: 0o700,
            tmp_limit_bytes: 67_108_864,
        },
        cgroup: ExternalPoolAdapterSupervisorLinuxCgroupPolicy {
            hierarchy: "cgroup_v2_dedicated_leaf_v1".into(),
            dedicated_leaf_required: true,
            pids_max: 1,
            memory_max_bytes: 268_435_456,
            memory_swap_max_bytes: 0,
            memory_oom_group: true,
            cpu_quota_us: 100_000,
            cpu_period_us: 100_000,
        },
        rlimits: ExternalPoolAdapterSupervisorLinuxRlimitPolicy {
            core_bytes: 0,
            nofile: 64,
            nproc: 1,
            address_space_bytes: 268_435_456,
            file_size_bytes: 67_108_864,
            stack_bytes: 8_388_608,
            memlock_bytes: 0,
            cpu_seconds: 30,
        },
        descriptors: ExternalPoolAdapterSupervisorLinuxDescriptorPolicy {
            stdin_fd: 0,
            stdin_policy: "read_only_dev_null_v1".into(),
            stdout_fd: 1,
            stdout_policy: "bounded_discard_v1".into(),
            stderr_fd: 2,
            stderr_policy: "bounded_supervisor_capture_v1".into(),
            child_ipc_fd: 3,
            child_ipc_fd_policy: "anonymous_sock_seqpacket_child_endpoint_v1".into(),
            seed_fd: 5,
            seed_fd_bytes: 32,
            seed_fd_cloexec: false,
            seed_fd_read_phase: "post_exec_before_hello_v1".into(),
            seed_fd_close_after_read: true,
            capsule_fd: 4,
            capsule_fd_cloexec: true,
            close_range_from_fd: 6,
            close_range_unshare: true,
            post_exec_open_fds: [0, 1, 2, 3, 5].into(),
            post_seed_open_fds: [0, 1, 2, 3].into(),
            child_ipc_fd_allowed: true,
            child_network_or_target_fd_allowed: false,
        },
        seccomp: ExternalPoolAdapterSupervisorLinuxSeccompPolicy {
            architecture: "x86_64".into(),
            unknown_syscall_action: "kill_process".into(),
            audit_arch_policy: "x86_64_only_kill_other_arch".into(),
            bootstrap_allowed_syscalls: [
                "read",
                "write",
                "close",
                "fcntl",
                "poll",
                "recvmsg",
                "sendmsg",
                "exit",
                "exit_group",
                "rt_sigaction",
                "rt_sigprocmask",
                "rt_sigreturn",
                "sigaltstack",
                "brk",
                "mmap",
                "mprotect",
                "munmap",
                "madvise",
                "futex",
                "clock_gettime",
                "arch_prctl",
                "set_tid_address",
                "set_robust_list",
                "rseq",
                "getrandom",
                "getpid",
                "gettid",
                "prlimit64",
                "execveat",
            ]
            .map(str::to_string)
            .into(),
            runtime_allowed_syscalls: [
                "read",
                "write",
                "close",
                "fcntl",
                "poll",
                "recvmsg",
                "sendmsg",
                "exit",
                "exit_group",
                "rt_sigaction",
                "rt_sigprocmask",
                "rt_sigreturn",
                "sigaltstack",
                "brk",
                "mmap",
                "mprotect",
                "munmap",
                "madvise",
                "futex",
                "clock_gettime",
                "arch_prctl",
                "set_tid_address",
                "set_robust_list",
                "rseq",
                "getrandom",
                "getpid",
                "gettid",
                "prlimit64",
            ]
            .map(str::to_string)
            .into(),
            argument_rules: [
                "execveat_fd4_empty_path_only",
                "mmap_prot_exec_denied",
                "mprotect_prot_exec_denied",
                "fcntl_getfd_fd3_fd5_only",
                "ioctl_denied",
                "poll_nfds3_timeout0_or_nfds1_timeout1_5000_only",
            ]
            .map(str::to_string)
            .into(),
            exec_rule: "single_execveat_capsule_fd_4_at_empty_path_v1".into(),
            deny_new_executable_mappings_after_exec: true,
            deny_process_creation: true,
            deny_network_syscalls: true,
            deny_mount_namespace_capability_keyring_ptrace_bpf_perf_io_uring: true,
        },
        network_policy:
            "child_newnet_no_interface_no_network_or_target_fd_ipc_fd3_only_server_broker_v1".into(),
        stderr_limit_bytes: 1_048_576,
        stderr_overflow_policy: "terminate_session_v1".into(),
        shutdown: ExternalPoolAdapterSupervisorLinuxShutdownPolicy {
            process_handle: "pidfd_only_v1".into(),
            initial_signal: "SIGTERM".into(),
            grace_ms: 5_000,
            terminal_signal: "SIGKILL".into(),
            reap: "waitid_pidfd_v1".into(),
            pid_fallback_allowed: false,
            descendant_policy: "single_process_cgroup_fail_closed_v1".into(),
        },
    }
}
