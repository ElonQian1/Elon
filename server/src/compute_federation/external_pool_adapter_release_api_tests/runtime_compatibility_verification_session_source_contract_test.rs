const SESSION_ROOTS: &str =
    include_str!("../../../external-pool-adapter-session-core/src/roots.rs");
const SESSION_NO_WORK: &str =
    include_str!("../../../external-pool-adapter-session-core/src/no_work.rs");
const SESSION_TRANSPORT_IO: &str =
    include_str!("../../../external-pool-adapter-session-core/src/transport_io.rs");
const CAPSULE_FACADE: &str = include_str!("../external_pool_adapter_entrypoint_capsule.rs");
const CAPSULE_LINUX: &str = include_str!("../external_pool_adapter_entrypoint_capsule/linux.rs");
const CAPSULE_ELF: &str = include_str!("../external_pool_adapter_entrypoint_capsule/elf.rs");
const LAUNCH_IMAGE: &str =
    include_str!("../external_pool_adapter_entrypoint_capsule/launch_image.rs");
const SUPERVISOR_LAUNCH: &str = include_str!("../external_pool_adapter_linux_supervisor/launch.rs");
const SUPERVISOR_CHILD: &str = include_str!("../external_pool_adapter_linux_supervisor/child.rs");
const SUPERVISOR_LIFECYCLE: &str =
    include_str!("../external_pool_adapter_linux_supervisor/lifecycle.rs");
const SESSION_FIXTURE: &str = include_str!("../../external_pool_adapter_session_fixture_main.rs");
const STORE_RUN: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/run.rs"
);
const STORE_RUN_EXECUTION: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/run/execution.rs"
);
const STORE_RUN_SUPPORT: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/run/support.rs"
);
const STORE_TYPES: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/types.rs"
);
const STORE_ROOT: &str = include_str!("../../store.rs");
const SERVICE: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification_service.rs");

const EXACT_RUNTIME_COMPATIBILITY_ROOTS: &[&str] = &[
    "supervisor_session_policy_digest",
    "runtime_compatibility_profile_digest",
    "challenge_digest",
    "runner_policy_digest",
    "fixture_catalog_digest",
    "sandbox_verifier_key_record_digest",
    "registry_release_digest",
    "installation_content_digest",
    "source_capsule_sha256",
    "launch_image_sha256",
    "public_fixture_delivery_root",
];

const EXACT_RUNTIME_COMPATIBILITY_PREFIXES: &[&str] = &[
    "--elon-runtime-compatibility-session-policy=",
    "--elon-runtime-compatibility-profile=",
    "--elon-runtime-compatibility-challenge=",
    "--elon-runtime-compatibility-runner-policy=",
    "--elon-runtime-compatibility-fixture-catalog=",
    "--elon-runtime-compatibility-sandbox-verifier-key-record=",
    "--elon-runtime-compatibility-registry-release=",
    "--elon-runtime-compatibility-installation-content=",
    "--elon-runtime-compatibility-source-capsule=",
    "--elon-runtime-compatibility-launch-image=",
    "--elon-runtime-compatibility-public-delivery=",
];

#[test]
fn runtime_compatibility_session_source_freezes_dedicated_eleven_root_abi() {
    let constructor = SESSION_ROOTS
        .split_once("pub fn new_runtime_compatibility(")
        .unwrap()
        .1
        .split_once("pub fn launch_arguments")
        .unwrap()
        .0;
    assert_eq!(
        constructor
            .split_once(") -> Result<Self>")
            .unwrap()
            .0
            .matches(": &str")
            .count(),
        11
    );
    for root in EXACT_RUNTIME_COMPATIBILITY_ROOTS {
        assert!(
            constructor.contains(root),
            "missing V268 session root {root}"
        );
        assert!(
            SESSION_ROOTS.contains(&format!("b\"{root}\\0\"")),
            "missing V268 transcript label {root}"
        );
    }
    for forbidden in ["target_digest", "companion_digest", "bundle_digest"] {
        assert!(
            !constructor.contains(forbidden),
            "production slot leaked into V268 constructor: {forbidden}"
        );
    }
    assert!(SESSION_ROOTS.contains("RuntimeCompatibility([String; 11])"));
    assert!(SESSION_ROOTS.contains(
        "elon.external_pool_adapter.runtime_compatibility_verification.session.roots.v1\\0"
    ));
    assert!(SESSION_ROOTS.contains(
        "elon.external_pool_adapter.runtime_compatibility_verification.session.kdf_salt.v1\\0"
    ));
}

