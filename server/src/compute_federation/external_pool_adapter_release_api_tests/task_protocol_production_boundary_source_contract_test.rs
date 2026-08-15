use sha2::{Digest, Sha256};

const MIGRATION: &str =
    include_str!("../../store_migrations/compute_external_pool_adapter_task_delivery.rs");
const TABLES: &str =
    include_str!("../../store_migrations/compute_external_pool_adapter_task_delivery/tables.rs");
const ATTEMPTS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/exchange_attempts.sql"
);
const RECEIPTS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/exchange_receipts.sql"
);
const POLLS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/polls.sql"
);
const EVENTS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/events.sql"
);
const INDEXES: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/tables/indexes.sql"
);
const GUARDS: &str =
    include_str!("../../store_migrations/compute_external_pool_adapter_task_delivery/guards.rs");
const IMMUTABILITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/immutability.sql"
);
const NO_REPLACE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/no_replace.sql"
);
const POLL_CLAIMS: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/poll_claims.sql"
);
const PROJECTION: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/projection.rs"
);
const SOURCE_LINEAGE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/source_lineage.rs"
);
const ROUTE_AUTHORITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/route_authority.rs"
);
const EVENT_LINEAGE: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/guards/event_lineage.rs"
);
const INTEGRITY: &str = include_str!(
    "../../store_migrations/compute_external_pool_adapter_task_delivery/receipt_integrity.rs"
);
const V254_FENCES: &str = include_str!(
    "../../store_migrations/compute_external_pool_provider_activation_candidate/guards/fences.rs"
);
const DOMAIN: &str = include_str!("../external_pool_adapter_task_protocol_production.rs");
const WORKER: &str = include_str!("../external_pool_adapter_task_worker.rs");
const TASK_DELIVERY: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_bundle/task_delivery.rs");
const RUNTIME_BUNDLE: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_bundle.rs");
const STORE_ROOT: &str = include_str!("../../store.rs");
const RELEASE_API: &str = include_str!("../external_pool_adapter_release_api.rs");
const AUTHORITY_DOC: &str = include_str!(
    "../../../../docs/distributed-compute/external-pool-adapter-task-protocol-production-authority.md"
);
const ACCEPTANCE_DOC: &str = include_str!(
    "../../../../docs/distributed-compute/external-pool-adapter-task-protocol-production-acceptance.md"
);

#[test]
fn task_protocol_production_boundary_preserves_all_v254_fences_and_opens_none() {
    assert_eq!(
        V254_FENCES.matches("CREATE TRIGGER IF NOT EXISTS").count(),
        18
    );
    assert_eq!(
        hex::encode(Sha256::digest(V254_FENCES.as_bytes())),
        "7d2971d0987e2c2939e0b212d4aedfa15a4b7cd3205e433eb7030f1371840de6"
    );

    let v273 = migration_sources();
    for forbidden in ["DROP TRIGGER", "DROP TABLE", "ALTER TABLE", "CREATE VIEW"] {
        assert!(!v273.contains(forbidden), "V273 gained {forbidden}");
    }
    for protected_mutation in [
        "UPDATE compute_providers",
        "UPDATE compute_provider_versions",
        "UPDATE compute_route_adapters",
        "UPDATE compute_route_adapter_versions",
        "UPDATE compute_service_actor_authorizations",
        "UPDATE compute_route_credential_versions",
        "UPDATE compute_route_authorization_receipts",
        "UPDATE compute_route_authorization_capabilities",
        "UPDATE compute_route_authorization_seals",
        "UPDATE compute_capacity_pools",
        "UPDATE compute_capacity_pool_versions",
        "UPDATE compute_offers",
        "UPDATE compute_offer_versions",
        "DELETE FROM compute_",
    ] {
        assert!(
            !v273.contains(protected_mutation),
            "V273 gained protected mutation {protected_mutation}"
        );
    }
}

#[test]
fn task_protocol_production_boundary_has_no_public_or_v213_authority_constructor() {
    assert!(STORE_ROOT.contains("mod compute_external_pool_adapter_task_delivery;"));
    assert!(!STORE_ROOT.contains("use compute_external_pool_adapter_task_delivery::"));
    assert!(RUNTIME_BUNDLE.contains("mod task_delivery;"));
    assert!(!RUNTIME_BUNDLE.contains("use task_delivery::"));
    assert!(
        TASK_DELIVERY.contains("pub(super) async fn exchange_external_pool_adapter_task_delivery")
    );
    assert!(!TASK_DELIVERY.contains("pub(crate) async fn"));

    let production = format!("{DOMAIN}{WORKER}{TASK_DELIVERY}");
    for forbidden in [
        "PreparedStartSendRequest",
        "CommittedStartSendAuthority",
        "VerifiedComputeStartOutboxRemoteObservation",
        "AcceptedComputeStartOutboxClosure",
        "persist_route_authority_on",
        "ComputeRouteAuthorizationEnvelope {",
    ] {
        assert!(
            !production.contains(forbidden),
            "V273 gained authority constructor marker {forbidden}"
        );
    }
    for public_marker in [
        "task-protocol-production",
        "task-delivery",
        "authenticated-events",
        "external-pool-task",
    ] {
        assert!(
            !RELEASE_API.contains(public_marker),
            "V273 gained public route marker {public_marker}"
        );
    }
}

#[test]
fn task_protocol_production_boundary_persists_only_redacted_evidence() {
    let ddl = format!("{ATTEMPTS}{RECEIPTS}{POLLS}{EVENTS}{INDEXES}");
    for forbidden in [
        "raw_request",
        "request_body",
        "raw_response",
        "response_body",
        "event_body",
        "credential_locator",
        "credential_hint",
        "target_hostname",
        "target_sni",
        "target_spki",
        "target_address",
        "bearer",
        "claim_token TEXT",
        "process_hmac",
        "mac_key",
        "raw_nonce",
    ] {
        assert!(!ddl.contains(forbidden), "V273 persisted {forbidden}");
    }
    assert_eq!(ddl.matches("CREATE TABLE").count(), 6);
    assert_eq!(ddl.matches("CREATE VIEW").count(), 0);
    assert_eq!(ddl.matches("execution_effect\":\"none").count(), 6);
    assert_eq!(ddl.matches("execution_ready\":false").count(), 6);
}

#[test]
fn task_protocol_production_boundary_reports_only_dormant_source_review() {
    for marker in [
        "source_review_only",
        "implementation_uncompiled",
        "implementation_unrun",
        "passed=0",
        "failed=0",
        "eligible_rows=0",
        "Provider=`registering`",
        "18 fences unchanged",
    ] {
        assert!(
            AUTHORITY_DOC.contains(marker) || ACCEPTANCE_DOC.contains(marker),
            "V273 docs lost boundary marker {marker}"
        );
    }
    assert!(AUTHORITY_DOC.contains("最多包含 256 个 event"));
    assert!(ACCEPTANCE_DOC.contains("257条及以上失败关闭"));
    assert!(AUTHORITY_DOC.contains("同步 validator 本身不可被抢占"));
    assert!(ACCEPTANCE_DOC.contains("不宣称可抢占同步validator"));
}

fn migration_sources() -> String {
    format!(
        "{MIGRATION}{TABLES}{ATTEMPTS}{RECEIPTS}{POLLS}{EVENTS}{INDEXES}{GUARDS}{IMMUTABILITY}{NO_REPLACE}{POLL_CLAIMS}{PROJECTION}{SOURCE_LINEAGE}{ROUTE_AUTHORITY}{EVENT_LINEAGE}{INTEGRITY}"
    )
}
