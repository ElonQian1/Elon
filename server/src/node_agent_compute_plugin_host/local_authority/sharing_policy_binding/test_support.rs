use chrono::{DateTime, TimeZone, Utc};
use homecli_proto::{
    ComputePluginSharingAuthorizationBindingV1, ComputePluginSharingPolicySnapshotV1,
    COMPUTE_PLUGIN_SHARING_POLICY_SNAPSHOT_V1_SCHEMA,
};

use super::types::{
    PolicyBindingAuthorityState, PreparedSharingPolicyBindingRequest,
    SharingPolicyBindingRequestDigest, COMPUTE_PLUGIN_SHARING_POLICY_BINDING_REQUEST_SCHEMA,
};
use super::validation::ReadPolicyBindingState;
use crate::compute_plugin_sharing_directive::compute_plugin_sharing_policy_snapshot_digest;
use crate::node_agent_compute_plugin_host::identity::ComputePluginReleaseRef;
use crate::node_agent_compute_plugin_host::{
    lifecycle::{
        ComputePluginHealthObservation, ComputePluginInventorySnapshot, ComputePluginLocalRecord,
        ComputePluginRuntimeObservation, ComputePluginSanitizedError, ComputePluginSlotRecord,
        ACTIVATION_ENABLED, ADMISSION_ALLOWED, COMPUTE_PLUGIN_INVENTORY_SCHEMA,
        DESIRED_PRESENCE_PRESENT, RUNTIME_STOPPED, SLOT_FAILED, SLOT_INSTALLED,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

mod prepared_work;
mod store;

pub(super) use prepared_work::seed_prepared_work;
pub(super) use store::{commit_transition, connection, projected_binding, read_current_state};

const INSTALLATION_DIGEST: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const POLICY_DIGEST: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

pub(super) fn trusted_high_water() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 7, 0, 1, 0).single().unwrap()
}

pub(super) fn bound_at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 7, 0, 2, 0).single().unwrap()
}

pub(super) fn request(policy_revision: i64) -> PreparedSharingPolicyBindingRequest {
    request_with_runtime(policy_revision, true)
}

pub(super) fn disabled_request(policy_revision: i64) -> PreparedSharingPolicyBindingRequest {
    request_with_runtime(policy_revision, false)
}

fn request_with_runtime(
    policy_revision: i64,
    sharing_enabled: bool,
) -> PreparedSharingPolicyBindingRequest {
    let authorization = sharing_enabled.then(|| ComputePluginSharingAuthorizationBindingV1 {
        authorization_ref: format!("authorization_{policy_revision}"),
        revision: u64::try_from(policy_revision).unwrap(),
        digest: POLICY_DIGEST.to_string(),
    });
    let snapshot = ComputePluginSharingPolicySnapshotV1 {
        schema: COMPUTE_PLUGIN_SHARING_POLICY_SNAPSHOT_V1_SCHEMA.to_string(),
        node_id: "node_1".to_string(),
        owner_user_id: "owner_1".to_string(),
        installation_identity_digest: INSTALLATION_DIGEST.to_string(),
        policy_revision: u64::try_from(policy_revision).unwrap(),
        policy_digest: POLICY_DIGEST.to_string(),
        plugin_runtime_requested: sharing_enabled,
        authorization,
    };
    let policy_snapshot_json = serde_json::to_string(&snapshot).unwrap();
    let policy_snapshot_digest = compute_plugin_sharing_policy_snapshot_digest(&snapshot).unwrap();
    let request_digest = jcs_sha256_hex(&SharingPolicyBindingRequestDigest {
        schema: COMPUTE_PLUGIN_SHARING_POLICY_BINDING_REQUEST_SCHEMA,
        policy_snapshot: &snapshot,
        policy_snapshot_digest: &policy_snapshot_digest,
    })
    .unwrap();
    PreparedSharingPolicyBindingRequest {
        node_id: snapshot.node_id,
        owner_user_id: snapshot.owner_user_id,
        installation_id_digest: INSTALLATION_DIGEST.to_string(),
        policy_revision,
        policy_digest: POLICY_DIGEST.to_string(),
        policy_snapshot_json,
        policy_snapshot_digest,
        sharing_enabled,
        sharing_authorization_ref: snapshot
            .authorization
            .as_ref()
            .map(|value| value.authorization_ref.clone()),
        sharing_authorization_revision: snapshot
            .authorization
            .as_ref()
            .map(|value| i64::try_from(value.revision).unwrap()),
        sharing_authorization_digest: snapshot.authorization.map(|value| value.digest),
        source_preparation_id: sharing_enabled.then(|| "preparation_4".to_string()),
        source_bootstrap_instance_id: "bootstrap_1".to_string(),
        source_configuration_generation: 5,
        source_cancellation_generation: 7,
        request_digest,
    }
}

