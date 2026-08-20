use sha2::{Digest, Sha256};

macro_rules! store_source {
    ($path:literal) => {
        include_str!(concat!(
            "../../store/compute_external_pool_adapter_provider_active_successor/",
            $path
        ))
    };
}
macro_rules! v253_source {
    ($path:literal) => {
        include_str!(concat!(
            "../../store/compute_external_pool_adapter_credential_reattestation/",
            $path
        ))
    };
}
macro_rules! migration_source {
    ($path:literal) => {
        include_str!(concat!(
            "../../store_migrations/compute_external_pool_adapter_provider_active_successor/",
            $path
        ))
    };
}
macro_rules! authority_doc {
    ($path:literal) => {
        include_str!(concat!("../../../../docs/distributed-compute/", $path))
    };
}

const AGGREGATOR: &str = include_str!("../external_pool_adapter_release_api_tests.rs");
const INTEGRITY: &str = migration_source!("receipt_integrity.rs");
const V253_VIEW: &str = migration_source!("v253/view.sql");
const V253_CHALLENGE_ROOTS: &str = migration_source!("v253/challenge_roots.sql");
const V253_RECEIPT_ROOTS: &str = migration_source!("v253/receipt_current_roots.sql");
const STORE_ROOT: &str = include_str!("../../store.rs");
const STORE: &str =
    include_str!("../../store/compute_external_pool_adapter_provider_active_successor.rs");
const STORE_PREPARATION: &str = store_source!("preparation.rs");
const STORE_TARGET: &str = store_source!("provider_target.rs");
const STORE_TYPES: &str = store_source!("types.rs");
const STORE_READ: &str = store_source!("read.rs");
const STORE_AUDIT: &str = store_source!("audit.rs");
const STORE_APPEND: &str = store_source!("append/mod.rs");
const STORE_APPEND_GENESIS: &str = store_source!("append/genesis.rs");
const STORE_APPEND_REFRESH: &str = store_source!("append/refresh.rs");
const STORE_APPEND_CURRENT: &str = store_source!("append/current.rs");
const STORE_APPEND_READBACK: &str = store_source!("append/readback.rs");
const V253_ROOT: &str =
    include_str!("../../store/compute_external_pool_adapter_credential_reattestation.rs");
const V253_CURRENT: &str = v253_source!("current.rs");
const V253_CHALLENGE: &str = v253_source!("challenge.rs");
const V253_WRITE: &str = v253_source!("write.rs");
const V253_AUDIT: &str = v253_source!("audit.rs");
const V253_TRANSITION: &str = v253_source!("projected_transition.rs");
const V253_ACTIVE_SUBJECT: &str = v253_source!("active_subject.rs");
const V253_HTTP_TEST: &str = include_str!("credential_reattestation_http_test.rs");
const RUNTIME: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_bundle/runtime.rs");
const CUSTODY_PARENT: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_bundle/runtime/custody.rs");
const CUSTODY: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_bundle/runtime/custody/provider_active_successor.rs"
);
const CUSTODY_VALIDATION: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_bundle/runtime/custody/provider_active_successor/validation.rs"
);
const RELEASE_API: &str = include_str!("../external_pool_adapter_release_api.rs");
const V254_FENCES: &str = include_str!(
    "../../store_migrations/compute_external_pool_provider_activation_candidate/guards/fences.rs"
);
const AUTHORITY_DOC: &str =
    authority_doc!("external-pool-adapter-provider-active-successor-authority.md");
const ACCEPTANCE_DOC: &str =
    authority_doc!("external-pool-adapter-provider-active-successor-acceptance.md");
const V253_AUTHORITY_DOC: &str =
    authority_doc!("external-pool-adapter-credential-reattestation-authority.md");
const V253_ACCEPTANCE_DOC: &str =
    authority_doc!("external-pool-adapter-credential-reattestation-acceptance.md");
const CURRENT_STATUS_DOC: &str = authority_doc!("current-implementation-status.md");

