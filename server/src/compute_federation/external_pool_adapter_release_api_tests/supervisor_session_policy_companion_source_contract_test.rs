use sha2::{Digest, Sha256};

const DOMAIN_POLICY: &str =
    include_str!("../external_pool_adapter_supervisor_session_policy_companion/policy.rs");
const DOMAIN_TYPES: &str =
    include_str!("../external_pool_adapter_supervisor_session_policy_companion/types.rs");
const SERVICE: &str =
    include_str!("../external_pool_adapter_supervisor_session_policy_companion_service.rs");
const VALIDATION: &str = include_str!(
    "../external_pool_adapter_supervisor_session_policy_companion_service_validation.rs"
);
const API: &str =
    include_str!("../external_pool_adapter_supervisor_session_policy_companion_api.rs");
const REDACTION: &str = include_str!(
    "../external_pool_adapter_supervisor_session_policy_companion_service_redaction.rs"
);
const STORE_FACADE: &str = include_str!(
    "../../store/compute_external_pool_adapter_supervisor_session_policy_companion.rs"
);
const STORE_CURRENT: &str = include_str!(
    "../../store/compute_external_pool_adapter_supervisor_session_policy_companion/current.rs"
);
const STORE_POLICY: &str = include_str!(
    "../../store/compute_external_pool_adapter_supervisor_session_policy_companion/policy.rs"
);
const STORE_READ: &str = include_str!(
    "../../store/compute_external_pool_adapter_supervisor_session_policy_companion/read.rs"
);
const STORE_WRITE: &str = include_str!(
    "../../store/compute_external_pool_adapter_supervisor_session_policy_companion/write.rs"
);
const V254_FENCES: &str = include_str!(
    "../../store_migrations/compute_external_pool_provider_activation_candidate/guards/fences.rs"
);

#[test]
fn supervisor_session_http_source_freezes_exact_owner_admin_and_store_abi() {
    for required in [
        "expected_target_digest",
        "expected_profile_digest",
        "expected_candidate_digest",
        "expected_provider_binding_digest",
        "expected_supervisor_session_policy_digest",
        "expected_predecessor: Option<ExpectedSupervisorSessionPolicyCompanionPredecessor>",
        "ProviderOwner(String)",
        "PlatformAdmin(String)",
        "Self::ProviderOwner",
        "Self::PlatformAdmin",
        "audit_external_pool_adapter_installation",
        "external_pool_adapter_supervisor_session_policy_summary",
        "create_external_pool_adapter_supervisor_session_policy_companion",
        "external_pool_adapter_supervisor_session_policy_companion_currentness",
        "revoke_external_pool_adapter_supervisor_session_policy_companion",
        "external_pool_adapter_supervisor_session_policy_companion_audit_target",
    ] {
        assert!(SERVICE.contains(required), "missing Service ABI {required}");
    }
    for method in [
        "external_pool_adapter_supervisor_session_policy_summary",
        "create_external_pool_adapter_supervisor_session_policy_companion",
        "external_pool_adapter_supervisor_session_policy_companion_currentness",
        "revoke_external_pool_adapter_supervisor_session_policy_companion",
        "external_pool_adapter_supervisor_session_policy_companion_audit_target",
    ] {
        let source =
            format!("{STORE_FACADE}{STORE_POLICY}{STORE_READ}{STORE_CURRENT}{STORE_WRITE}");
        assert!(
            source.contains(method),
            "missing frozen Store method {method}"
        );
    }
    for required in [
        "supervisor-session-policy",
        "supervisor-session-policy-companions",
        "owner_policy",
        "admin_policy",
        "owner_create",
        "admin_create",
        "owner_currentness",
        "admin_currentness",
        "owner_revoke",
        "admin_revoke",
        "JsonRejection",
        "auth_from_headers(state, headers)",
        "SupervisorSessionPolicyCompanionActor::ProviderOwner(user.id)",
        "Ok(SupervisorSessionPolicyCompanionActor::PlatformAdmin(",
        "matches!(user.role.as_str(), \"admin\" | \"owner\")",
    ] {
        assert!(API.contains(required), "missing HTTP boundary {required}");
    }
    assert_eq!(
        API.matches(".route(").count(),
        8,
        "owner/admin surface drifted"
    );
}

