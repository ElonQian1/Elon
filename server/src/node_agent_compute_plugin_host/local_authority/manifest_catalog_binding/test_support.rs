use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{params, Connection, TransactionBehavior};
use serde_json::json;

use crate::node_agent_compute_plugin_host::{
    keyring::ComputePluginKeyringBinding,
    lifecycle::{ComputePluginInventorySnapshot, COMPUTE_PLUGIN_INVENTORY_SCHEMA},
    local_authority::manifest_catalog_binding::{
        types::{
            ManifestCatalogAuthorityState, ManifestCatalogBindingRequestDigest,
            PreparedManifestCatalogBindingRequest, ProjectedManifestCatalogBinding,
            COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_REQUEST_SCHEMA,
        },
        validation::{project, read_state_at_or_before, validate_authority_after},
        write::{
            insert_receipt, read_binding_by_revision, validate_current_catalog_head,
            validate_exact_request,
        },
    },
    local_authority_schema::ensure_schema,
    manifest_catalog::{
        ComputePluginManifestCatalog, SignedComputePluginManifestCatalog,
        COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA, SIGNED_COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA,
    },
    plugin_manifest::{
        ComputePluginSignature, COMPUTE_PLUGIN_DIGEST_ALGORITHM,
        COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION, COMPUTE_PLUGIN_SIGNATURE_ALGORITHM,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

const INSTALLATION_DIGEST: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PUBLISHER_DIGEST: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const CONTROL_DIGEST: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const NODE_PROFILE_DIGEST: &str =
    "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const CONTROL_KEY_FINGERPRINT: &str =
    "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const BOUND_AT_MS: i64 = 1_786_233_720_000;

pub(super) fn projected() -> ProjectedManifestCatalogBinding {
    projected_revision(4, authority_state(), BOUND_AT_MS)
}

pub(super) fn projected_revision(
    catalog_revision: i64,
    before: ManifestCatalogAuthorityState,
    bound_at_ms: i64,
) -> ProjectedManifestCatalogBinding {
    project(request(catalog_revision), before, bound_at_ms).unwrap()
}

pub(super) fn connection(before: &ManifestCatalogAuthorityState) -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .unwrap();
    connection
        .pragma_update(None, "trusted_schema", "OFF")
        .unwrap();
    ensure_schema(&mut connection).unwrap();
    seed_keyring(&connection, before);
    seed_authority(&connection, before);
    connection
}

pub(super) fn bound_at_ms() -> i64 {
    BOUND_AT_MS
}

pub(super) fn commit_transition(
    connection: &mut Connection,
    projected: &ProjectedManifestCatalogBinding,
) {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    insert_receipt(&transaction, projected).unwrap();
    let stored = read_binding_by_revision(&transaction, projected.request.catalog_revision)
        .unwrap()
        .unwrap();
    validate_exact_request(&stored.request, &projected.request).unwrap();
    assert_eq!(stored.hashed_receipt, projected.hashed_receipt);
    let trusted_now = Utc
        .timestamp_millis_opt(projected.hashed_receipt.receipt.bound_at_ms)
        .single()
        .unwrap();
    validate_current_catalog_head(&transaction, &stored.hashed_receipt, &trusted_now).unwrap();
    validate_authority_after(&transaction, projected, &trusted_now).unwrap();
    transaction.commit().unwrap();
}

pub(super) fn read_current_state(
    connection: &mut Connection,
    trusted_now: &DateTime<Utc>,
) -> ManifestCatalogAuthorityState {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();
    let current = read_state_at_or_before(&transaction, trusted_now).unwrap();
    transaction.rollback().unwrap();
    current
}

fn request(catalog_revision: i64) -> PreparedManifestCatalogBindingRequest {
    let publisher_keyring = publisher_keyring();
    let control_keyring = control_keyring();
    let catalog = ComputePluginManifestCatalog {
        schema: COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA.to_string(),
        catalog_revision,
        target_id: "windows_x86_64".to_string(),
        host_api_protocol_id: "elon_compute_plugin_host".to_string(),
        host_api_revision: 1,
        keyring_bundle_revision: 3,
        publisher_keyring: publisher_keyring.clone(),
        control_keyring: control_keyring.clone(),
        entries: Vec::new(),
    };
    let catalog_json = serde_json::to_string(&catalog).unwrap();
    let catalog_digest = jcs_sha256_hex(&catalog).unwrap();
    let signed_catalog = SignedComputePluginManifestCatalog {
        schema: SIGNED_COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA.to_string(),
        catalog,
        canonicalization: COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION.to_string(),
        catalog_digest_algorithm: COMPUTE_PLUGIN_DIGEST_ALGORITHM.to_string(),
        catalog_digest: catalog_digest.clone(),
        signature: ComputePluginSignature {
            algorithm: COMPUTE_PLUGIN_SIGNATURE_ALGORITHM.to_string(),
            signing_key_id: "control_key_3".to_string(),
            signature_base64: "test-signature-not-verified-by-store-readback".to_string(),
        },
    };
    let signed_catalog_json = serde_json::to_string(&signed_catalog).unwrap();
    let signed_catalog_envelope_digest = jcs_sha256_hex(&signed_catalog).unwrap();
    let signed_manifests_json = "[]".to_string();
    let signed_manifest_set_digest = jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.manifest_catalog_signed_manifest_set.v1",
        "signed_manifest_envelope_digests": [],
    }))
    .unwrap();
    let mut request = PreparedManifestCatalogBindingRequest {
        request_id: format!("catalog_request_{catalog_revision}"),
        request_digest: String::new(),
        installation_id_digest: INSTALLATION_DIGEST.to_string(),
        catalog_revision,
        catalog_json,
        catalog_digest,
        signed_catalog_json,
        signed_catalog_envelope_digest,
        control_signing_key_id: "control_key_3".to_string(),
        control_signing_key_fingerprint: CONTROL_KEY_FINGERPRINT.to_string(),
        signed_manifests_json,
        signed_manifest_set_digest,
        catalog_entry_count: 0,
        node_profile_digest: NODE_PROFILE_DIGEST.to_string(),
        target_id: "windows_x86_64".to_string(),
        host_api_protocol_id: "elon_compute_plugin_host".to_string(),
        host_api_revision: 1,
        keyring_bundle_revision: 3,
        publisher_keyring,
        control_keyring,
    };
    request.request_digest = jcs_sha256_hex(&ManifestCatalogBindingRequestDigest {
        schema: COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_REQUEST_SCHEMA,
        request_id: &request.request_id,
        installation_id_digest: &request.installation_id_digest,
        catalog_revision: request.catalog_revision,
        catalog_digest: &request.catalog_digest,
        signed_catalog_envelope_digest: &request.signed_catalog_envelope_digest,
        control_signing_key_id: &request.control_signing_key_id,
        control_signing_key_fingerprint: &request.control_signing_key_fingerprint,
        signed_manifest_set_digest: &request.signed_manifest_set_digest,
        node_profile_digest: &request.node_profile_digest,
        target_id: &request.target_id,
        host_api_protocol_id: &request.host_api_protocol_id,
        host_api_revision: request.host_api_revision,
        keyring_bundle_revision: request.keyring_bundle_revision,
        publisher_keyring: &request.publisher_keyring,
        control_keyring: &request.control_keyring,
    })
    .unwrap();
    request
}