#[test]
fn provider_active_successor_udfs_and_custody_keep_pending_nondeterministic() {
    for udf in [
        "elon_v274_provider_active_successor_receipt_is_exact",
        "elon_v274_provider_active_successor_revocation_is_exact",
        "elon_v274_provider_active_successor_receipt_integrity_is_exact",
        "elon_v274_provider_active_successor_pending_process_seal_is_exact",
    ] {
        assert!(INTEGRITY.contains(udf), "migration lost UDF {udf}");
    }
    assert_eq!(INTEGRITY.matches("conn.create_scalar_function").count(), 4);
    assert_eq!(INTEGRITY.matches(", deterministic, |context|").count(), 3);
    let pending_flags = source_block(
        INTEGRITY,
        "let pending =",
        "conn.create_scalar_function(PENDING_EXACT",
    );
    assert!(pending_flags.contains("FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS"));
    assert!(!pending_flags.contains("SQLITE_DETERMINISTIC"));
    assert_eq!(INTEGRITY.matches("WHEN {PENDING_EXACT}").count(), 2);
    for marker in [
        "It never opens SQLite, mints a",
        "verify_pending_external_pool_adapter_provider_active_successor_process_seal",
        ".attests_pending_provider_active_successor_process_seal(",
        ".unwrap_or(false)",
    ] {
        assert!(RUNTIME.contains(marker), "runtime verifier lost {marker}");
    }
    for marker in [
        "mod provider_active_successor;",
        "provider_active_successor_seals:",
        "ProviderActiveSuccessorSealRegistry::default()",
    ] {
        assert!(
            CUSTODY_PARENT.contains(marker),
            "custody parent lost {marker}"
        );
    }
    for marker in [
        "ExternalPoolAdapterProviderActiveSuccessorProcessSeal",
        "ExternalPoolAdapterProviderActiveSuccessorProcessSealInput",
        "receipt_integrity_digest: &str",
    ] {
        assert!(RUNTIME.contains(marker), "runtime facade lost {marker}");
    }
    for marker in [
        "ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-PROCESS-NONCE-V1",
        "ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-PROCESS-SEAL-V1",
        "LockedSensitiveBytes::random(PROCESS_NONCE_BYTES)",
        "const MAX_LIVE_PROCESS_SEALS: usize = 4_096;",
        "remember_pending_provider_active_successor_process_seal",
        "committed: false",
        "promote_provider_active_successor_process_seal",
        "connection.is_autocommit()",
        "plan_guard.ensure_same_connection(connection)?",
        "plan_guard.ensure_fully_consumed()?",
        "stored.committed = true",
        "attests_committed_provider_active_successor_process_seal",
        "discard_pending_provider_active_successor_process_seal",
    ] {
        assert!(CUSTODY.contains(marker), "custody lost {marker}");
    }
    assert!(CUSTODY_VALIDATION.contains("const MAX_PROCESS_SEAL_TTL_MS: i64 = 15_000;"));
}