#[test]
fn supervisor_session_source_freezes_inert_projection_and_recursive_redaction() {
    for required in [
        "SUPERVISOR_SESSION_COMPANION_NO_EFFECT",
        "SUPERVISOR_SESSION_COMPANION_REVOCATION_EFFECT",
        "value.process_spawn_ready",
        "value.ipc_session_ready",
        "value.secret_delivery_ready",
        "value.broker_connect_ready",
        "value.upstream_probe_observed",
        "value.runtime_launch_ready",
        "value.activation_ready",
        "require_currentness_inert",
    ] {
        assert!(
            VALIDATION.contains(required),
            "missing inert guard {required}"
        );
    }
    for required in [
        "dns_hostname",
        "expected_tls_leaf_spki_sha256",
        "provider_owner_account_id",
        "recorded_by_actor_user_id",
        "revoked_by_actor_user_id",
        "idempotency_key",
        "credential_locator",
        "config_locator",
        "entrypoint_path",
        "session_key",
        "host_nonce",
        "child_nonce",
        "transcript_digest",
        "pidfd",
        "cgroup_path",
        "receipt_json",
        "map.values_mut().for_each(redact)",
    ] {
        assert!(
            REDACTION.contains(required),
            "missing redaction guard {required}"
        );
    }
}

#[test]
fn supervisor_session_policy_source_freezes_v1_and_current_v2_exactly() {
    for required in [
        "child_ipc_fd: 3",
        "capsule_fd: 4",
        "capsule_fd_cloexec: true",
        "seed_fd: 5",
        "seed_fd_bytes: 32",
        "seed_fd_cloexec: false",
        "seed_fd_read_phase: \"post_exec_before_hello_v1\"",
        "seed_fd_close_after_read: true",
        "close_range_from_fd: 6",
        "close_range_unshare: true",
        "post_exec_open_fds: [0, 1, 2, 3, 5]",
        "post_seed_open_fds: [0, 1, 2, 3]",
        "child_ipc_fd_allowed: true",
        "child_network_or_target_fd_allowed: false",
        "unknown_syscall_action: \"kill_process\"",
        "audit_arch_policy: \"x86_64_only_kill_other_arch\"",
        "execveat_fd4_empty_path_only",
        "mmap_prot_exec_denied",
        "mprotect_prot_exec_denied",
        "fcntl_getfd_fd3_fd5_only",
        "ioctl_denied",
        "SUPERVISOR_SESSION_POLICY_V1_ID",
        "SUPERVISOR_SESSION_POLICY_V2_ID",
        "policy_v1_for_validation",
        "policy_v2_for_validation",
        "historical_supervisor_session_policy_v1_catalog",
        "v255_intent_preserved_v267_post_exec_dumpable_launch_capsule_v2",
        "yama_ptrace_scope_2_or_stricter_v2",
        "&mut seccomp.bootstrap_allowed_syscalls",
        "insert_before(&mut seccomp.runtime_allowed_syscalls, \"prlimit64\", \"prctl\")",
        "prctl_dumpable_set_zero_or_get_only",
        "single_execveat_derived_launch_capsule_fd_4_at_empty_path_v2",
    ] {
        assert!(
            DOMAIN_POLICY.contains(required),
            "missing exact policy {required}"
        );
    }
    for field in [
        "bootstrap_allowed_syscalls",
        "runtime_allowed_syscalls",
        "argument_rules",
        "unknown_syscall_action",
        "audit_arch_policy",
    ] {
        assert!(DOMAIN_TYPES.contains(field), "missing seccomp ABI {field}");
    }
    let bootstrap = policy_array("bootstrap_allowed_syscalls");
    let runtime = policy_array("runtime_allowed_syscalls");
    let argument_rules = policy_array("argument_rules");
    assert_eq!(
        bootstrap,
        [
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
    );
    assert_eq!(runtime, bootstrap[..bootstrap.len() - 1]);
    assert_eq!(
        argument_rules,
        [
            "execveat_fd4_empty_path_only",
            "mmap_prot_exec_denied",
            "mprotect_prot_exec_denied",
            "fcntl_getfd_fd3_fd5_only",
            "ioctl_denied",
            "poll_nfds3_timeout0_or_nfds1_timeout1_5000_only",
        ]
    );
    assert!(DOMAIN_TYPES.contains("#[serde(default, skip_serializing_if = \"Option::is_none\")]"));
    assert!(DOMAIN_TYPES.contains("exec_transition_ptrace_guard: Option<String>"));
}

#[test]
fn supervisor_session_http_source_has_no_runtime_or_downstream_consumer() {
    let source = format!("{SERVICE}{VALIDATION}{API}{REDACTION}");
    for forbidden in [
        "std::process::Command",
        "tokio::process",
        "clone3(",
        "execveat(",
        "socketpair(",
        "TcpStream",
        "TcpListener",
        "reqwest::",
        "hickory_resolver",
        "rustls::",
        "LockedSensitiveBytes",
        "with_sensitive_bytes",
        "deliver_sensitive",
        "probe_external_pool",
        "activate_external_pool",
        "compute_capacity_pools",
        "compute_offers",
        "compute_jobs",
        "compute_attempt_start_outbox",
        "compute_attempt_settlements",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden V259 consumer {forbidden}"
        );
    }
    assert_eq!(source.matches("spawn_blocking").count(), 1);
    assert_eq!(
        source
            .matches("audit_external_pool_adapter_installation")
            .count(),
        2,
        "one import plus one filesystem audit call are required"
    );
}

