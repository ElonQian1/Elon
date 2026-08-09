use std::collections::HashSet;

use anyhow::{bail, Result};

use super::{
    types::{
        ComputePluginInstallPlanGenerationOutcomeV1, ComputePluginInstallPlanGenerationRequestV1,
        NodeComputePluginInstallPlanPlanningDispatchIntentV2,
    },
    GENERATION_OUTCOME_SCHEMA_V1, GENERATION_REQUEST_SCHEMA_V1, GENERATION_SIGNER_PROFILE_V2,
    MAX_SAFE_INTEGER,
};

mod work_admission;

pub(super) fn validate_planning_request(
    request: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2,
) -> Result<()> {
    if request.schema
        != homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_REQUEST_V2_SCHEMA
        || !bounded_identifier(&request.preparation_id)
        || !bounded_identifier(&request.cloud_session_id)
        || !bounded_identifier(&request.source_preparation_delivery_id)
        || !is_sha256(&request.source_preparation_observation_digest)
        || !bounded_identifier(&request.node_id)
        || !bounded_identifier(&request.owner_user_id)
        || !is_sha256(&request.installation_identity_digest)
        || !safe_positive(request.policy_revision)
        || !is_sha256(&request.policy_digest)
        || !is_sha256(&request.policy_snapshot_digest)
        || !bounded_identifier(&request.authorization.authorization_ref)
        || request.authorization.revision != request.policy_revision
        || request.authorization.digest != request.policy_digest
    {
        bail!("算力插件 Planning Snapshot V2 请求绑定无效");
    }
    Ok(())
}

pub(super) fn validate_source_preparation_observation(
    request: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2,
    json: &str,
) -> Result<homecli_proto::ComputePluginInstallPlanPreparationObservedV1> {
    let observed: homecli_proto::ComputePluginInstallPlanPreparationObservedV1 =
        serde_json::from_str(json)?;
    if observed.schema != homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PREPARATION_OBSERVED_V1_SCHEMA
        || observed.preparation_id != request.preparation_id
        || observed.node_id != request.node_id
        || observed.owner_user_id != request.owner_user_id
        || observed.installation_identity_digest.as_deref()
            != Some(request.installation_identity_digest.as_str())
        || !observed.accepted
        || observed.observed_policy_revision != Some(request.policy_revision)
        || observed.observed_policy_digest.as_deref() != Some(request.policy_digest.as_str())
        || observed.observed_policy_snapshot_digest.as_deref()
            != Some(request.policy_snapshot_digest.as_str())
        || observed.observed_authorization.as_ref() != Some(&request.authorization)
        || !bounded_identifier(&observed.bootstrap_instance_id)
        || observed.context_ready
        || observed.context.is_some()
        || observed.phase != "blocked"
        || !safe(observed.configuration_generation)
        || !safe(observed.cancellation_generation)
        || observed.blocked_reasons.is_empty()
        || observed.blocked_reasons.len() > 64
        || observed
            .blocked_reasons
            .iter()
            .any(|reason| !bounded_text(reason))
        || observed.error_code.is_some()
        || observed.compute_plugin_root_lock_acquired
        || observed.trusted_time_authority_configured
        || observed.rollback_anchor_witness_configured
        || observed.root_pinned
        || observed.authority_opened
        || observed.process_fence_acquired
        || observed.new_work_admission_enabled
        || observed.downloads_allowed
        || observed.side_effects_started
    {
        bail!("算力插件 Planning Snapshot V2 的 v209 observation 不精确");
    }
    Ok(observed)
}

pub(super) fn validate_source_sharing_observation(
    request: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2,
    json: &str,
) -> Result<()> {
    let observed: homecli_proto::ComputePluginSharingPolicyObservedV1 = serde_json::from_str(json)?;
    if observed.schema != homecli_proto::COMPUTE_PLUGIN_SHARING_POLICY_OBSERVED_V1_SCHEMA
        || observed.node_id != request.node_id
        || observed.owner_user_id != request.owner_user_id
        || observed.installation_identity_digest.as_deref()
            != Some(request.installation_identity_digest.as_str())
        || !observed.accepted
        || observed.observed_policy_revision != Some(request.policy_revision)
        || observed.observed_policy_digest.as_deref() != Some(request.policy_digest.as_str())
        || observed.observed_snapshot_digest.as_deref()
            != Some(request.policy_snapshot_digest.as_str())
        || observed.phase != "blocked"
        || !safe(observed.configuration_generation)
        || !safe(observed.cancellation_generation)
        || observed.blocked_reasons.is_empty()
        || observed.blocked_reasons.len() > 64
        || observed
            .blocked_reasons
            .iter()
            .any(|reason| !bounded_text(reason))
        || observed.error_code.is_some()
        || observed.side_effects_started
    {
        bail!("算力插件 Planning Snapshot V2 的 sharing observation 不精确");
    }
    Ok(())
}