#[test]
fn runtime_compatibility_session_source_freezes_exact_argv_order_at_both_ends() {
    assert!(SUPERVISOR_LAUNCH.contains("RUNTIME_COMPATIBILITY_ROOT_ARGUMENT_PREFIXES: [&str; 11]"));
    assert!(SESSION_FIXTURE.contains("RUNTIME_COMPATIBILITY_ROOT_ARGUMENT_PREFIXES: [&str; 11]"));
    for prefix in EXACT_RUNTIME_COMPATIBILITY_PREFIXES {
        assert!(
            SUPERVISOR_LAUNCH.contains(prefix),
            "supervisor missing argv prefix {prefix}"
        );
        assert!(
            SESSION_FIXTURE.contains(prefix),
            "child fixture missing argv prefix {prefix}"
        );
    }
    for required in [
        "roots.runtime_compatibility_values()",
        "parse_runtime_compatibility_roots(&arguments[1..])",
        "ExternalPoolAdapterSessionRoots::new_runtime_compatibility(",
    ] {
        let source = format!("{SUPERVISOR_LAUNCH}{SESSION_FIXTURE}");
        assert!(
            source.contains(required),
            "missing dedicated V268 ABI use {required}"
        );
    }
}

#[test]
fn runtime_compatibility_session_source_preserves_legacy_six_root_abi() {
    for required in [
        "elon.external_pool_adapter.supervisor_session.roots.v1\\0",
        "elon.external_pool_adapter.supervisor_session.kdf_salt.v1\\0",
        "Production([String; 6])",
        "--elon-session-policy=",
        "--elon-session-profile=",
        "--elon-session-target=",
        "--elon-session-companion=",
        "--elon-session-capsule=",
        "--elon-session-bundle=",
    ] {
        let source = format!("{SESSION_ROOTS}{SESSION_FIXTURE}");
        assert!(
            source.contains(required),
            "legacy session ABI drifted: {required}"
        );
    }
    let production_constructor = SESSION_ROOTS
        .split_once("pub fn new(")
        .unwrap()
        .1
        .split_once("pub fn new_runtime_compatibility(")
        .unwrap()
        .0;
    assert_eq!(production_constructor.matches(": &str").count(), 6);
}

#[test]
fn runtime_compatibility_runner_policy_is_bound_to_v267_v265_implementation_paths() {
    for required in [
        "validate_static_elf64_x86_64(source_file, expected_size)",
        "validate_static_elf64_x86_64(&source_capsule, expected_size)",
        "derive_launch_image(&source_capsule)",
    ] {
        assert!(
            CAPSULE_LINUX.contains(required),
            "source capsule no longer validates before deriving launch image: {required}"
        );
    }
    for required in [
        "u16_at(&header, 16) != ET_EXEC",
        "flags & PF_X != 0 && flags & PF_W != 0",
        "kind == PT_GNU_STACK && flags & PF_X != 0",
    ] {
        assert!(
            CAPSULE_ELF.contains(required),
            "V267 static ET_EXEC safety gate drifted: {required}"
        );
    }
    for required in [
        "relocate_headers(",
        "copy_relocated_ranges(",
        "header.set_kind(PT_LOAD)",
        "header.set_flags(PF_R | PF_X)",
    ] {
        assert!(
            LAUNCH_IMAGE.contains(required),
            "V267 relocated RX launch image drifted: {required}"
        );
    }
    assert!(CAPSULE_FACADE.contains("&self.launch_image"));

    let stub = LAUNCH_IMAGE
        .split_once("fn build_stub(")
        .unwrap()
        .1
        .split_once("fn append_prctl(")
        .unwrap()
        .0;
    assert_ordered(
        stub,
        &[
            "libc::PR_SET_DUMPABLE",
            "libc::PR_GET_DUMPABLE",
            "original_entry.to_le_bytes()",
        ],
    );
    assert!(SUPERVISOR_CHILD.contains("libc::SYS_execveat"));
    assert!(SUPERVISOR_CHILD.contains("libc::AT_EMPTY_PATH"));

    for required in [
        "require_exec_transition_ptrace_guard(&policy)",
        "libc::O_CLOEXEC | libc::O_NOFOLLOW",
        "matches!(observed[0], b'2' | b'3')",
    ] {
        assert!(
            SUPERVISOR_LAUNCH.contains(required),
            "V267 Yama exec-transition guard drifted: {required}"
        );
    }
    for required in [
        "libc::recvmsg(fd, &mut message, libc::MSG_TRUNC)",
        "libc::MSG_TRUNC | libc::MSG_CTRUNC",
        "message.msg_controllen != 0",
        "unexpected control data",
    ] {
        assert!(
            SESSION_TRANSPORT_IO.contains(required),
            "V267 seqpacket ancillary rejection drifted: {required}"
        );
    }
    for required in [
        "const PROBE_MAGIC: &[u8; 4] = b\"ELNW\"",
        "const PROBE_VERSION: u8 = 1",
        "ExternalPoolAdapterNoWorkProbeHostReceipt",
        "valid_receipt(",
    ] {
        assert!(
            SESSION_NO_WORK.contains(required),
            "V265 ELNW v1 receipt path drifted: {required}"
        );
    }
}

