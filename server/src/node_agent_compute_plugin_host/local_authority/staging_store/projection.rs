use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};

use super::super::{
    plan_application::AuthorityPlanApplicationState,
    verification_store::VerifiedCandidateStagingSnapshot,
};
use crate::node_agent_compute_plugin_host::{
    install_plan_admission::validate_inventory,
    lifecycle::{
        is_valid_slot_transition, ComputePluginInventorySnapshot, SLOT_STAGED, SLOT_VERIFYING,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) struct CandidateStagingProjection {
    pub(super) inventory: ComputePluginInventorySnapshot,
    pub(super) inventory_json: String,
    pub(super) inventory_digest: String,
    pub(super) state_revision: i64,
    pub(super) authority_epoch: i64,
}

pub(super) fn project_candidate_staging(
    authority: &AuthorityPlanApplicationState,
    snapshot: &VerifiedCandidateStagingSnapshot,
    trusted_now: &DateTime<Utc>,
) -> Result<CandidateStagingProjection> {
    if !is_valid_slot_transition(Some(SLOT_VERIFYING), Some(SLOT_STAGED)) {
        bail!("COMPUTE_PLUGIN_STAGING_SLOT_TRANSITION_INVALID");
    }
    let current = &snapshot.current;
    let mut inventory = authority.inventory.clone();
    inventory.inventory_revision = inventory
        .inventory_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_INVENTORY_EXHAUSTED"))?;
    let observed_at = trusted_now.to_rfc3339_opts(SecondsFormat::Millis, true);
    inventory.observed_at = observed_at.clone();

    let mut records = inventory.plugins.iter_mut().filter(|record| {
        record.plugin_id == current.candidate_plugin_id
            && record.candidate_slot_ref.as_deref() == Some(current.candidate_slot_ref.as_str())
    });
    let record = records
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_RECORD_MISSING"))?;
    if records.next().is_some() {
        bail!("COMPUTE_PLUGIN_STAGING_RECORD_DUPLICATED");
    }
    let mut slots = record.slots.iter_mut().filter(|slot| {
        slot.slot_ref == current.candidate_slot_ref
            && slot.release == current.candidate_release
            && slot.phase == SLOT_VERIFYING
    });
    let slot = slots
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_SLOT_MISSING"))?;
    if slots.next().is_some() {
        bail!("COMPUTE_PLUGIN_STAGING_SLOT_DUPLICATED");
    }
    slot.phase = SLOT_STAGED.to_string();
    slot.phase_changed_at = observed_at.clone();
    record.state_changed_at = observed_at;

    validate_inventory(&inventory, trusted_now.clone())?;
    let inventory_json =
        serde_json::to_string(&inventory).context("COMPUTE_PLUGIN_STAGING_INVENTORY_JSON")?;
    let inventory_digest = jcs_sha256_hex(&inventory)?;
    Ok(CandidateStagingProjection {
        inventory,
        inventory_json,
        inventory_digest,
        state_revision: authority
            .state_revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_STATE_EXHAUSTED"))?,
        authority_epoch: authority
            .authority_epoch
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_EPOCH_EXHAUSTED"))?,
    })
}
