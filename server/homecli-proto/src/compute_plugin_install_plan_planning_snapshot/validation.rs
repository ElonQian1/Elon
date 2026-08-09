use super::*;

impl HashedComputePluginInstallPlanPlanningSnapshotV2 {
    pub const CANONICALIZATION: &'static str = "rfc8785_jcs";
    pub const DIGEST_ALGORITHM: &'static str = "sha256";

    /// Validates the bounded, I-JSON-safe shape required before a future node producer may set
    /// `snapshot_ready=true`. It deliberately does not grant or acquire any local capability.
    pub fn validate_ready_shape_v2(&self) -> Result<(), &'static str> {
        if self.schema != HASHED_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_V2_SCHEMA
            || self.canonicalization != Self::CANONICALIZATION
            || self.snapshot_digest_algorithm != Self::DIGEST_ALGORITHM
            || !is_sha256(&self.snapshot_digest)
        {
            return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_HASH_METADATA_INVALID");
        }
        validate_snapshot(&self.snapshot)?;
        let encoded = serde_json::to_vec(self)
            .map_err(|_| "COMPUTE_PLUGIN_PLANNING_SNAPSHOT_SERIALIZATION_INVALID")?;
        if encoded.len() > MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_BYTES {
            return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_BYTES_EXCEEDED");
        }
        Ok(())
    }
}

fn validate_snapshot(
    snapshot: &ComputePluginInstallPlanPlanningSnapshotV2,
) -> Result<(), &'static str> {
    if snapshot.schema != COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_V2_SCHEMA
        || !bounded_identifier(&snapshot.preparation_id)
        || !bounded_identifier(&snapshot.cloud_session_id)
        || !bounded_identifier(&snapshot.source_preparation_delivery_id)
        || !is_sha256(&snapshot.source_preparation_observation_digest)
        || !bounded_identifier(&snapshot.node_id)
        || !bounded_identifier(&snapshot.owner_user_id)
        || !is_sha256(&snapshot.installation_identity_digest)
        || !safe_positive(snapshot.policy_revision)
        || !is_sha256(&snapshot.policy_digest)
        || !is_sha256(&snapshot.policy_snapshot_digest)
        || !snapshot.sharing_enabled
        || !bounded_identifier(&snapshot.authorization.authorization_ref)
        || snapshot.authorization.revision != snapshot.policy_revision
        || snapshot.authorization.digest != snapshot.policy_digest
        || !bounded_identifier(&snapshot.bootstrap_instance_id)
        || !bounded_identifier(&snapshot.policy_binding_source_preparation_id)
        || snapshot.policy_binding_source_preparation_id != snapshot.preparation_id
        || !bounded_identifier(&snapshot.target_id)
        || !bounded_identifier(&snapshot.host_api_protocol_id)
        || snapshot.host_api_revision == 0
    {
        return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_IDENTITY_INVALID");
    }
    if !safe_snapshot_numbers(snapshot) {
        return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_INTEGER_OUT_OF_RANGE");
    }
    if !snapshot_digests_are_valid(snapshot) {
        return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_DIGEST_INVALID");
    }
    if snapshot.policy_binding_authority_epoch == 0
        || snapshot.policy_binding_process_owner_epoch == 0
        || snapshot.authority_state_revision == 0
        || snapshot.authority_epoch < snapshot.policy_binding_authority_epoch
        || snapshot.process_owner_epoch < snapshot.policy_binding_process_owner_epoch
        || snapshot.trusted_time_high_water_ms == 0
        || snapshot.captured_at_ms <= snapshot.trusted_time_high_water_ms
        || snapshot.expires_at_ms <= snapshot.captured_at_ms
        || snapshot.expires_at_ms - snapshot.captured_at_ms
            > MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SNAPSHOT_LIFETIME_MS
        || snapshot.keyring_bundle_revision == 0
        || snapshot.publisher_keyring.revision == 0
        || snapshot.control_keyring.revision == 0
        || snapshot.publisher_keyring == snapshot.control_keyring
    {
        return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_AUTHORITY_INVALID");
    }
    if snapshot.installed_records.len() > MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_INSTALLED_RECORDS
    {
        return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_RECORDS_EXCEEDED");
    }
    validate_installed_records(snapshot)
}

