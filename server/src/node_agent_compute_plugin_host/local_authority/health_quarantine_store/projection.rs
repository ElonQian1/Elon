use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};

use super::super::plan_application::AuthorityPlanApplicationState;
use crate::node_agent_compute_plugin_host::{
    install_plan_admission::validate_inventory,
    lifecycle::{
        is_valid_slot_transition, ComputePluginInventorySnapshot, SLOT_FAILED, SLOT_STAGED,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) struct CandidateHealthQuarantineProjection {
    pub(super) inventory: ComputePluginInventorySnapshot,
    pub(super) inventory_json: String,
    pub(super) inventory_digest: String,
    pub(super) state_revision: i64,
    pub(super) authority_epoch: i64,
}

pub(super) fn project_candidate_health_quarantine(
    authority: &AuthorityPlanApplicationState,
    plugin_id: &str,
    slot_ref: &str,
    release: &crate::node_agent_compute_plugin_host::identity::ComputePluginReleaseRef,
    trusted_now: &DateTime<Utc>,
) -> Result<CandidateHealthQuarantineProjection> {
    if !is_valid_slot_transition(Some(SLOT_STAGED), Some(SLOT_FAILED)) {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_SLOT_TRANSITION_INVALID");
    }
    let mut inventory = authority.inventory.clone();
    inventory.inventory_revision =
        inventory.inventory_revision.checked_add(1).ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_INVENTORY_EXHAUSTED")
        })?;
    let observed_at = trusted_now.to_rfc3339_opts(SecondsFormat::Millis, true);
    inventory.observed_at = observed_at.clone();

    let mut records = inventory.plugins.iter_mut().filter(|record| {
        record.plugin_id == plugin_id && record.candidate_slot_ref.as_deref() == Some(slot_ref)
    });
    let record = records
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECORD_MISSING"))?;
    if records.next().is_some() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECORD_DUPLICATED");
    }
    let mut slots = record.slots.iter_mut().filter(|slot| {
        slot.slot_ref == slot_ref && &slot.release == release && slot.phase == SLOT_STAGED
    });
    let slot = slots
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_SLOT_MISSING"))?;
    if slots.next().is_some() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_SLOT_DUPLICATED");
    }
    slot.phase = SLOT_FAILED.to_string();
    slot.phase_changed_at = observed_at.clone();
    record.state_changed_at = observed_at;

    validate_inventory(&inventory, trusted_now.clone())?;
    let inventory_json = serde_json::to_string(&inventory)
        .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_INVENTORY_JSON")?;
    let inventory_digest = jcs_sha256_hex(&inventory)?;
    Ok(CandidateHealthQuarantineProjection {
        inventory,
        inventory_json,
        inventory_digest,
        state_revision: authority.state_revision.checked_add(1).ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_STATE_EXHAUSTED")
        })?,
        authority_epoch: authority.authority_epoch.checked_add(1).ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_EPOCH_EXHAUSTED")
        })?,
    })
}
