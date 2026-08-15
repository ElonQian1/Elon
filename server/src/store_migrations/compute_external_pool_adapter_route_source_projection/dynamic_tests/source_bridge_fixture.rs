use rusqlite::{named_params, Connection, Result};
use uuid::Uuid;

use crate::store::Store;

pub(super) const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(super) const OTHER_DIGEST: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub(super) const LOGICAL_ADAPTER_ID: &str = "logical-adapter-1";
pub(super) const PROJECTION_ADAPTER_ID: &str = "projection-adapter-1";

const CAPABILITIES: &str = r#"[{"capability_id":"authenticated_ack"},{"capability_id":"authenticated_events"},{"capability_id":"cancel_no_start"},{"capability_id":"idempotent_commit"},{"capability_id":"prepare"},{"capability_id":"reconcile"}]"#;
const AUTHENTICATED_AT: &str = "2026-01-01T00:00:00.000000000Z";
const VALID_UNTIL: &str = "2030-01-01T00:00:00.000000000Z";

#[derive(Clone)]
pub(super) struct RouteAttempt {
    pub route_id: String,
    pub adapter_id: String,
    pub provider_id: String,
    pub owner_id: String,
    pub source_digest: String,
    pub route_binding_digest: String,
    pub release_version: String,
    pub implementation_digest: String,
    pub config_digest: String,
    pub service_actor_id: String,
}

impl RouteAttempt {
    pub(super) fn exact(route_id: &str) -> Self {
        Self {
            route_id: route_id.to_owned(),
            adapter_id: PROJECTION_ADAPTER_ID.to_owned(),
            provider_id: "provider-1".to_owned(),
            owner_id: "owner-1".to_owned(),
            source_digest: DIGEST.to_owned(),
            route_binding_digest: DIGEST.to_owned(),
            release_version: "1.0.0".to_owned(),
            implementation_digest: DIGEST.to_owned(),
            config_digest: DIGEST.to_owned(),
            service_actor_id: "service-actor-1".to_owned(),
        }
    }
}

pub(super) fn source_bridge_fixture() -> Connection {
    let trigger_sql = current_v271_source_trigger_sql();
    let connection = Connection::open_in_memory().expect("source bridge fixture should open");
    connection
        .execute_batch(SOURCE_BRIDGE_SCHEMA)
        .expect("source bridge fixture schema should install");
    connection
        .execute_batch(&trigger_sql)
        .expect("exact V271 trigger should install in the derived fixture");
    seed_exact_roots(&connection);
    connection
}

pub(super) fn insert_route(connection: &Connection, attempt: &RouteAttempt) -> Result<usize> {
    connection.execute(
        "INSERT INTO compute_route_authorization_receipts(
           route_authorization_id,credential_id,credential_revision,credential_digest,
           provider_id,provider_kind,provider_owner_account_id,route_kind,
           route_binding_digest,adapter_binding_digest,endpoint_id,endpoint_transport,
           adapter_id,adapter_revision,adapter_registry_digest,adapter_release_version,
           implementation_digest,adapter_config_revision,adapter_config_digest,
           credential_expires_at,credential_cleanup_expires_at,verification_kind,
           verifier_id,verifier_revision,verifier_digest,verification_receipt_id,
           verification_receipt_digest,verified_by_service_actor_id,
           actor_authorization_id,actor_authorization_digest,authenticated_at,
           authorized_at,recorded_at,source_kind,source_id,source_digest,approved_by_user_id
         ) VALUES(
           :route_id,'credential-1',1,:digest,:provider_id,'external_pool',:owner_id,
           'server_adapter',:binding_digest,:binding_digest,NULL,NULL,:adapter_id,1,
           :digest,:release_version,:implementation_digest,1,:config_digest,
           :valid_until,:valid_until,'signed_receipt','verifier-1',1,:digest,
           'verification-receipt-1',:digest,:service_actor_id,
           'actor-authorization-1',:digest,:authenticated_at,:authenticated_at,
           :authenticated_at,'external_pool_onboarding','application-1',
           :source_digest,'owner-1'
         )",
        named_params! {
            ":route_id": attempt.route_id,
            ":digest": DIGEST,
            ":provider_id": attempt.provider_id,
            ":owner_id": attempt.owner_id,
            ":binding_digest": attempt.route_binding_digest,
            ":adapter_id": attempt.adapter_id,
            ":release_version": attempt.release_version,
            ":implementation_digest": attempt.implementation_digest,
            ":config_digest": attempt.config_digest,
            ":valid_until": VALID_UNTIL,
            ":service_actor_id": attempt.service_actor_id,
            ":authenticated_at": AUTHENTICATED_AT,
            ":source_digest": attempt.source_digest,
        },
    )
}

