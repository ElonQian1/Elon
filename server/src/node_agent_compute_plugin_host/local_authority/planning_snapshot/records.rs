use anyhow::{bail, Context, Result};
use homecli_proto::{
    ComputePluginInstallPlanPlanningCandidateV2, ComputePluginInstallPlanPlanningInstalledRecordV2,
    ComputePluginInstallPlanPlanningReleaseV2,
    MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_INSTALLED_RECORDS,
    MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SAFE_INTEGER,
};
use rusqlite::Transaction;

use super::super::{
    manifest_catalog_binding::PlanningCatalogBinding,
    plan_application::AuthorityPlanApplicationState,
    promotion_store::{read_planning_active_promotion_on, read_planning_candidate_manifest_on},
    work_admission_store::read_planning_work_admission_on,
};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    lifecycle::{ComputePluginLocalRecord, ComputePluginSlotRecord},
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(super) fn project_planning_records_on(
    transaction: &Transaction<'_>,
    observation: &ComputePluginTrustedTimeObservation,
    authority: &AuthorityPlanApplicationState,
    catalog: &PlanningCatalogBinding,
) -> Result<Vec<ComputePluginInstallPlanPlanningInstalledRecordV2>> {
    if authority.inventory.plugins.len()
        > MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_INSTALLED_RECORDS
    {
        bail!("COMPUTE_PLUGIN_PLANNING_RECORD_LIMIT_EXCEEDED");
    }
    let mut records = authority
        .inventory
        .plugins
        .iter()
        .map(|record| project_record_on(transaction, observation, authority, catalog, record))
        .collect::<Result<Vec<_>>>()?;
    records.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));
    if records
        .windows(2)
        .any(|pair| pair[0].plugin_id == pair[1].plugin_id)
    {
        bail!("COMPUTE_PLUGIN_PLANNING_RECORD_DUPLICATED");
    }
    Ok(records)
}

