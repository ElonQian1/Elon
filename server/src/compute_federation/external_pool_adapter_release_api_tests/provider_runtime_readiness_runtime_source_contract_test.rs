const DOMAIN_TYPES: &str =
    include_str!("../external_pool_adapter_provider_runtime_readiness/types.rs");
const STORE_WRITE: &str =
    include_str!("../../store/compute_external_pool_adapter_provider_runtime_readiness/write.rs");
const STORE_CURRENT: &str =
    include_str!("../../store/compute_external_pool_adapter_provider_runtime_readiness/current.rs");
const RUNTIME: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_bundle/runtime.rs");
const CUSTODY: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_bundle/runtime/custody.rs");
const NO_WORK_ORCHESTRATION: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_bundle/no_work_probe.rs");
const NO_WORK_REPROOF: &str = include_str!(
    "../../store/compute_external_pool_adapter_runtime_bundle/no_work_probe/reproof.rs"
);
const BROKER_TLS: &str = include_str!(
    "../../store/compute_external_pool_adapter_upstream_transport_target/broker_tls.rs"
);

#[test]
fn provider_runtime_readiness_runtime_source_freezes_startup_and_domain_separation() {
    for environment in [
        "ELON_EXTERNAL_POOL_ADAPTER_PROVIDER_RUNTIME_READINESS_ENABLED",
        "ELON_EXTERNAL_POOL_ADAPTER_PROVIDER_RUNTIME_READINESS_CGROUP_PARENT_PATH",
        "ELON_EXTERNAL_POOL_ADAPTER_PROVIDER_RUNTIME_READINESS_BUNDLE_ROOT_PATH",
    ] {
        assert!(RUNTIME.contains(environment), "startup lost {environment}");
    }
    assert_eq!(RUNTIME.matches("std::env::var_os(").count(), 3);
    assert!(
        RUNTIME.contains("ExternalPoolAdapterProviderRuntimeReadinessProcessCustody::generate()?")
    );
    assert!(!RUNTIME.contains("SIGNING_HANDOFF"));
    for (name, literal) in [
        (
            "PROVIDER_RUNTIME_READINESS_CUSTODY_EPOCH_DIGEST_DOMAIN",
            "ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-RUNTIME-READINESS-CUSTODY-EPOCH-DIGEST-V1",
        ),
        (
            "PROVIDER_RUNTIME_READINESS_BUNDLE_IDENTITY_COMMITMENT_DOMAIN",
            "ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-RUNTIME-READINESS-BUNDLE-IDENTITY-HMAC-V1",
        ),
        (
            "PROVIDER_RUNTIME_READINESS_POST_CLEANUP_COMMITMENT_DOMAIN",
            "ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-RUNTIME-READINESS-POST-CLEANUP-HMAC-V1",
        ),
    ] {
        let definition = format!("const {name}");
        assert_eq!(DOMAIN_TYPES.matches(definition.as_str()).count(), 1);
        assert_eq!(DOMAIN_TYPES.matches(literal).count(), 1);
        assert_eq!(CUSTODY.matches(name).count(), 2);
        assert!(
            !CUSTODY.contains(literal),
            "custody duplicated domain {name}"
        );
    }
}

#[test]
fn provider_runtime_readiness_runtime_source_freezes_two_phase_fresh_seals() {
    for required in [
        "MAX_READINESS_SEAL_TTL_MS: i64 = 15_000",
        "MAX_LIVE_READINESS_SEALS: usize = 4_096",
        "readiness_seals: Mutex<ProviderRuntimeReadinessSealRegistry>",
        "by_receipt_id: HashMap<String, ProviderRuntimeReadinessSeal>",
        "registry.prune(now)",
    ] {
        assert!(CUSTODY.contains(required), "seal registry lost {required}");
    }
    assert!(!CUSTODY.contains("rusqlite"));

    let create = source_block(
        STORE_WRITE,
        "pub(crate) async fn create_external_pool_adapter_provider_runtime_readiness(",
        "fn exact_readiness_replay(",
    );
    assert_ordered(
        create,
        &[
            "if let Some(replay) = self.exact_readiness_replay(&input)?",
            "return Ok(replay)",
            "with_current_external_pool_adapter_no_work_probe_observation(",
            ".await",
            "if !completed",
            "let output = output",
            "if !output.replayed",
            ".commit_readiness_seal(",
            "if !committed_seal",
            "Ok(output)",
        ],
    );
    let fresh_publish = source_block(create, "if !output.replayed {", "Ok(output)");
    assert!(fresh_publish.contains(".commit_readiness_seal("));
    let outer_replay = source_block(
        create,
        "if let Some(replay)",
        "let output = Mutex::new(None)",
    );
    assert!(!outer_replay.contains("remember_readiness_seal"));
    assert!(!outer_replay.contains("commit_readiness_seal"));
    let replay = source_block(
        STORE_WRITE,
        "fn exact_readiness_replay(",
        "fn preflight_create(",
    );
    assert!(
        !replay.contains("readiness_seal"),
        "outer replay minted a seal"
    );

    let finalize = source_block(STORE_WRITE, "fn finalize_create(", "fn ensure_predecessor(");
    assert_eq!(STORE_WRITE.matches(".remember_readiness_seal(").count(), 1);
    assert_eq!(STORE_WRITE.matches(".commit_readiness_seal(").count(), 1);
    assert_ordered(
        finalize,
        &[
            "replayed: true",
            "insert_readiness(transaction, &receipt)",
            ".remember_readiness_seal(",
            "replayed: false",
        ],
    );
    let remember = source_block(
        CUSTODY,
        "pub(in crate::store) fn remember_readiness_seal(",
        "pub(in crate::store) fn commit_readiness_seal(",
    );
    assert!(remember.contains("committed: false"));
    assert!(!remember.contains("committed = true"));
    let commit = source_block(
        CUSTODY,
        "pub(in crate::store) fn commit_readiness_seal(",
        "pub(in crate::store) fn attests_readiness_seal(",
    );
    assert_ordered(
        commit,
        &[
            "return Ok(false)",
            "constant_time_equal(&seal.receipt_digest, readiness_receipt_digest)",
            "seal.committed = true",
            "Ok(true)",
        ],
    );
    let attests = source_block(
        CUSTODY,
        "pub(in crate::store) fn attests_readiness_seal(",
        "pub(in crate::store) fn post_cleanup_observation_commitment(",
    );
    assert_ordered(attests, &["seal.committed", "seal.matches("]);
}

