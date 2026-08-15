const RUNTIME: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_signing_handoff_runtime.rs");
const SUPERVISOR_CGROUP: &str = include_str!("../external_pool_adapter_linux_supervisor/cgroup.rs");
const STARTUP: &str = include_str!("../../node_endpoint_session_startup.rs");
const WORKERS: &str = include_str!("../../server_background_workers.rs");
const MIGRATIONS: &str = include_str!("../../store_migrations.rs");
const STORE_HANDOFF: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/handoff.rs"
);
const STORE_RUN: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_compatibility_verification/run.rs"
);
const DOMAIN_HANDOFF: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification/handoff_types.rs");
const API: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification_api.rs");
const SIGNATURE_CANONICAL: &str =
    include_str!("../external_pool_adapter_runtime_compatibility_verification/canonical.rs");

#[test]
fn signing_handoff_runtime_source_freezes_exact_startup_env_matrix() {
    for required in [
        "ELON_EXTERNAL_POOL_ADAPTER_RUNTIME_COMPATIBILITY_SIGNING_HANDOFF_ENABLED",
        "ELON_EXTERNAL_POOL_ADAPTER_RUNTIME_COMPATIBILITY_CGROUP_PARENT_PATH",
        "std::env::var_os(ENABLED_ENV)",
        "std::env::var_os(CGROUP_PARENT_PATH_ENV)",
        "None => false",
        "Some(\"true\") => true",
        "Some(\"false\") => false",
        "_ => bail!",
        "if !enabled",
        "if path.is_some()",
        ".filter(|value| !value.is_empty())",
        "if !path.is_absolute()",
    ] {
        assert!(
            RUNTIME.contains(required),
            "missing exact env gate {required}"
        );
    }
    assert!(RUNTIME.contains("requires Linux x86-64"));
    assert!(RUNTIME.contains("#[cfg(not(all(target_os = \"linux\", target_arch = \"x86_64\")))]"));
    assert!(RUNTIME.contains(
        "ExternalPoolAdapterSupervisorCgroupParent::from_operator_delegated_path(&path)"
    ));
}

#[test]
fn signing_handoff_runtime_source_freezes_private_fd_custody_and_startup_order() {
    for required in [
        "static SIGNING_HANDOFF_RUNTIME: OnceLock<",
        "Option<Arc<ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRuntime>>",
        "initialize_external_pool_adapter_runtime_compatibility_signing_handoff_runtime",
        "external_pool_adapter_runtime_compatibility_signing_handoff_runtime",
        "ExternalPoolAdapterRuntimeCompatibilitySigningHandoffUnavailable",
        ".and_then(Option::as_ref)",
        ".map(Arc::clone)",
        ".ok_or(ExternalPoolAdapterRuntimeCompatibilitySigningHandoffUnavailable)",
    ] {
        assert!(
            RUNTIME.contains(required),
            "missing private custody {required}"
        );
    }
    let runtime_type = struct_block(
        RUNTIME,
        "ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRuntime",
    );
    assert_eq!(runtime_type.matches("cgroup_parent:").count(), 1);
    for forbidden in ["Serialize", "Deserialize", "Clone", "Debug"] {
        assert!(!runtime_type.contains(forbidden));
    }
    assert_eq!(DOMAIN_HANDOFF.matches("#[derive(Serialize)]").count(), 3);
    for forbidden in ["Clone", "Debug", "Deserialize", "Eq", "PartialEq"] {
        assert!(!DOMAIN_HANDOFF.contains(forbidden));
    }
    assert!(STARTUP.contains(
        "initialize_external_pool_adapter_runtime_compatibility_signing_handoff_runtime()?"
    ));
    assert_ordered(
        STARTUP,
        &[
            "initialize_external_pool_adapter_runtime_compatibility_signing_handoff_runtime()?",
            "restart_node_endpoint_sessions()",
        ],
    );
}

#[test]
fn signing_handoff_runtime_source_freezes_no_follow_trusted_cgroup_path() {
    for required in [
        "open_operator_delegated_directory",
        "Component::CurDir | Component::ParentDir | Component::Prefix(_)",
        "delegated cgroup parent cannot be the filesystem root",
        "libc::O_NOFOLLOW",
        "libc::O_DIRECTORY",
        "libc::O_CLOEXEC",
        "require_trusted_operator_path_component",
        "let effective_user_id = unsafe { libc::geteuid() }",
        "status.st_uid != 0 && status.st_uid != effective_user_id",
        "libc::S_IWGRP | libc::S_IWOTH",
        "final_component && status.st_uid != effective_user_id",
    ] {
        assert!(
            SUPERVISOR_CGROUP.contains(required),
            "missing trusted path gate {required}"
        );
    }
    assert!(SUPERVISOR_CGROUP.matches("libc::O_NOFOLLOW").count() >= 2);
    for required in [
        "libc::fstatfs",
        "CGROUP2_SUPER_MAGIC",
        "REQUIRED_CONTROLLERS: [&str; 3]",
        "read_control_file(directory.as_raw_fd(), c\"cgroup.controllers\")",
        "read_control_file(directory.as_raw_fd(), c\"cgroup.subtree_control\")",
    ] {
        assert!(
            SUPERVISOR_CGROUP.contains(required),
            "missing cgroup authority check {required}"
        );
    }
    for controller in ["\"cpu\"", "\"memory\"", "\"pids\""] {
        assert!(SUPERVISOR_CGROUP.contains(controller));
    }
}