fn project_record_on(
    transaction: &Transaction<'_>,
    observation: &ComputePluginTrustedTimeObservation,
    authority: &AuthorityPlanApplicationState,
    catalog: &PlanningCatalogBinding,
    record: &ComputePluginLocalRecord,
) -> Result<ComputePluginInstallPlanPlanningInstalledRecordV2> {
    let active_slot = exact_slot(record, record.active_slot_ref.as_deref(), "ACTIVE")?;
    let candidate_slot = exact_slot(record, record.candidate_slot_ref.as_deref(), "CANDIDATE")?;
    let active =
        read_planning_active_promotion_on(transaction, &authority.installation_id_digest, record)?;
    let candidate = read_planning_candidate_manifest_on(
        transaction,
        &authority.installation_id_digest,
        record,
    )?;
    if active_slot.is_some() != active.is_some() || candidate_slot.is_some() != candidate.is_some()
    {
        bail!("COMPUTE_PLUGIN_PLANNING_RECORD_PROVENANCE_INCOMPLETE");
    }

    let (active_release, active_install_digest, active_promotion_digest, active_manifest_digest) =
        match (active_slot, active) {
            (Some(slot), Some(projection)) => {
                validate_release(&slot.release, &record.plugin_id, catalog.target_id())?;
                match catalog
                    .signed_manifest_envelope_digest_for(&record.plugin_id, &slot.release)?
                {
                    Some(digest) if digest == projection.signed_manifest_envelope_digest() => {}
                    _ => bail!("COMPUTE_PLUGIN_PLANNING_ACTIVE_CATALOG_MISSING_OR_CONFLICT"),
                }
                (
                    Some(wire_release(&slot.release)),
                    Some(projection.install_receipt_digest().to_string()),
                    Some(projection.promotion_receipt_digest().to_string()),
                    Some(projection.signed_manifest_envelope_digest().to_string()),
                )
            }
            (None, None) => (None, None, None, None),
            _ => bail!("COMPUTE_PLUGIN_PLANNING_ACTIVE_PROVENANCE_CHANGED"),
        };
    let candidate = match (candidate_slot, candidate) {
        (Some(slot), Some(projection)) => {
            validate_release(&slot.release, &record.plugin_id, catalog.target_id())?;
            match catalog.signed_manifest_envelope_digest_for(&record.plugin_id, &slot.release)? {
                Some(digest) if digest == projection.signed_manifest_envelope_digest() => {}
                _ => bail!("COMPUTE_PLUGIN_PLANNING_CANDIDATE_CATALOG_MISSING_OR_CONFLICT"),
            }
            Some(ComputePluginInstallPlanPlanningCandidateV2 {
                release: wire_release(&slot.release),
                phase: slot.phase.clone(),
                signed_manifest_envelope_digest: projection
                    .signed_manifest_envelope_digest()
                    .to_string(),
            })
        }
        (None, None) => None,
        _ => bail!("COMPUTE_PLUGIN_PLANNING_CANDIDATE_PROVENANCE_CHANGED"),
    };
    let work_admission =
        read_planning_work_admission_on(transaction, observation, authority, record)?;
    let projected = ComputePluginInstallPlanPlanningInstalledRecordV2 {
        plugin_id: record.plugin_id.clone(),
        install_generation: to_safe_u64(record.install_generation, "INSTALL_GENERATION")?,
        active_slot_ref: record.active_slot_ref.clone(),
        active_release,
        active_install_receipt_digest: active_install_digest,
        active_promotion_receipt_digest: active_promotion_digest,
        active_signed_manifest_envelope_digest: active_manifest_digest,
        candidate_slot_ref: record.candidate_slot_ref.clone(),
        candidate,
        desired_presence: record.desired_presence.clone(),
        desired_activation: record.desired_activation.clone(),
        admission: record.admission.clone(),
        runtime_phase: record.runtime.phase.clone(),
        runtime_generation: to_safe_u64(record.runtime.runtime_generation, "RUNTIME_GENERATION")?,
        active_attempts: to_safe_u64(record.active_attempts, "ACTIVE_ATTEMPTS")?,
        permission_grant_digest: record.permission_grant_digest.clone(),
        work_admission,
    };
    validate_projected_record(&projected, catalog.target_id())?;
    Ok(projected)
}

fn exact_slot<'a>(
    record: &'a ComputePluginLocalRecord,
    slot_ref: Option<&str>,
    field: &'static str,
) -> Result<Option<&'a ComputePluginSlotRecord>> {
    let Some(slot_ref) = slot_ref else {
        return Ok(None);
    };
    let mut matches = record.slots.iter().filter(|slot| slot.slot_ref == slot_ref);
    let slot = matches
        .next()
        .with_context(|| format!("COMPUTE_PLUGIN_PLANNING_{field}_SLOT_MISSING"))?;
    if matches.next().is_some() {
        bail!("COMPUTE_PLUGIN_PLANNING_SLOT_AMBIGUOUS");
    }
    Ok(Some(slot))
}