pub(super) fn validate_planning_observation(
    intent: &NodeComputePluginInstallPlanPlanningDispatchIntentV2,
    observed: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotObservedV2,
) -> Result<()> {
    let request = &intent.request;
    if observed.schema
        != homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_OBSERVED_V2_SCHEMA
        || observed.preparation_id != request.preparation_id
        || observed.cloud_session_id != intent.cloud_session_id
        || observed.source_preparation_delivery_id != request.source_preparation_delivery_id
        || observed.source_preparation_observation_digest
            != request.source_preparation_observation_digest
        || observed.bootstrap_instance_id != intent.source_bootstrap_instance_id
        || observed.configuration_generation != intent.source_configuration_generation
        || observed.cancellation_generation != intent.source_cancellation_generation
        || observed.node_id != request.node_id
        || observed.owner_user_id != request.owner_user_id
        || observed
            .installation_identity_digest
            .as_deref()
            .is_some_and(|value| value != request.installation_identity_digest)
        || !bounded_identifier(&observed.bootstrap_instance_id)
        || !bounded_text(&observed.phase)
        || !safe(observed.configuration_generation)
        || !safe(observed.cancellation_generation)
        || observed.snapshot_ready != observed.snapshot.is_some()
        || observed.plan_apply_allowed
        || observed.new_work_admission_enabled
        || observed.downloads_allowed
        || observed.sidecar_launch_allowed
        || observed.side_effects_started
        || observed.local_confirmation_available
        || observed.blocked_reasons.len() > 64
        || observed
            .blocked_reasons
            .iter()
            .any(|reason| !bounded_text(reason))
        || observed
            .error_code
            .as_deref()
            .is_some_and(|code| !bounded_text(code))
    {
        bail!("算力插件 Planning Snapshot V2 observation 绑定无效");
    }
    if observed.accepted {
        if observed.installation_identity_digest.as_deref()
            != Some(request.installation_identity_digest.as_str())
            || observed.observed_policy_revision != Some(request.policy_revision)
            || observed.observed_policy_digest.as_deref() != Some(request.policy_digest.as_str())
            || observed.observed_policy_snapshot_digest.as_deref()
                != Some(request.policy_snapshot_digest.as_str())
            || observed.observed_authorization.as_ref() != Some(&request.authorization)
            || observed.error_code.is_some()
        {
            bail!("算力插件 Planning Snapshot V2 accepted observation 不精确");
        }
    } else if observed.error_code.is_none() || observed.snapshot_ready {
        bail!("算力插件 Planning Snapshot V2 拒绝 observation 缺少稳定终态");
    }
    if observed.snapshot_ready {
        if !observed.accepted
            || observed.phase != "planning_snapshot_ready"
            || !observed.blocked_reasons.is_empty()
            || !observed.compute_plugin_root_lock_acquired
            || !observed.trusted_time_authority_configured
            || !observed.rollback_anchor_witness_configured
            || !observed.root_pinned
            || !observed.authority_opened
            || !observed.process_fence_acquired
        {
            bail!("算力插件 Planning Snapshot V2 ready observation 缺少只读权威门卫");
        }
        validate_hashed_snapshot(request, observed, observed.snapshot.as_ref().unwrap())?;
    } else if observed.phase != "blocked"
        || observed.blocked_reasons.is_empty()
        || observed.compute_plugin_root_lock_acquired
        || observed.trusted_time_authority_configured
        || observed.rollback_anchor_witness_configured
        || observed.root_pinned
        || observed.authority_opened
        || observed.process_fence_acquired
    {
        bail!("算力插件 Planning Snapshot V2 未就绪 observation 缺少 blocker 或越过只读门卫");
    }
    Ok(())
}

