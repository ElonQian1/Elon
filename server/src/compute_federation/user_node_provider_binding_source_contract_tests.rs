const DOMAIN_ROOT: &str = include_str!("user_node_provider_binding.rs");
const DOMAIN_TYPES: &str = include_str!("user_node_provider_binding/types.rs");
const DOMAIN_CANONICAL: &str = include_str!("user_node_provider_binding/canonical.rs");
const DOMAIN_VALIDATED: &str = include_str!("user_node_provider_binding/validated.rs");
const MIGRATION_ROOT: &str = include_str!("../compute_user_node_provider_binding_migration.rs");
const MIGRATION_TABLES: &str =
    include_str!("../compute_user_node_provider_binding_migration/tables.rs");
const MIGRATION_PRECHECK: &str =
    include_str!("../compute_user_node_provider_binding_migration/precheck.rs");
const MIGRATION_GUARDS: &str =
    include_str!("../compute_user_node_provider_binding_migration/guards.rs");
const STORE_ROOT: &str = include_str!("../store/compute_user_node_provider_bindings.rs");
const STORE_READ: &str = include_str!("../store/compute_user_node_provider_bindings/read.rs");
const STORE_REPROOF: &str = include_str!("../store/compute_user_node_provider_bindings/reproof.rs");
const STORE_WRITE: &str = include_str!("../store/compute_user_node_provider_bindings/write.rs");
const ENDPOINT_BINDING: &str =
    include_str!("../store/node_credentials/endpoint_authority/provider_binding.rs");
const ACTIVATION_STORE: &str = include_str!("../store/compute_activation_requests.rs");
const ACTIVATION_BINDING: &str = include_str!("../store/compute_activation_user_node_binding.rs");
const BINDING_API: &str = include_str!("../compute_federation_user_node_binding_api.rs");
const STORE_MIGRATIONS: &str = include_str!("../store_migrations.rs");

#[test]
fn v279_domain_is_canonical_identity_only_evidence() {
    for marker in [
        "compute_federation.user_node_provider_binding.v1",
        "ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-ID-V1",
        "ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-REQUEST-V1",
        "ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-MATERIAL-V1",
        "ELON-COMPUTE-USER-NODE-PROVIDER-BINDING-RECEIPT-V1",
        "confirm_user_node_provider_binding",
    ] {
        assert!(DOMAIN_TYPES.contains(marker));
    }
    assert!(DOMAIN_TYPES.contains("#[serde(deny_unknown_fields)]"));
    assert!(DOMAIN_TYPES.contains("pub(super) binding_id: String"));
    assert!(DOMAIN_CANONICAL.contains("digest.update([0])"));
    assert!(DOMAIN_VALIDATED.contains("user_node_provider_binding_receipt_from_json"));
    assert!(DOMAIN_VALIDATED.contains("receipt.binding_json()? == value"));
    assert!(DOMAIN_TYPES.contains("identity_binding_recorded"));
    assert!(DOMAIN_ROOT.contains("does not prove current consent"));
    for forbidden in [
        "struct Current",
        "struct Authorized",
        "struct Ready",
        "struct Permit",
    ] {
        assert!(!DOMAIN_TYPES.contains(forbidden));
    }
}

#[test]
fn v279_table_is_one_immutable_exact_37_column_root() {
    let definition = source_block(
        MIGRATION_TABLES,
        "CREATE TABLE IF NOT EXISTS compute_user_node_provider_bindings (",
        "UNIQUE(provider_id)",
    );
    assert_ordered(
        definition,
        &[
            "binding_id TEXT",
            "binding_schema TEXT",
            "binding_digest TEXT",
            "binding_json TEXT",
            "binding_material_digest TEXT",
            "canonicalization TEXT",
            "digest_algorithm TEXT",
            "provider_id TEXT",
            "provider_genesis_policy_revision INTEGER",
            "provider_genesis_digest TEXT",
            "node_id TEXT",
            "owner_user_id TEXT",
            "installation_identity_digest TEXT",
            "endpoint_installation_binding_digest TEXT",
            "source_endpoint_credential_id TEXT",
            "source_endpoint_credential_revision INTEGER",
            "source_endpoint_credential_digest TEXT",
            "source_consent_receipt_id TEXT",
            "source_consent_policy_revision INTEGER",
            "source_consent_policy_digest TEXT",
            "source_authorization_ref TEXT",
            "source_authorization_revision INTEGER",
            "source_authorization_digest TEXT",
            "confirmation TEXT",
            "idempotency_scope TEXT",
            "idempotency_key TEXT",
            "request_digest TEXT",
            "bound_at TEXT",
            "recorded_at TEXT",
            "binding_effect TEXT",
            "provider_effect TEXT",
            "capacity_effect TEXT",
            "offer_effect TEXT",
            "readiness_effect TEXT",
            "route_effect TEXT",
            "execution_effect TEXT",
            "settlement_effect TEXT",
        ],
    );
    assert!(MIGRATION_TABLES.contains("UNIQUE(provider_id)"));
    assert!(MIGRATION_TABLES.contains("UNIQUE(node_id)"));
    assert!(MIGRATION_GUARDS.contains("user_node_provider_binding_no_update"));
    assert!(MIGRATION_GUARDS.contains("user_node_provider_binding_no_delete"));
    assert!(MIGRATION_GUARDS.contains("user_node_provider_binding_no_replace"));
    assert_eq!(MIGRATION_TABLES.matches("CREATE TABLE").count(), 1);
    assert_eq!(
        MIGRATION_PRECHECK.matches("CREATE TRIGGER").count()
            + MIGRATION_GUARDS.matches("CREATE TRIGGER").count(),
        6
    );
    assert!(!MIGRATION_TABLES.contains("CREATE VIEW"));
    assert!(!MIGRATION_TABLES.contains("revocation"));
}