#[test]
fn provider_runtime_readiness_runtime_source_freezes_constant_time_current_attestation() {
    let get = source_block(
        STORE_CURRENT,
        "pub(crate) fn external_pool_adapter_provider_runtime_readiness_currentness(",
        "pub(in crate::store) fn current_external_pool_adapter_provider_runtime_readiness_authority_on<",
    );
    let authority = source_block(
        STORE_CURRENT,
        "pub(in crate::store) fn current_external_pool_adapter_provider_runtime_readiness_authority_on<",
        "fn relational_currentness_on(",
    );
    for required in [".attests_custody_epoch_digest(", ".attests_readiness_seal("] {
        assert!(get.contains(required), "GET lost {required}");
        assert!(authority.contains(required), "authority lost {required}");
    }
    assert!(authority.contains(".attests_runtime_bundle_identity_commitment("));
    for (start, end) in [
        (
            "pub(in crate::store) fn attests_custody_epoch_digest(",
            "pub(in crate::store) fn attests_runtime_bundle_identity_commitment(",
        ),
        (
            "pub(in crate::store) fn attests_runtime_bundle_identity_commitment(",
            "pub(in crate::store) fn remember_readiness_seal(",
        ),
        ("fn matches(", "fn validate_readiness_seal_material("),
    ] {
        let block = source_block(CUSTODY, start, end);
        assert!(block.contains("constant_time_equal("));
        assert!(!block.contains("!="), "ordinary HMAC inequality returned");
    }
}

#[test]
fn provider_runtime_readiness_runtime_source_freezes_six_late_reopens_and_cleanup_order() {
    let broker = source_block(
        BROKER_TLS,
        "pub(in crate::store) async fn prepare_current_external_pool_adapter_broker_tls_channel(",
        "fn current_installation_binding(",
    );
    let probe = source_block(
        NO_WORK_ORCHESTRATION,
        "pub(in crate::store) async fn with_current_external_pool_adapter_no_work_probe_observation(",
        "fn require_preflight_dynamic_and_compatibility_roots(",
    );
    let reopen = "reopen_prepared().map_err(anyhow::Error::new)?";
    assert_eq!(broker.matches(reopen).count(), 2);
    assert_eq!(probe.matches(reopen).count(), 4);
    assert_ordered(
        probe,
        &[
            "prepare_current_external_pool_adapter_broker_tls_channel(",
            ".await?",
            "delivery_bundle_prepared = reopen_prepared()",
            "delivery_session_prepared = reopen_prepared()",
            ".exchange_no_work(",
            ".await?",
            "shutdown_and_reap()?",
            "reproof_bundle_prepared = reopen_prepared()",
            "reproof_session_prepared = reopen_prepared()",
            "with_reproved_external_pool_adapter_no_work_roots(",
        ],
    );
    let network = source_block(
        broker,
        "transaction.commit()?;",
        "let postflight_prepared =",
    );
    assert!(network.contains("connect_external_pool_adapter_broker_tls(broker_target).await?"));
    for forbidden in [
        "self.conn()",
        "TransactionBehavior",
        "PreparedExternalPoolAdapterInstallation",
    ] {
        assert!(!network.contains(forbidden), "network retained {forbidden}");
    }
    let final_tx = source_block(
        NO_WORK_REPROOF,
        "pub(super) fn with_reproved_external_pool_adapter_no_work_roots(",
        "fn audit_runtime_compatibility_roots(",
    );
    assert_ordered(
        final_tx,
        &[
            "TransactionBehavior::Immediate",
            "let checked_at =",
            "current_external_pool_adapter_runtime_bundle_authority_on(",
            "current_external_pool_adapter_supervisor_session_policy_companion_authority_on(",
            "select_current_probe_preparation_roots_on(",
            "current_external_pool_adapter_runtime_compatibility_verification_authority_on(",
            "post_cleanup_observation_commitment(",
            "consume(&transaction, &observation)?",
            "transaction.commit()?",
            "Ok(true)",
        ],
    );
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

fn assert_ordered(source: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let offset = source[cursor..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing ordered source marker {needle}"));
        cursor += offset + needle.len();
    }
}