pub(super) fn validate_hashed_snapshot(
    request: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2,
    observed: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotObservedV2,
    hashed: &homecli_proto::HashedComputePluginInstallPlanPlanningSnapshotV2,
) -> Result<()> {
    let snapshot = &hashed.snapshot;
    if snapshot.schema != homecli_proto::COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_V2_SCHEMA
        || snapshot.preparation_id != request.preparation_id
        || snapshot.cloud_session_id != request.cloud_session_id
        || snapshot.source_preparation_delivery_id != request.source_preparation_delivery_id
        || snapshot.source_preparation_observation_digest
            != request.source_preparation_observation_digest
        || snapshot.node_id != request.node_id
        || snapshot.owner_user_id != request.owner_user_id
        || snapshot.installation_identity_digest != request.installation_identity_digest
        || snapshot.policy_revision != request.policy_revision
        || snapshot.policy_digest != request.policy_digest
        || snapshot.policy_snapshot_digest != request.policy_snapshot_digest
        || !snapshot.sharing_enabled
        || snapshot.authorization != request.authorization
        || snapshot.bootstrap_instance_id != observed.bootstrap_instance_id
        || snapshot.configuration_generation != observed.configuration_generation
        || snapshot.cancellation_generation != observed.cancellation_generation
        || snapshot.policy_binding_source_preparation_id != request.preparation_id
        || !safe_snapshot_scalars(snapshot)
        || !snapshot_digests_are_valid(snapshot)
        || !bounded_identifier(&snapshot.target_id)
        || !bounded_identifier(&snapshot.host_api_protocol_id)
        || snapshot.host_api_revision == 0
        || snapshot.installed_records.len()
            > homecli_proto::MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_INSTALLED_RECORDS
    {
        bail!("算力插件 Planning Snapshot V2 快照事实无效");
    }
    if snapshot.publisher_keyring == snapshot.control_keyring
        || snapshot.captured_at_ms <= snapshot.trusted_time_high_water_ms
        || snapshot.expires_at_ms <= snapshot.captured_at_ms
        || snapshot.expires_at_ms - snapshot.captured_at_ms
            > homecli_proto::MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_LIFETIME_MS
    {
        bail!("算力插件 Planning Snapshot V2 时间或 keyring 边界无效");
    }
    validate_installed_records(snapshot)?;
    Ok(())
}

fn safe_snapshot_scalars(
    snapshot: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotV2,
) -> bool {
    [
        snapshot.configuration_generation,
        snapshot.cancellation_generation,
        snapshot.policy_binding_authority_epoch,
        snapshot.policy_binding_process_owner_epoch,
        snapshot.authority_state_revision,
        snapshot.authority_epoch,
        snapshot.process_owner_epoch,
        snapshot.trusted_time_high_water_ms,
        snapshot.captured_at_ms,
        snapshot.expires_at_ms,
        snapshot.inventory_revision,
        snapshot.manifest_catalog_revision,
        snapshot.keyring_bundle_revision,
        snapshot.publisher_keyring.revision,
        snapshot.control_keyring.revision,
        u64::from(snapshot.host_api_revision),
    ]
    .into_iter()
    .all(safe)
        && snapshot.policy_binding_authority_epoch > 0
        && snapshot.policy_binding_process_owner_epoch > 0
        && snapshot.authority_state_revision > 0
        && snapshot.authority_epoch >= snapshot.policy_binding_authority_epoch
        && snapshot.process_owner_epoch >= snapshot.policy_binding_process_owner_epoch
        && snapshot.trusted_time_high_water_ms > 0
        && snapshot.keyring_bundle_revision > 0
        && snapshot.publisher_keyring.revision > 0
        && snapshot.control_keyring.revision > 0
}

fn snapshot_digests_are_valid(
    snapshot: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotV2,
) -> bool {
    [
        snapshot.policy_binding_receipt_digest.as_str(),
        snapshot
            .policy_capability_revocation_receipt_digest
            .as_str(),
        snapshot.clock_epoch_digest.as_str(),
        snapshot.rollback_anchor_witness_digest.as_str(),
        snapshot.inventory_digest.as_str(),
        snapshot.node_profile_digest.as_str(),
        snapshot.manifest_catalog_digest.as_str(),
        snapshot.publisher_keyring.digest.as_str(),
        snapshot.control_keyring.digest.as_str(),
    ]
    .into_iter()
    .all(is_sha256)
}

fn validate_installed_records(
    snapshot: &homecli_proto::ComputePluginInstallPlanPlanningSnapshotV2,
) -> Result<()> {
    let mut plugin_ids = HashSet::new();
    let mut previous: Option<&str> = None;
    for record in &snapshot.installed_records {
        work_admission::validate(record)?;
        if !bounded_identifier(&record.plugin_id)
            || previous.is_some_and(|value| value >= record.plugin_id.as_str())
            || !plugin_ids.insert(record.plugin_id.as_str())
            || !safe(record.install_generation)
            || !safe(record.runtime_generation)
            || !safe(record.active_attempts)
            || record.active_slot_ref == record.candidate_slot_ref
                && record.active_slot_ref.is_some()
            || !matches!(record.desired_presence.as_str(), "present" | "absent")
            || !matches!(record.desired_activation.as_str(), "enabled" | "disabled")
            || !matches!(
                record.admission.as_str(),
                "allowed" | "quarantined" | "revoked"
            )
            || !matches!(
                record.runtime_phase.as_str(),
                "stopped" | "starting" | "ready" | "draining" | "crashed"
            )
            || record
                .permission_grant_digest
                .as_deref()
                .is_some_and(|value| !is_sha256(value))
        {
            bail!("算力插件 Planning Snapshot V2 installed record 无效");
        }
        validate_active_record(record, &snapshot.target_id)?;
        validate_candidate_record(record, &snapshot.target_id)?;
        previous = Some(&record.plugin_id);
    }
    Ok(())
}