fn safe_snapshot_numbers(snapshot: &ComputePluginInstallPlanPlanningSnapshotV2) -> bool {
    [
        snapshot.policy_revision,
        snapshot.authorization.revision,
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
}

fn snapshot_digests_are_valid(snapshot: &ComputePluginInstallPlanPlanningSnapshotV2) -> bool {
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
    snapshot: &ComputePluginInstallPlanPlanningSnapshotV2,
) -> Result<(), &'static str> {
    let mut previous_plugin_id: Option<&str> = None;
    for record in &snapshot.installed_records {
        validate_work_admission(record)?;
        if !bounded_identifier(&record.plugin_id)
            || previous_plugin_id.is_some_and(|previous| previous >= record.plugin_id.as_str())
            || !safe(record.install_generation)
            || !safe(record.runtime_generation)
            || !safe(record.active_attempts)
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
                .is_some_and(|digest| !is_sha256(digest))
            || record.active_slot_ref == record.candidate_slot_ref
                && record.active_slot_ref.is_some()
        {
            return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_RECORD_INVALID");
        }
        validate_active_record(record, &snapshot.target_id)?;
        validate_candidate_record(record, &snapshot.target_id)?;
        previous_plugin_id = Some(&record.plugin_id);
    }
    Ok(())
}

fn validate_work_admission(
    record: &ComputePluginInstallPlanPlanningInstalledRecordV2,
) -> Result<(), &'static str> {
    let Some(work_admission) = record.work_admission.as_ref() else {
        return Ok(());
    };
    let active_provenance_complete = [
        record.active_slot_ref.is_some(),
        record.active_release.is_some(),
        record.active_install_receipt_digest.is_some(),
        record.active_promotion_receipt_digest.is_some(),
        record.active_signed_manifest_envelope_digest.is_some(),
        record.permission_grant_digest.is_some(),
    ]
    .into_iter()
    .all(|present| present);
    if !active_provenance_complete
        || record.desired_presence != "present"
        || record.desired_activation != "enabled"
        || record.admission != "allowed"
        || record.runtime_phase != "stopped"
        || record.active_attempts != 0
        || record.candidate_slot_ref.is_some()
        || record.candidate.is_some()
        || !safe_positive(work_admission.generation)
        || !is_sha256(&work_admission.receipt_digest)
    {
        return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_WORK_ADMISSION_INVALID");
    }
    Ok(())
}

fn validate_active_record(
    record: &ComputePluginInstallPlanPlanningInstalledRecordV2,
    target_id: &str,
) -> Result<(), &'static str> {
    let evidence = [
        record.active_slot_ref.is_some(),
        record.active_release.is_some(),
        record.active_install_receipt_digest.is_some(),
        record.active_promotion_receipt_digest.is_some(),
        record.active_signed_manifest_envelope_digest.is_some(),
        record.permission_grant_digest.is_some(),
    ];
    let active_present = evidence.into_iter().all(|present| present);
    if (evidence.into_iter().any(|present| present) && !active_present)
        || (active_present && record.install_generation == 0)
    {
        return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_ACTIVE_PROVENANCE_INCOMPLETE");
    }
    if record
        .active_slot_ref
        .as_deref()
        .is_some_and(|slot| !bounded_opaque_ref(slot))
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
        return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_ACTIVE_PROVENANCE_INVALID");
    }
    Ok(())
}

fn validate_candidate_record(
    record: &ComputePluginInstallPlanPlanningInstalledRecordV2,
    target_id: &str,
) -> Result<(), &'static str> {
    if record.candidate.is_some() != record.candidate_slot_ref.is_some()
        || record
            .candidate_slot_ref
            .as_deref()
            .is_some_and(|slot| !bounded_opaque_ref(slot))
        || record.candidate.as_ref().is_some_and(|candidate| {
            !matches!(
                candidate.phase.as_str(),
                "downloading" | "verifying" | "staged" | "failed" | "removing"
            ) || !is_sha256(&candidate.signed_manifest_envelope_digest)
                || !valid_release(&candidate.release, &record.plugin_id, target_id)
        })
    {
        return Err("COMPUTE_PLUGIN_PLANNING_SNAPSHOT_CANDIDATE_PROVENANCE_INVALID");
    }
    Ok(())
}

fn valid_release(
    release: &ComputePluginInstallPlanPlanningReleaseV2,
    plugin_id: &str,
    target_id: &str,
) -> bool {
    release.plugin_id == plugin_id
        && release.target_id == target_id
        && bounded_identifier(&release.plugin_version)
        && is_sha256(&release.manifest_digest)
        && is_sha256(&release.package_digest)
}

fn bounded_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn bounded_opaque_ref(value: &str) -> bool {
    bounded_identifier(value) && !value.contains(['/', '\\', ':']) && value != "." && value != ".."
}

fn safe(value: u64) -> bool {
    value <= MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SAFE_INTEGER
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