fn current_v271_source_trigger_sql() -> String {
    let root = std::env::temp_dir().join(format!(
        "elon-route-source-derived-fixture-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("derived fixture directory should exist");
    let database = root.join("state.sqlite");
    let trigger_sql = {
        let store = Store::open(&database).expect("full Store should migrate through V271");
        let connection = store.conn().expect("full V271 database should lock");
        let fence_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='trigger' AND name LIKE 'v254_external_pool_%_fence'",
                [],
                |row| row.get(0),
            )
            .expect("V254 fence inventory should read");
        assert_eq!(fence_count, 18);
        connection
            .query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type='trigger' AND name='trg_compute_route_authorization_exact_source'",
                [],
                |row| row.get(0),
            )
            .expect("current V271 source trigger should read")
    };
    std::fs::remove_dir_all(&root).expect("derived fixture directory should be removable");
    trigger_sql
}

fn seed_exact_roots(connection: &Connection) {
    connection
        .execute(
            "INSERT INTO compute_external_pool_onboarding_requests
             VALUES('request-1',:digest,'applied')",
            named_params! { ":digest": DIGEST },
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_onboarding_reviews
             VALUES('review-1','request-1','reviewer-1',:digest,'approved')",
            named_params! { ":digest": DIGEST },
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_onboarding_applications VALUES(
               'application-1',:digest,'provider-1','external_pool','owner-1','owner-1',
               'reviewer-1',:digest,'review-1','request-1',:digest,1,:digest,
               :logical_adapter,'1.0.0',1,:digest
             )",
            named_params! {
                ":digest": DIGEST,
                ":logical_adapter": LOGICAL_ADAPTER_ID,
            },
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_registry_provider_bindings VALUES(
               'binding-1',:digest,'application-1',:digest,'provider-1','owner-1',1,
               :digest,:logical_adapter,'1.0.0',1,:digest,'release-1',:digest,
               :projection_adapter,'installation-1',:digest,:digest,'adoption-1',:digest
             )",
            named_params! {
                ":digest": DIGEST,
                ":logical_adapter": LOGICAL_ADAPTER_ID,
                ":projection_adapter": PROJECTION_ADAPTER_ID,
            },
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_registry_release_current
             VALUES('release-1',:digest,'release_current')",
            named_params! { ":digest": DIGEST },
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_adapter_registry_releases VALUES(
               'release-1',:digest,:logical_adapter,'1.0.0',:digest,:digest,:digest,:capabilities
             )",
            named_params! {
                ":digest": DIGEST,
                ":logical_adapter": LOGICAL_ADAPTER_ID,
                ":capabilities": CAPABILITIES,
            },
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_external_pool_provider_activation_delegations VALUES(
               'delegation-1',:digest,'binding-1',:digest,1,'service-actor-1','owner-1',
               'provider-1','owner-1',1,:digest,'registering',:logical_adapter,'1.0.0',
               1,:digest,'platform_dispatch_service','[\"server_adapter\"]',:authenticated_at
             )",
            named_params! {
                ":digest": DIGEST,
                ":logical_adapter": LOGICAL_ADAPTER_ID,
                ":authenticated_at": AUTHENTICATED_AT,
            },
        )
        .unwrap();
    let candidate_json = format!(
        r#"{{"candidate":{{"logical_adapter_binding_digest":"{DIGEST}","logical_projection_compatibility_digest":"{OTHER_DIGEST}"}}}}"#
    );
    connection
        .execute(
            "INSERT INTO compute_external_pool_provider_activation_candidates VALUES(
               'candidate-1',:digest,'binding-1',:digest,'release-1',:digest,
               'installation-1',:digest,:digest,:projection_adapter,'provider-1','owner-1',
               1,:digest,:logical_adapter,'1.0.0',1,:digest,:digest,:digest,:digest,
               'service-actor-1',:digest,:other_digest,:candidate_json,'registering',
               'candidate_current_not_activation_ready','activation_closure_not_implemented',
               'delegation-1',:digest,1,:authenticated_at
             )",
            named_params! {
                ":digest": DIGEST,
                ":other_digest": OTHER_DIGEST,
                ":projection_adapter": PROJECTION_ADAPTER_ID,
                ":logical_adapter": LOGICAL_ADAPTER_ID,
                ":candidate_json": candidate_json,
                ":authenticated_at": AUTHENTICATED_AT,
            },
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_providers VALUES(
               'provider-1','external_pool','owner-1','registering',1,:digest
             )",
            named_params! { ":digest": DIGEST },
        )
        .unwrap();
    let provider_json = format!(
        r#"{{"provider_id":"provider-1","provider_kind":"external_pool","owner_account_id":"owner-1","status":"registering","policy_revision":1,"adapter":{{"adapter_id":"{LOGICAL_ADAPTER_ID}","adapter_version":"1.0.0","config_revision":1,"config_digest":"{DIGEST}"}}}}"#
    );
    connection
        .execute(
            "INSERT INTO compute_provider_versions VALUES('provider-1',1,:digest,:provider_json)",
            named_params! { ":digest": DIGEST, ":provider_json": provider_json },
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_route_adapter_versions VALUES(
               :projection_adapter,1,:digest,:capabilities
             )",
            named_params! {
                ":projection_adapter": PROJECTION_ADAPTER_ID,
                ":digest": DIGEST,
                ":capabilities": CAPABILITIES,
            },
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_route_credential_versions VALUES(
               'credential-1',1,:digest,'provider-1','external_pool','owner-1',
               'server_adapter',:digest,:digest,NULL,NULL,:digest,'1.0.0',:digest,1,
               :digest,:valid_until,:valid_until,'signed_receipt','verifier-1',1,:digest,
               'verification-receipt-1',:digest
             )",
            named_params! { ":digest": DIGEST, ":valid_until": VALID_UNTIL },
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO compute_service_actor_authorizations VALUES(
               'actor-authorization-1',:digest,'provider-1','owner-1','service-actor-1',
               'platform_dispatch_service',:authenticated_at,:valid_until,
               '[\"server_adapter\"]'
             )",
            named_params! {
                ":digest": DIGEST,
                ":authenticated_at": AUTHENTICATED_AT,
                ":valid_until": VALID_UNTIL,
            },
        )
        .unwrap();
}