pub(super) fn authority_state() -> ManifestCatalogAuthorityState {
    let inventory = ComputePluginInventorySnapshot {
        schema: COMPUTE_PLUGIN_INVENTORY_SCHEMA.to_string(),
        inventory_revision: 2,
        desired_policy_revision: 0,
        sharing_enabled: false,
        plugins: Vec::new(),
        observed_at: "2026-08-09T00:00:00.000Z".to_string(),
    };
    let inventory_json = serde_json::to_string(&inventory).unwrap();
    ManifestCatalogAuthorityState {
        installation_id_digest: INSTALLATION_DIGEST.to_string(),
        state_revision: 8,
        inventory_revision: inventory.inventory_revision,
        inventory_digest: jcs_sha256_hex(&inventory).unwrap(),
        inventory_json,
        desired_policy_revision: inventory.desired_policy_revision,
        sharing_enabled: inventory.sharing_enabled,
        node_profile_digest: NODE_PROFILE_DIGEST.to_string(),
        manifest_catalog_revision: 3,
        target_id: "windows_x86_64".to_string(),
        host_api_protocol_id: "elon_compute_plugin_host".to_string(),
        host_api_revision: 1,
        authority_epoch: 10,
        process_owner_epoch: 2,
        trusted_time_high_water_ms: BOUND_AT_MS - 60_000,
        updated_at_ms: BOUND_AT_MS - 60_000,
        keyring_bundle_revision: 3,
        publisher_keyring: publisher_keyring(),
        control_keyring: control_keyring(),
    }
}