#[test]
fn runtime_compatibility_private_run_commits_only_after_shutdown_reap_and_cleanup() {
    assert!(STORE_RUN.contains(
        "pub(in crate::store) fn run_external_pool_adapter_runtime_compatibility_verification_challenge"
    ));
    assert!(
        !SERVICE.contains("run_external_pool_adapter_runtime_compatibility_verification_challenge")
    );
    let private_receipt = STORE_TYPES
        .split_once(
            "pub(in crate::store) struct ExternalPoolAdapterRuntimeCompatibilityRunObservationWriteReceipt",
        )
        .unwrap()
        .1
        .split_once('}')
        .unwrap()
        .0;
    for field in ["run_observation", "signature_challenge", "replayed"] {
        assert!(private_receipt.contains(field));
    }
    assert!(
        !STORE_ROOT.contains("ExternalPoolAdapterRuntimeCompatibilityRunObservationWriteReceipt")
    );
    assert_eq!(
        STORE_RUN.matches("TransactionBehavior::Immediate").count(),
        2
    );
    assert_ordered(
        STORE_RUN,
        &["tx.commit()?", "let fixtures =", "execution::execute("],
    );
    let fresh = STORE_RUN.split_once("let fixtures =").unwrap().1;
    assert_ordered(
        fresh,
        &[
            "execution::execute(",
            "transaction_with_behavior(TransactionBehavior::Immediate)",
            "run_observation_by_challenge_on(&tx, challenge_id)",
            "insert_run_observation(",
            "runtime_compatibility_signature_challenge(",
            "tx.commit()",
        ],
    );
    for required in [
        "external_pool_adapter_runtime_compatibility_session_roots(",
        "runner_policy.max_probe_timeout_ms != RUNTIME_COMPATIBILITY_MAX_PROBE_TIMEOUT_MS",
        "session_policy.state.probe_timeout_ms != runner_policy.max_probe_timeout_ms",
        "Duration::from_millis(session_policy.state.probe_timeout_ms)",
        "delivery_receipt.shutdown(&mut session)",
        ".wait(CHILD_EXIT_TIMEOUT)",
        "child.collect_stderr()",
    ] {
        assert!(
            STORE_RUN_EXECUTION.contains(required),
            "private runner lost a terminal evidence step: {required}"
        );
    }
    for required in [
        "binding.admission_id != release.admission_id",
        "binding.admission_digest != release.admission_digest",
        "binding.package_receipt_id != release.package_receipt_id",
        "binding.package_receipt_digest != release.package_receipt_digest",
        "binding.package_material_digest != release.package_material_digest",
        "binding.source_receipt_id != release.source_receipt_id",
        "binding.source_receipt_digest != release.source_receipt_digest",
        "binding.capability_set_digest != release.capability_set_digest",
    ] {
        assert!(
            STORE_RUN_SUPPORT.contains(required),
            "Prepared installation lost an exact V249 root: {required}"
        );
    }
    for required in [
        "waitid_pidfd(self.pidfd.as_raw_fd())",
        "self.cleanup_after_reap()",
        "supervisor cgroup cleanup failed after reap",
        "supervisor scratch cleanup failed after reap",
    ] {
        assert!(
            SUPERVISOR_LIFECYCLE.contains(required),
            "V267 reap/cleanup visibility drifted: {required}"
        );
    }
}

fn assert_ordered(source: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let offset = source[cursor..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing ordered source marker {needle}"));
        cursor += offset + needle.len();
    }
}