#[test]
fn provider_active_successor_store_and_v253_are_witness_gated_after_v277() {
    assert!(STORE_ROOT.contains("mod compute_external_pool_adapter_provider_active_successor;"));
    assert!(!STORE_ROOT.contains("use compute_external_pool_adapter_provider_active_successor::"));
    assert!(STORE.contains("There is deliberately no public Store facade"));
    assert!(STORE.contains("mod append;"));
    assert!(STORE.contains("prepare_external_pool_adapter_provider_active_successor_target_on"));
    assert!(STORE_PREPARATION.contains(
        "pub(in crate::store) struct PrepareExternalPoolAdapterProviderActiveSuccessorTarget"
    ));
    assert!(STORE_PREPARATION.contains(
        "pub(in crate::store) fn prepare_external_pool_adapter_provider_active_successor_target_on"
    ));
    assert!(STORE_PREPARATION.contains("current_registered_provider_on"));
    assert!(STORE_PREPARATION.contains("source Provider is not exact V249 registering history"));
    for marker in [
        "current_external_pool_adapter_supervisor_session_policy_companion_authority_on",
        "input.prepared_installation",
        "current_external_pool_adapter_runtime_compatibility_verification_authority_on",
        "server_task_protocol_conformance_profile_catalog",
        "&task_protocol.profile_digest",
        "audited_structural_input(",
        "reprove_external_pool_adapter_provider_active_successor_target_on",
        "final provider active-successor target/root differs from its pre-I/O plan",
    ] {
        assert!(
            STORE_PREPARATION.contains(marker),
            "preparation lost {marker}"
        );
    }
    assert_eq!(
        STORE_PREPARATION
            .matches(".checked_at() != authority_checked_at")
            .count(),
        6
    );
    for forbidden in [
        "expected_task_protocol_profile_digest",
        "expected_lane_subject_digest",
        "expected_task_production_carrier_policy_digest",
    ] {
        assert!(!STORE_PREPARATION.contains(forbidden));
    }
    assert!(STORE_TARGET.contains("pub(super) fn derive_target("));
    assert!(STORE_TARGET
        .contains("derive_external_pool_adapter_provider_active_successor_activation_root"));
    let linux_cfg = "#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]";
    let cfg_offsets = STORE_APPEND
        .match_indices(linux_cfg)
        .map(|(offset, _)| offset)
        .collect::<Vec<_>>();
    assert_eq!(cfg_offsets.len(), 2);
    assert!(
        cfg_offsets[0] < STORE_APPEND.find("mod genesis;").unwrap()
            && STORE_APPEND.find("mod genesis;").unwrap() < cfg_offsets[1]
            && cfg_offsets[1] < STORE_APPEND.find("pub(super) use genesis::").unwrap()
    );
    assert!(STORE_APPEND.contains("pub(super) use genesis::"));
    assert!(STORE_APPEND.contains("pub(super) use material::"));
    assert!(STORE_APPEND.contains("pub(super) use readback::"));
    assert!(!STORE_APPEND.contains("pub(in crate::store) use genesis::"));
    assert!(!STORE_APPEND.contains("pub(in crate::store) use readback::"));
    assert!(STORE_APPEND_GENESIS.contains(
        "pub(in crate::store::compute_external_pool_adapter_provider_active_successor) fn prepare_external_pool_adapter_provider_active_successor_genesis_append_on"
    ));
    assert!(STORE_APPEND_GENESIS.contains(
        "pub(in crate::store::compute_external_pool_adapter_provider_active_successor) fn insert_prepared_external_pool_adapter_provider_active_successor_genesis_on"
    ));
    assert!(STORE_APPEND_READBACK.contains(
        "pub(in crate::store::compute_external_pool_adapter_provider_active_successor) fn postcommit_external_pool_adapter_provider_active_successor_readback_on"
    ));
    let prepared = source_block(
        STORE_TYPES,
        "pub(in crate::store) struct PreparedExternalPoolAdapterProviderActiveSuccessorTarget<'tx, 'conn> {",
        "impl<'tx, 'conn>",
    );
    for forbidden in [
        "#[derive",
        "impl Clone",
        "impl Debug",
        "Serialize",
        "Deserialize",
    ] {
        assert!(
            !prepared.contains(forbidden),
            "Prepared target gained {forbidden}"
        );
    }
    for marker in [
        "receipt_by_id_on(",
        "head_by_binding_and_root_on(",
        "revocation_by_target_on(",
        "bounded_decode",
    ] {
        assert!(STORE_READ.contains(marker), "private read lost {marker}");
    }
    for marker in [
        "validate_external_pool_adapter_provider_active_successor_receipt",
        "provider_active_successor_private_integrity_digest",
        "historical_external_pool_adapter_registry_provider_binding_authority_on",
    ] {
        assert!(STORE_AUDIT.contains(marker), "private audit lost {marker}");
    }
    let store_sources = format!(
        "{STORE}{STORE_PREPARATION}{STORE_TARGET}{STORE_TYPES}{STORE_READ}{STORE_AUDIT}{STORE_APPEND}{STORE_APPEND_GENESIS}{STORE_APPEND_REFRESH}{STORE_APPEND_CURRENT}{STORE_APPEND_READBACK}"
    );
    for forbidden in [
        "UPDATE compute_external_pool_adapter_provider_active_successor_",
        "DELETE FROM compute_external_pool_adapter_provider_active_successor_",
        "impl Store",
    ] {
        assert!(
            !store_sources.contains(forbidden),
            "V274 gained {forbidden}"
        );
    }
    for marker in [
        "prepare_external_pool_adapter_provider_active_successor_genesis_append_on",
        "insert_prepared_external_pool_adapter_provider_active_successor_genesis_on",
        "append_external_pool_adapter_provider_active_successor_refresh_on",
        "require_current_external_pool_adapter_provider_active_successor_on",
        "postcommit_external_pool_adapter_provider_active_successor_readback_on",
        "INSERT INTO compute_external_pool_adapter_provider_active_successor_receipts",
    ] {
        assert!(
            store_sources.contains(marker),
            "V274 append seam lost {marker}"
        );
    }
    assert!(!RELEASE_API.contains("provider-active-successor"));

    assert!(V253_ROOT.contains("mod projected_transition;"));
    assert!(V253_ROOT.contains("mod active_subject;"));
    assert!(V253_CURRENT.contains("PROVIDER_STATUS_ACTIVE"));
    assert!(V253_CHALLENGE.contains("PROVIDER_STATUS_ACTIVE"));
    assert!(V253_WRITE.contains("PROVIDER_STATUS_ACTIVE"));
    assert!(V253_AUDIT.contains("PROVIDER_STATUS_ACTIVE"));
    for marker in [
        "historical_external_pool_adapter_atomic_activation_for_binding_on",
        "historical_external_pool_adapter_atomic_activation_for_observed_provider_on",
        "current_external_pool_adapter_projected_active_credential_reattestation_authority_on",
        "route_adapter_projection_id",
    ] {
        assert!(
            V253_ACTIVE_SUBJECT.contains(marker),
            "V253 active subject lost {marker}"
        );
    }
    assert!(V253_CURRENT.contains("exact_registering"));
    assert!(V253_CHALLENGE.contains("live Provider is not the exact registering observation"));
    assert!(V253_VIEW.contains("display.revision_status='exact_registering'"));
    assert!(!V253_VIEW.contains("adjacent_active"));
    assert!(V253_CHALLENGE_ROOTS
        .contains("V253 challenge is registering-only until V277 activation witness"));
    assert!(V253_RECEIPT_ROOTS
        .contains("V253 receipt is registering-only until V277 activation witness"));
    for marker in [
        "prepare_external_pool_adapter_credential_projected_active_transition_on",
        "Non-authorizing V253 proof for V277",
        "observed.adapter_id != activation.logical_adapter_id",
        "Some(activation.route_adapter_projection_id.as_str())",
    ] {
        assert!(
            V253_TRANSITION.contains(marker),
            "transition proof lost {marker}"
        );
    }
    for marker in [
        "credential_reattestation_http_is_registering_only_before_v277",
        "StatusCode::CONFLICT, \"{historical}\"",
        "pre-V277 must not mint an active receipt",
        "assert_eq!(current_status, \"historical_only\")",
    ] {
        assert!(
            V253_HTTP_TEST.contains(marker),
            "HTTP regression lost {marker}"
        );
    }
}