fn validate_active_record(
    record: &homecli_proto::ComputePluginInstallPlanPlanningInstalledRecordV2,
    target_id: &str,
) -> Result<()> {
    let active_presence = [
        record.active_release.is_some(),
        record.active_slot_ref.is_some(),
        record.active_install_receipt_digest.is_some(),
        record.active_promotion_receipt_digest.is_some(),
        record.active_signed_manifest_envelope_digest.is_some(),
        record.permission_grant_digest.is_some(),
    ];
    let active_present = active_presence.iter().all(|present| *present);
    let active_absent = active_presence.iter().all(|present| !*present);
    if (!active_present && !active_absent)
        || (active_present && record.install_generation == 0)
        || record
            .active_slot_ref
            .as_deref()
            .is_some_and(|value| !bounded_opaque_ref(value))
        || record
            .active_release
            .as_ref()
            .is_some_and(|release| !valid_release(release, &record.plugin_id, target_id))
        || [
            record.active_install_receipt_digest.as_deref(),
            record.active_promotion_receipt_digest.as_deref(),
            record.active_signed_manifest_envelope_digest.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|digest| !is_sha256(digest))
    {
        bail!("算力插件 Planning Snapshot V2 active provenance 无效");
    }
    Ok(())
}

fn validate_candidate_record(
    record: &homecli_proto::ComputePluginInstallPlanPlanningInstalledRecordV2,
    target_id: &str,
) -> Result<()> {
    if record.candidate.is_some() != record.candidate_slot_ref.is_some()
        || record
            .candidate_slot_ref
            .as_deref()
            .is_some_and(|value| !bounded_opaque_ref(value))
        || record.candidate.as_ref().is_some_and(|candidate| {
            !matches!(
                candidate.phase.as_str(),
                "downloading" | "verifying" | "staged" | "failed" | "removing"
            ) || !is_sha256(&candidate.signed_manifest_envelope_digest)
                || !valid_release(&candidate.release, &record.plugin_id, target_id)
        })
    {
        bail!("算力插件 Planning Snapshot V2 candidate provenance 无效");
    }
    Ok(())
}

fn valid_release(
    release: &homecli_proto::ComputePluginInstallPlanPlanningReleaseV2,
    plugin_id: &str,
    target_id: &str,
) -> bool {
    release.plugin_id == plugin_id
        && release.target_id == target_id
        && bounded_identifier(&release.plugin_version)
        && is_sha256(&release.manifest_digest)
        && is_sha256(&release.package_digest)
}

pub(super) fn validate_generation_request(
    request: &ComputePluginInstallPlanGenerationRequestV1,
) -> Result<()> {
    if request.schema != GENERATION_REQUEST_SCHEMA_V1
        || !bounded_identifier(&request.generation_request_id)
        || !bounded_identifier(&request.snapshot_id)
        || !is_sha256(&request.snapshot_digest)
        || !bounded_identifier(&request.node_id)
        || !bounded_identifier(&request.owner_user_id)
        || !is_sha256(&request.installation_identity_digest)
        || !safe_positive(request.policy_revision)
        || !is_sha256(&request.policy_digest)
        || !bounded_identifier(&request.authorization_ref)
        || request.authorization_revision != request.policy_revision
        || request.authorization_digest != request.policy_digest
        || !safe_positive(request.requested_control_keyring_revision)
        || !is_sha256(&request.requested_control_keyring_digest)
        || request.signer_profile != GENERATION_SIGNER_PROFILE_V2
        || !safe_positive(request.requested_at_ms)
    {
        bail!("算力插件 InstallPlan generation request 无效");
    }
    Ok(())
}

pub(super) fn validate_generation_outcome(
    outcome: &ComputePluginInstallPlanGenerationOutcomeV1,
) -> Result<()> {
    if outcome.schema != GENERATION_OUTCOME_SCHEMA_V1
        || !bounded_identifier(&outcome.outcome_id)
        || !bounded_identifier(&outcome.generation_request_id)
        || !is_sha256(&outcome.generation_request_digest)
        || !matches!(
            outcome.outcome_kind.as_str(),
            "signer_unavailable" | "rejected"
        )
        || !stable_code(&outcome.detail_code)
        || (outcome.outcome_kind == "rejected" && outcome.retryable)
    {
        bail!("算力插件 InstallPlan generation outcome 无效");
    }
    Ok(())
}

fn bounded_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn bounded_text(value: &str) -> bool {
    bounded_identifier(value)
}

fn bounded_opaque_ref(value: &str) -> bool {
    bounded_identifier(value) && !value.contains(['/', '\\', ':']) && value != "." && value != ".."
}

fn stable_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn safe(value: u64) -> bool {
    value <= MAX_SAFE_INTEGER
}

fn safe_positive(value: u64) -> bool {
    value > 0 && safe(value)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