fn validate_projected_record(
    record: &ComputePluginInstallPlanPlanningInstalledRecordV2,
    target_id: &str,
) -> Result<()> {
    let active_complete = [
        record.active_slot_ref.is_some(),
        record.active_release.is_some(),
        record.active_install_receipt_digest.is_some(),
        record.active_promotion_receipt_digest.is_some(),
        record.active_signed_manifest_envelope_digest.is_some(),
        record.permission_grant_digest.is_some(),
    ]
    .into_iter()
    .all(|present| present);
    let active_absent = [
        record.active_slot_ref.is_none(),
        record.active_release.is_none(),
        record.active_install_receipt_digest.is_none(),
        record.active_promotion_receipt_digest.is_none(),
        record.active_signed_manifest_envelope_digest.is_none(),
        record.permission_grant_digest.is_none(),
    ]
    .into_iter()
    .all(|absent| absent);
    if !bounded_identifier(&record.plugin_id)
        || (!active_complete && !active_absent)
        || record.candidate.is_some() != record.candidate_slot_ref.is_some()
        || record.active_slot_ref == record.candidate_slot_ref && record.active_slot_ref.is_some()
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
            .active_slot_ref
            .as_deref()
            .is_some_and(|value| !bounded_slot_ref(value))
        || record
            .candidate_slot_ref
            .as_deref()
            .is_some_and(|value| !bounded_slot_ref(value))
        || record
            .permission_grant_digest
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
        || record.active_release.as_ref().is_some_and(|release| {
            validate_wire_release(release, &record.plugin_id, target_id).is_err()
        })
        || record.candidate.as_ref().is_some_and(|candidate| {
            !matches!(
                candidate.phase.as_str(),
                "downloading" | "verifying" | "staged" | "failed" | "removing"
            ) || !is_sha256(&candidate.signed_manifest_envelope_digest)
                || validate_wire_release(&candidate.release, &record.plugin_id, target_id).is_err()
        })
        || [
            record.active_install_receipt_digest.as_deref(),
            record.active_promotion_receipt_digest.as_deref(),
            record.active_signed_manifest_envelope_digest.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|digest| !is_sha256(digest))
    {
        bail!("COMPUTE_PLUGIN_PLANNING_RECORD_INVALID");
    }
    if let Some(admission) = record.work_admission.as_ref() {
        if !active_complete
            || record.desired_presence != "present"
            || record.desired_activation != "enabled"
            || record.admission != "allowed"
            || record.runtime_phase != "stopped"
            || record.active_attempts != 0
            || record.candidate.is_some()
            || admission.generation == 0
            || admission.generation > MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SAFE_INTEGER
            || !is_sha256(&admission.receipt_digest)
        {
            bail!("COMPUTE_PLUGIN_PLANNING_WORK_ADMISSION_PROJECTION_INVALID");
        }
    }
    Ok(())
}

fn validate_release(
    release: &ComputePluginReleaseRef,
    plugin_id: &str,
    target_id: &str,
) -> Result<()> {
    validate_wire_release(&wire_release(release), plugin_id, target_id)
}

fn validate_wire_release(
    release: &ComputePluginInstallPlanPlanningReleaseV2,
    plugin_id: &str,
    target_id: &str,
) -> Result<()> {
    if release.plugin_id != plugin_id
        || release.target_id != target_id
        || !bounded_identifier(&release.plugin_version)
        || !is_sha256(&release.manifest_digest)
        || !is_sha256(&release.package_digest)
    {
        bail!("COMPUTE_PLUGIN_PLANNING_RELEASE_INVALID");
    }
    Ok(())
}

fn wire_release(release: &ComputePluginReleaseRef) -> ComputePluginInstallPlanPlanningReleaseV2 {
    ComputePluginInstallPlanPlanningReleaseV2 {
        plugin_id: release.plugin_id.clone(),
        plugin_version: release.plugin_version.clone(),
        target_id: release.target_id.clone(),
        manifest_digest: release.manifest_digest.clone(),
        package_digest: release.package_digest.clone(),
    }
}

fn to_safe_u64(value: i64, field: &'static str) -> Result<u64> {
    let value =
        u64::try_from(value).with_context(|| format!("COMPUTE_PLUGIN_PLANNING_{field}_RANGE"))?;
    if value > MAX_COMPUTE_PLUGIN_INSTALL_PLAN_PLANNING_SAFE_INTEGER {
        bail!("COMPUTE_PLUGIN_PLANNING_INTEGER_OUT_OF_RANGE");
    }
    Ok(value)
}

fn bounded_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn bounded_slot_ref(value: &str) -> bool {
    bounded_identifier(value) && !value.contains(['/', '\\', ':']) && value != "." && value != ".."
}
