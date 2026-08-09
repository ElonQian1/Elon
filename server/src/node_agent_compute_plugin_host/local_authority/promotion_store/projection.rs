use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};

use super::super::plan_application::AuthorityPlanApplicationState;
use crate::node_agent_compute_plugin_host::{
    install_plan_admission::validate_inventory,
    lifecycle::{
        is_valid_slot_transition, ComputePluginInventorySnapshot, ACTIVATION_ENABLED,
        ADMISSION_ALLOWED, DESIRED_PRESENCE_PRESENT, RUNTIME_STOPPED, SLOT_INSTALLED, SLOT_STAGED,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) struct CandidatePromotionProjection {
    pub(super) inventory: ComputePluginInventorySnapshot,
    pub(super) inventory_json: String,
    pub(super) inventory_digest: String,
    pub(super) state_revision: i64,
    pub(super) authority_epoch: i64,
    pub(super) install_generation_before: i64,
    pub(super) install_generation_after: i64,
    pub(super) activation_generation_before: i64,
    pub(super) activation_generation_after: i64,
}

pub(super) fn project_candidate_promotion(
    authority: &AuthorityPlanApplicationState,
    plugin_id: &str,
    slot_ref: &str,
    release: &crate::node_agent_compute_plugin_host::identity::ComputePluginReleaseRef,
    candidate_generation: i64,
    permission_grant_digest: &str,
    owner_plan_id: &str,
    trusted_now: &DateTime<Utc>,
) -> Result<CandidatePromotionProjection> {
    if !is_valid_slot_transition(Some(SLOT_STAGED), Some(SLOT_INSTALLED)) {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_SLOT_TRANSITION_INVALID");
    }
    let mut inventory = authority.inventory.clone();
    inventory.inventory_revision = inventory
        .inventory_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_INVENTORY_EXHAUSTED"))?;
    let promoted_at = trusted_now.to_rfc3339_opts(SecondsFormat::Millis, true);
    inventory.observed_at = promoted_at.clone();

    let mut matching = inventory.plugins.iter_mut().filter(|record| {
        record.plugin_id == plugin_id && record.candidate_slot_ref.as_deref() == Some(slot_ref)
    });
    let record = matching
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECORD_MISSING"))?;
    if matching.next().is_some() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECORD_DUPLICATED");
    }
    if record.desired_presence != DESIRED_PRESENCE_PRESENT
        || record.desired_activation != ACTIVATION_ENABLED
        || record.admission != ADMISSION_ALLOWED
        || record.runtime.phase != RUNTIME_STOPPED
        || record.runtime.slot_ref.is_some()
        || record.runtime.runner_digest.is_some()
        || record.active_attempts != 0
        || candidate_generation <= record.install_generation
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECORD_NOT_QUIESCENT");
    }
    let install_generation_before = record.install_generation;
    let install_generation_after = candidate_generation;
    let activation_generation_before = record.activation_generation;
    let activation_generation_after =
        activation_generation_before.checked_add(1).ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_ACTIVATION_EXHAUSTED")
        })?;
    let mut slots = record.slots.iter_mut().filter(|slot| {
        slot.slot_ref == slot_ref && &slot.release == release && slot.phase == SLOT_STAGED
    });
    let slot = slots
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_SLOT_MISSING"))?;
    if slots.next().is_some() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_SLOT_DUPLICATED");
    }
    slot.phase = SLOT_INSTALLED.to_string();
    slot.phase_changed_at = promoted_at.clone();
    slot.installed_at = Some(promoted_at.clone());
    record.install_generation = install_generation_after;
    record.activation_generation = activation_generation_after;
    record.active_slot_ref = Some(slot_ref.to_string());
    record.candidate_slot_ref = None;
    record.permission_grant_digest = Some(permission_grant_digest.to_string());
    record.health = None;
    record.last_error = None;
    record.last_plan_id = Some(owner_plan_id.to_string());
    record.state_changed_at = promoted_at;

    validate_inventory(&inventory, trusted_now.clone())?;
    let inventory_json = serde_json::to_string(&inventory)
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_INVENTORY_JSON")?;
    let inventory_digest = jcs_sha256_hex(&inventory)?;
    Ok(CandidatePromotionProjection {
        inventory,
        inventory_json,
        inventory_digest,
        state_revision: authority
            .state_revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_STATE_EXHAUSTED"))?,
        authority_epoch: authority
            .authority_epoch
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_EPOCH_EXHAUSTED"))?,
        install_generation_before,
        install_generation_after,
        activation_generation_before,
        activation_generation_after,
    })
}