#[test]
fn v279_migration_keeps_historical_sources_and_reproves_advancing_heads() {
    let guards = squash(MIGRATION_GUARDS);
    for marker in [
        "genesis.policy_revision=NEW.provider_genesis_policy_revision",
        "current_provider.policy_revision=provider.current_policy_revision",
        "endpoint.current_credential_revision=NEW.source_endpoint_credential_revision",
        "policy.plugin_policy_revision=NEW.source_consent_policy_revision",
        "endpoint.current_credential_revision>=binding.source_endpoint_credential_revision",
        "policy.plugin_policy_revision>=binding.source_consent_policy_revision",
        "provider.current_policy_revision=NEW.expected_provider_policy_revision",
        "provider.current_provider_digest=NEW.expected_provider_digest",
    ] {
        assert!(guards.contains(marker), "missing guard marker {marker}");
    }
    assert!(MIGRATION_PRECHECK.contains("elon_v279_user_node_provider_binding_is_exact"));
    assert!(MIGRATION_PRECHECK.contains("SQLITE_DETERMINISTIC"));
    assert!(MIGRATION_PRECHECK.contains("SQLITE_INNOCUOUS"));
    assert_ordered(
        MIGRATION_ROOT,
        &[
            "tables::create",
            "precheck::install",
            "guards::install",
            "transaction.commit",
        ],
    );
    assert!(STORE_MIGRATIONS.contains("(279,"));
    assert!(STORE_MIGRATIONS.contains("migration_v279"));
}

#[test]
fn v279_store_replay_and_fresh_write_have_distinct_authority_paths() {
    let replay = source_block(STORE_WRITE, "if let Some(existing)", "let provider =");
    assert!(replay.contains("binding_by_idempotency_on"));
    assert!(replay.contains("ExactReplay"));
    for forbidden in [
        "current_registered_provider_on",
        "select_current_intent",
        "insert_on(",
        "current_user_node_provider_binding_on",
    ] {
        assert!(!replay.contains(forbidden));
    }

    let fresh = source_block(STORE_WRITE, "let provider =", "fn insert_on(");
    assert_ordered(
        fresh,
        &[
            "current_registered_provider_on",
            "registered_provider_version_on",
            "current_node_endpoint_credential_source_for_user_node_provider_binding_on",
            "select_current_intent",
            "binding_by_provider_on",
            "binding_by_node_on",
            "build_user_node_provider_binding_receipt",
            "insert_on",
            "binding_by_provider_on",
            "current_user_node_provider_binding_on",
            "transaction.commit",
        ],
    );
    assert!(STORE_READ.contains("for index in 0..37"));
    assert!(STORE_READ.contains("user_node_provider_binding_receipt_from_json"));
    assert!(STORE_REPROOF.contains("consent.policy_revision >="));
    assert!(ENDPOINT_BINDING.contains("credential_revision())? < source_credential_revision"));
    assert!(
        STORE_ROOT.contains("pub(in crate::store) struct CurrentUserNodeProviderBindingAuthority")
    );
    assert!(!STORE_ROOT.contains("derive(Clone"));
}

#[test]
fn v279_user_node_activation_and_api_require_the_exact_binding() {
    let submit = source_block(
        ACTIVATION_STORE,
        "pub(crate) fn submit_compute_activation_evidence_request",
        "pub(crate) fn compute_activation_evidence_request(",
    );
    assert_ordered(
        submit,
        &[
            "request_by_idempotency_on",
            "replayed: true",
            "require_submission_binding_on",
            "INSERT INTO compute_activation_evidence_requests",
            "tx.commit",
        ],
    );
    assert!(ACTIVATION_BINDING.contains("require_user_node_provider_activation_binding_on"));
    assert!(STORE_REPROOF.contains("provider.provider.provider_kind != PROVIDER_KIND_USER_NODE"));
    assert!(STORE_REPROOF.contains("receipt.binding_id() != binding_id"));
    assert!(BINDING_API.contains("/api/me/compute/providers/:provider_id/node-binding"));
    assert!(BINDING_API.contains("get(get_binding).post(bind_provider)"));
    for forbidden in ["Ready", "route_authority", "external_pool", "settlement"] {
        assert!(!ACTIVATION_BINDING.contains(forbidden));
    }
}

fn source_block<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let (_, tail) = source.split_once(start).expect("start marker must exist");
    let (block, _) = tail.split_once(end).expect("end marker must exist");
    block
}

fn assert_ordered(source: &str, markers: &[&str]) {
    let mut cursor = 0;
    for marker in markers {
        let relative = source[cursor..]
            .find(marker)
            .unwrap_or_else(|| panic!("missing ordered marker {marker}"));
        cursor += relative + marker.len();
    }
}

fn squash(source: &str) -> String {
    source.split_whitespace().collect()
}