#[test]
fn provider_active_successor_boundary_preserves_fences_docs_and_registration() {
    assert_eq!(
        V254_FENCES.matches("CREATE TRIGGER IF NOT EXISTS").count(),
        18
    );
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6"
    );
    for marker in [
        "source_review_only",
        "implementation_uncompiled",
        "implementation_unrun",
        "passed=0 / failed=0",
        "V277 之前两张表必须保持零行",
        "prepare_external_pool_adapter_provider_active_successor_target_on",
        "仓库仍没有public Store facade或HTTP/API producer",
        "restart/refresh真实I/O保持失败关闭",
    ] {
        assert!(
            AUTHORITY_DOC.contains(marker),
            "authority doc lost {marker}"
        );
    }
    for marker in [
        "两张 immutable表、一个非权威诊断view",
        "Provider=`registering`",
        "eligible_rows=0",
        "V254 18 deny逐字不变",
    ] {
        assert!(
            ACCEPTANCE_DOC.contains(marker),
            "acceptance doc lost {marker}"
        );
    }
    for marker in [
        "pre-V277",
        "registering-only",
        "historical/superseded",
        "durable V277 activation witness",
        "route_adapter_projection_id",
        "永远不得断言logical ID与projection ID相等",
    ] {
        assert!(
            V253_AUTHORITY_DOC.contains(marker),
            "V253 authority lost {marker}"
        );
    }
    for marker in [
        "historical_local_rust_sqlite_axum_verified_current_narrowing_source_review_only",
        "registering-only",
        "historical/superseded",
        "本批 registering-only narrowing",
        "pre-V277 must not mint an active receipt",
    ] {
        assert!(
            V253_ACCEPTANCE_DOC.contains(marker) || V253_HTTP_TEST.contains(marker),
            "V253 acceptance boundary lost {marker}"
        );
    }
    for marker in [
        "这 `8 passed` 现仅为 historical/superseded evidence",
        "pre-V277 V253 currentness严格为 `registering-only`",
    ] {
        assert!(
            CURRENT_STATUS_DOC.contains(marker),
            "status doc lost {marker}"
        );
    }
    for marker in [
        "provider_active_successor_source_contract_test",
        "provider_active_successor_store_boundary_source_contract_test",
    ] {
        assert!(AGGREGATOR.contains(marker), "aggregator lost {marker}");
    }
}

fn source_block<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .unwrap()
        .1
        .split_once(end)
        .unwrap()
        .0
}