#[test]
fn signing_handoff_runtime_source_adds_no_worker_schema_or_signature_write() {
    assert!(!MIGRATIONS.contains("migration_v269"));
    assert!(!MIGRATIONS.contains("(269,"));
    assert!(!WORKERS.contains("signing_handoff"));
    for forbidden in [
        "INSERT INTO compute_external_pool_adapter_runtime_compatibility_verification_receipts",
        "insert_verification(",
        "record_external_pool_adapter_runtime_compatibility_verification(",
        "signature_base64 TEXT",
        "signature_message_digest TEXT",
    ] {
        assert!(
            !STORE_HANDOFF.contains(forbidden),
            "handoff gained signer persistence {forbidden}"
        );
    }
}

#[test]
fn signing_handoff_runtime_source_locks_typed_http_error_mapping() {
    let compact_api = API.split_whitespace().collect::<String>();
    for exact in [
        "RuntimeCompatibilitySigningHandoffServiceError::NotFound=>StatusCode::NOT_FOUND",
        "RuntimeCompatibilitySigningHandoffServiceError::Invalid(_)=>StatusCode::BAD_REQUEST",
        "RuntimeCompatibilitySigningHandoffServiceError::Conflict(_)=>StatusCode::CONFLICT",
        "RuntimeCompatibilitySigningHandoffServiceError::Unavailable(_)=>{StatusCode::SERVICE_UNAVAILABLE}",
        "RuntimeCompatibilitySigningHandoffServiceError::Internal(_)=>{StatusCode::INTERNAL_SERVER_ERROR}",
    ] {
        assert!(
            compact_api.contains(exact),
            "missing typed HTTP mapping {exact}"
        );
    }
}

#[test]
fn signing_handoff_runtime_source_locks_decoded_signer_message_roots() {
    for required in [
        "SIGNATURE_MESSAGE_DOMAIN",
        "runner_execution_id",
        "challenge_nonce_digest",
        "sandbox_verifier_operator",
        "sandbox_verifier_product",
        "registry_release_digest",
        "registry_release_material_digest",
        "installation_content_digest",
        "source_capsule_sha256",
        "launch_image_sha256",
        "public_fixture_delivery_root",
    ] {
        assert!(
            SIGNATURE_CANONICAL.contains(required),
            "decoded signer message lost root {required}"
        );
    }
}

#[test]
fn signing_handoff_runtime_source_keeps_file_audits_outside_immediate_transactions() {
    assert_eq!(
        STORE_RUN
            .matches("support::audit_prepared_installation")
            .count(),
        2
    );
    assert_ordered(
        STORE_RUN,
        &[
            "challenge_by_id_on(&conn, challenge_id)",
            "support::audit_prepared_installation(&prepared, &challenge.receipt)",
            "transaction_with_behavior(TransactionBehavior::Immediate)",
            "challenge_by_id_on(&tx, challenge_id)",
            "current.receipt != audited_challenge",
            "execution::execute(",
            "support::audit_prepared_installation(&prepared, &challenge)",
            "transaction_with_behavior(TransactionBehavior::Immediate)",
            "challenge_by_id_on(&tx, challenge_id)",
            "current.receipt != challenge",
        ],
    );
    assert!(STORE_RUN.contains("classify_runtime_file_audit_error"));
    assert!(STORE_RUN.contains("downcast_ref::<std::io::Error>()"));
    assert!(STORE_RUN.contains("downcast_ref::<std::num::TryFromIntError>()"));

    assert_eq!(
        STORE_RUN
            .matches("transaction_with_behavior(TransactionBehavior::Immediate)")
            .count(),
        2
    );
    let preflight_transaction = STORE_RUN
        .split_once("let preflight = {")
        .unwrap()
        .1
        .split_once("let challenge = match preflight")
        .unwrap()
        .0;
    let postflight_transaction = STORE_RUN
        .split_once("support::audit_prepared_installation(&prepared, &challenge)")
        .unwrap()
        .1
        .split_once("pub(super) fn require_fresh_current_authority")
        .unwrap()
        .0;
    for transaction in [preflight_transaction, postflight_transaction] {
        for forbidden in [
            "audit_prepared_installation",
            "load_public_fixtures",
            "retained_resource",
            "read_at",
            "metadata",
        ] {
            assert!(
                !transaction.contains(forbidden),
                "SQLite Immediate block gained file work {forbidden}"
            );
        }
    }
}

fn struct_block<'a>(source: &'a str, name: &str) -> &'a str {
    source
        .split_once(&format!("struct {name} {{"))
        .unwrap()
        .1
        .split_once('}')
        .unwrap()
        .0
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