#[test]
fn supervisor_session_source_preserves_v254_market_fences_exactly() {
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6"
    );
    assert_eq!(V254_TRIGGER_NAMES.len(), 18);
    for name in V254_TRIGGER_NAMES {
        assert!(V254_FENCES.contains(name), "missing V254 fence {name}");
    }
}

fn policy_array(field: &str) -> Vec<&'static str> {
    let source = DOMAIN_POLICY
        .split_once(&format!("{field}: ["))
        .unwrap()
        .1
        .split_once(']')
        .unwrap()
        .0;
    source
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix('"')?
                .strip_suffix(",")
                .or_else(|| line.trim().strip_prefix('"')?.strip_suffix('"'))
        })
        .map(|value| value.trim_end_matches('"'))
        .collect()
}

const V254_TRIGGER_NAMES: &[&str] = &[
    "v254_external_pool_provider_activation_fence",
    "v254_external_pool_provider_insert_active_fence",
    "v254_external_pool_provider_identity_update_fence",
    "v254_external_pool_provider_kind_update_fence",
    "v254_external_pool_provider_version_active_fence",
    "v254_external_pool_candidate_projection_adapter_fence",
    "v254_external_pool_candidate_projection_adapter_version_fence",
    "v254_external_pool_candidate_service_actor_fence",
    "v254_external_pool_route_credential_fence",
    "v254_external_pool_route_authorization_fence",
    "v254_external_pool_route_capability_fence",
    "v254_external_pool_route_seal_fence",
    "v254_external_pool_capacity_pool_insert_active_fence",
    "v254_external_pool_capacity_pool_update_active_fence",
    "v254_external_pool_capacity_pool_version_active_fence",
    "v254_external_pool_offer_insert_market_fence",
    "v254_external_pool_offer_update_market_fence",
    "v254_external_pool_offer_version_market_fence",
];