pub(super) fn authority_state() -> ReadPolicyBindingState {
    let inventory = inventory();
    let inventory_json = serde_json::to_string(&inventory).unwrap();
    let inventory_digest = jcs_sha256_hex(&inventory).unwrap();
    let trusted_time_high_water_ms = trusted_high_water().timestamp_millis();
    ReadPolicyBindingState {
        authority: PolicyBindingAuthorityState {
            installation_id_digest: INSTALLATION_DIGEST.to_string(),
            state_revision: 9,
            inventory_revision: inventory.inventory_revision,
            inventory_digest,
            inventory_json,
            desired_policy_revision: inventory.desired_policy_revision,
            sharing_enabled: inventory.sharing_enabled,
            sharing_authorization_ref: Some("authorization_3".to_string()),
            sharing_authorization_revision: Some(3),
            sharing_authorization_digest: Some("e".repeat(64)),
            authority_epoch: 11,
            process_owner_epoch: 2,
            trusted_time_high_water_ms,
            updated_at_ms: trusted_time_high_water_ms,
        },
        inventory,
    }
}

pub(super) fn decode_inventory(json: &str) -> ComputePluginInventorySnapshot {
    serde_json::from_str(json).unwrap()
}

fn release(version: &str) -> ComputePluginReleaseRef {
    ComputePluginReleaseRef {
        plugin_id: "merchant_plugin".to_string(),
        plugin_version: version.to_string(),
        target_id: "windows_x86_64".to_string(),
        manifest_digest: "1".repeat(64),
        package_digest: "2".repeat(64),
    }
}

fn inventory() -> ComputePluginInventorySnapshot {
    ComputePluginInventorySnapshot {
        schema: COMPUTE_PLUGIN_INVENTORY_SCHEMA.to_string(),
        inventory_revision: 7,
        desired_policy_revision: 3,
        sharing_enabled: true,
        plugins: vec![ComputePluginLocalRecord {
            plugin_id: "merchant_plugin".to_string(),
            last_plan_id: Some("plan_3".to_string()),
            install_generation: 2,
            activation_generation: 1,
            active_slot_ref: Some("active_a".to_string()),
            candidate_slot_ref: Some("candidate_b".to_string()),
            slots: vec![
                ComputePluginSlotRecord {
                    slot_ref: "active_a".to_string(),
                    release: release("1.0.0"),
                    phase: SLOT_INSTALLED.to_string(),
                    phase_changed_at: "2026-08-07T00:00:00.000Z".to_string(),
                    installed_at: Some("2026-08-07T00:00:00.000Z".to_string()),
                },
                ComputePluginSlotRecord {
                    slot_ref: "candidate_b".to_string(),
                    release: release("2.0.0"),
                    phase: SLOT_FAILED.to_string(),
                    phase_changed_at: "2026-08-07T00:01:00.000Z".to_string(),
                    installed_at: None,
                },
            ],
            desired_presence: DESIRED_PRESENCE_PRESENT.to_string(),
            desired_activation: ACTIVATION_ENABLED.to_string(),
            admission: ADMISSION_ALLOWED.to_string(),
            runtime: ComputePluginRuntimeObservation {
                phase: RUNTIME_STOPPED.to_string(),
                runtime_generation: 0,
                slot_ref: None,
                runner_digest: None,
                started_at: None,
                stopped_at: None,
            },
            permission_grant_digest: Some("3".repeat(64)),
            active_attempts: 2,
            health: Some(ComputePluginHealthObservation {
                status: "healthy".to_string(),
                observation_digest: "4".repeat(64),
                runtime_generation: 0,
                slot_ref: "active_a".to_string(),
                runner_digest: "5".repeat(64),
                reason_codes: Vec::new(),
                observed_at: "2026-08-07T00:00:30.000Z".to_string(),
                expires_at: "2026-08-07T00:05:30.000Z".to_string(),
            }),
            last_error: Some(ComputePluginSanitizedError {
                code: "candidate_failed".to_string(),
                safe_message: "candidate failed validation".to_string(),
                observed_at: "2026-08-07T00:01:00.000Z".to_string(),
            }),
            state_changed_at: "2026-08-07T00:01:00.000Z".to_string(),
        }],
        observed_at: "2026-08-07T00:01:00.000Z".to_string(),
    }
}