const SOURCE_BRIDGE_SCHEMA: &str = r#"
CREATE TABLE compute_route_authorization_receipts(
  route_authorization_id TEXT, credential_id TEXT, credential_revision INTEGER,
  credential_digest TEXT, provider_id TEXT, provider_kind TEXT,
  provider_owner_account_id TEXT, route_kind TEXT, route_binding_digest TEXT,
  adapter_binding_digest TEXT, endpoint_id TEXT, endpoint_transport TEXT,
  adapter_id TEXT, adapter_revision INTEGER, adapter_registry_digest TEXT,
  adapter_release_version TEXT, implementation_digest TEXT,
  adapter_config_revision INTEGER, adapter_config_digest TEXT,
  credential_expires_at TEXT, credential_cleanup_expires_at TEXT,
  verification_kind TEXT, verifier_id TEXT, verifier_revision INTEGER,
  verifier_digest TEXT, verification_receipt_id TEXT,
  verification_receipt_digest TEXT, verified_by_service_actor_id TEXT,
  actor_authorization_id TEXT, actor_authorization_digest TEXT,
  authenticated_at TEXT, authorized_at TEXT, recorded_at TEXT,
  source_kind TEXT, source_id TEXT, source_digest TEXT, approved_by_user_id TEXT
);
CREATE TABLE compute_route_credential_versions(
  credential_id TEXT, credential_revision INTEGER, credential_digest TEXT,
  provider_id TEXT, provider_kind TEXT, provider_owner_account_id TEXT,
  route_kind TEXT, route_binding_digest TEXT, adapter_binding_digest TEXT,
  endpoint_id TEXT, endpoint_transport TEXT, adapter_registry_digest TEXT,
  adapter_release_version TEXT, implementation_digest TEXT,
  adapter_config_revision INTEGER, adapter_config_digest TEXT, expires_at TEXT,
  cleanup_expires_at TEXT, verification_kind TEXT, verifier_id TEXT,
  verifier_revision INTEGER, verifier_digest TEXT, verification_receipt_id TEXT,
  verification_receipt_digest TEXT
);
CREATE TABLE compute_route_adapter_versions(
  adapter_id TEXT, adapter_revision INTEGER, adapter_digest TEXT,
  supported_capabilities_json TEXT
);
CREATE TABLE compute_service_actor_authorizations(
  actor_authorization_id TEXT, actor_authorization_digest TEXT, provider_id TEXT,
  provider_owner_account_id TEXT, service_actor_id TEXT, service_actor_kind TEXT,
  issued_at TEXT, valid_until TEXT, allowed_route_kinds_json TEXT
);
CREATE TABLE compute_route_credential_revocations(
  credential_id TEXT, credential_revision INTEGER, revoked_at TEXT
);
CREATE TABLE compute_activation_applications(
  application_id TEXT, application_digest TEXT, provider_id TEXT,
  applied_by_user_id TEXT
);
CREATE TABLE compute_activation_recovery_applications(
  recovery_application_id TEXT, application_digest TEXT, provider_id TEXT,
  applied_by_user_id TEXT
);
CREATE TABLE compute_external_pool_onboarding_requests(
  request_id TEXT, request_digest TEXT, status TEXT
);
CREATE TABLE compute_external_pool_onboarding_reviews(
  review_id TEXT, request_id TEXT, reviewed_by_user_id TEXT,
  review_digest TEXT, decision TEXT
);
CREATE TABLE compute_external_pool_onboarding_applications(
  application_id TEXT, application_digest TEXT, provider_id TEXT,
  provider_kind TEXT, provider_owner_account_id TEXT, approved_by_user_id TEXT,
  reviewed_by_user_id TEXT, review_digest TEXT, review_id TEXT, request_id TEXT,
  request_digest TEXT, target_provider_policy_revision INTEGER,
  target_provider_digest TEXT, adapter_id TEXT, adapter_release_version TEXT,
  adapter_config_revision INTEGER, adapter_config_digest TEXT
);
CREATE TABLE compute_external_pool_adapter_registry_provider_bindings(
  provider_binding_id TEXT, provider_binding_digest TEXT, application_id TEXT,
  application_digest TEXT, provider_id TEXT, provider_owner_account_id TEXT,
  provider_policy_revision INTEGER, provider_digest TEXT, adapter_id TEXT,
  release_version TEXT, adapter_config_revision INTEGER, adapter_config_digest TEXT,
  registry_release_id TEXT, registry_release_digest TEXT,
  route_adapter_projection_id TEXT, installation_receipt_id TEXT,
  installation_receipt_digest TEXT, installation_content_digest TEXT,
  adoption_receipt_id TEXT, adoption_receipt_digest TEXT
);
CREATE TABLE compute_external_pool_adapter_registry_release_current(
  registry_release_id TEXT, registry_release_digest TEXT, current_status TEXT
);
CREATE TABLE compute_external_pool_adapter_registry_releases(
  registry_release_id TEXT, registry_release_digest TEXT, adapter_id TEXT,
  release_version TEXT, implementation_digest TEXT, capability_set_digest TEXT,
  credential_verifier_digest TEXT, supported_capabilities_json TEXT
);
CREATE TABLE compute_external_pool_provider_activation_candidates(
  candidate_id TEXT, candidate_digest TEXT, provider_binding_id TEXT,
  provider_binding_digest TEXT, registry_release_id TEXT,
  registry_release_digest TEXT, installation_receipt_id TEXT,
  installation_receipt_digest TEXT, installation_content_digest TEXT,
  route_adapter_projection_id TEXT, provider_id TEXT,
  provider_owner_account_id TEXT, provider_policy_revision INTEGER,
  provider_digest TEXT, logical_adapter_id TEXT, release_version TEXT,
  adapter_config_revision INTEGER, adapter_config_digest TEXT,
  implementation_digest TEXT, capability_set_digest TEXT,
  credential_verifier_digest TEXT, service_actor_id TEXT,
  logical_adapter_binding_digest TEXT, logical_projection_compatibility_digest TEXT,
  candidate_json TEXT, provider_status TEXT, candidate_status TEXT,
  activation_closure_status TEXT, delegation_id TEXT, delegation_digest TEXT,
  sequence INTEGER, checked_at TEXT
);
CREATE TABLE compute_external_pool_provider_activation_delegations(
  delegation_id TEXT, delegation_digest TEXT, provider_binding_id TEXT,
  provider_binding_digest TEXT, sequence INTEGER, service_actor_id TEXT,
  issued_by_owner_user_id TEXT, provider_id TEXT, provider_owner_account_id TEXT,
  provider_policy_revision INTEGER, provider_digest TEXT, provider_status TEXT,
  logical_adapter_id TEXT, release_version TEXT, adapter_config_revision INTEGER,
  adapter_config_digest TEXT, service_actor_kind TEXT,
  allowed_route_kinds_json TEXT, issued_at TEXT
);
CREATE TABLE compute_providers(
  provider_id TEXT, provider_kind TEXT, owner_account_id TEXT, status TEXT,
  current_policy_revision INTEGER, current_provider_digest TEXT
);
CREATE TABLE compute_provider_versions(
  provider_id TEXT, policy_revision INTEGER, provider_digest TEXT,
  provider_json TEXT
);
CREATE TABLE compute_external_pool_provider_activation_delegation_revocations(
  delegation_id TEXT, delegation_digest TEXT, candidate_id TEXT,
  candidate_digest TEXT
);
CREATE TABLE compute_external_pool_adapter_installation_terminal_receipts(
  installation_receipt_id TEXT, installation_receipt_digest TEXT
);
CREATE TABLE compute_external_pool_adapter_adoption_terminal_receipts(
  adoption_receipt_id TEXT, adoption_receipt_digest TEXT
);
"#;