fn publisher_keyring() -> ComputePluginKeyringBinding {
    ComputePluginKeyringBinding {
        revision: 5,
        digest: PUBLISHER_DIGEST.to_string(),
    }
}

fn control_keyring() -> ComputePluginKeyringBinding {
    ComputePluginKeyringBinding {
        revision: 7,
        digest: CONTROL_DIGEST.to_string(),
    }
}

fn seed_keyring(connection: &Connection, before: &ManifestCatalogAuthorityState) {
    connection
        .execute(
            r#"INSERT INTO keyring_bundles (
                bundle_revision, bundle_digest, signed_envelope_digest, signed_bundle_json,
                root_signing_key_id, root_key_fingerprint,
                publisher_revision, publisher_digest,
                control_revision, control_digest,
                publisher_key_count, control_key_count,
                generated_at_ms, expires_at_ms, installed_at_ms
            ) VALUES (?1, ?2, ?3, '{}', 'root_key_1', ?4, ?5, ?6, ?7, ?8, 0, 0, ?9, ?10, ?11)"#,
            params![
                before.keyring_bundle_revision,
                "1".repeat(64),
                "2".repeat(64),
                "3".repeat(64),
                before.publisher_keyring.revision,
                &before.publisher_keyring.digest,
                before.control_keyring.revision,
                &before.control_keyring.digest,
                before.updated_at_ms - 60_000,
                before.updated_at_ms + 3_600_000,
                before.updated_at_ms,
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO keyring_seals (bundle_revision, sealed_at_ms) VALUES (?1, ?2)",
            params![before.keyring_bundle_revision, before.updated_at_ms],
        )
        .unwrap();
}

fn seed_authority(connection: &Connection, before: &ManifestCatalogAuthorityState) {
    connection
        .execute(
            r#"INSERT INTO authority_meta (
                singleton, schema_version, installation_id_digest,
                state_revision, inventory_revision, inventory_digest, inventory_json,
                desired_policy_revision, sharing_enabled,
                sharing_authorization_ref, sharing_authorization_revision,
                sharing_authorization_digest, node_profile_digest,
                manifest_catalog_revision, target_id, host_api_protocol_id,
                host_api_revision, active_bundle_revision,
                publisher_keyring_revision, publisher_keyring_digest,
                control_keyring_revision, control_keyring_digest,
                authority_epoch, process_owner_epoch,
                trusted_time_high_water_ms, clock_status, updated_at_ms
            ) VALUES (
                1, 3, ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                NULL, NULL, NULL, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18, ?19, ?20, 'trusted', ?21
            )"#,
            params![
                &before.installation_id_digest,
                before.state_revision,
                before.inventory_revision,
                &before.inventory_digest,
                &before.inventory_json,
                before.desired_policy_revision,
                i64::from(before.sharing_enabled),
                &before.node_profile_digest,
                before.manifest_catalog_revision,
                &before.target_id,
                &before.host_api_protocol_id,
                before.host_api_revision,
                before.keyring_bundle_revision,
                before.publisher_keyring.revision,
                &before.publisher_keyring.digest,
                before.control_keyring.revision,
                &before.control_keyring.digest,
                before.authority_epoch,
                before.process_owner_epoch,
                before.trusted_time_high_water_ms,
                before.updated_at_ms,
            ],
        )
        .unwrap();
}
