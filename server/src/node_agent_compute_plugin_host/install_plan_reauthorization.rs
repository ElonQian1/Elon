use anyhow::{bail, Result};

use super::{
    identity::ComputePluginReleaseRef,
    install_plan::{
        ComputePluginPlanItem, SignedComputePluginInstallPlan, PLAN_ACTION_CANCEL_CANDIDATE,
        PLAN_ACTION_DISABLE, PLAN_ACTION_INSTALL, PLAN_ACTION_KEEP,
        PLAN_ACTION_REAUTHORIZE_EXISTING, PLAN_ACTION_REMOVE, PLAN_ACTION_UPGRADE,
        PLAN_TARGET_ENABLED,
    },
    lifecycle::{
        ComputePluginInventorySnapshot, ComputePluginLocalRecord, ACTIVATION_DISABLED,
        ACTIVATION_ENABLED, ADMISSION_ALLOWED, DESIRED_PRESENCE_ABSENT, DESIRED_PRESENCE_PRESENT,
        RUNTIME_STOPPED, SLOT_INSTALLED,
    },
};

/// Returns the publisher-signed Manifest release that must accompany this plan item.
///
/// Install/upgrade bind `target_release` elsewhere. Reauthorization deliberately has no target
/// download, so its Manifest source is the exact currently active release instead.
pub(super) fn manifest_release_for_item(
    item: &ComputePluginPlanItem,
) -> Option<&ComputePluginReleaseRef> {
    item.target_release.as_ref().or_else(|| {
        (item.action == PLAN_ACTION_REAUTHORIZE_EXISTING)
            .then_some(item.expected_current_release.as_ref())
            .flatten()
    })
}

pub(super) fn manifest_binding_action_is_valid(item: &ComputePluginPlanItem) -> bool {
    matches!(
        item.action.as_str(),
        PLAN_ACTION_INSTALL | PLAN_ACTION_UPGRADE | PLAN_ACTION_REAUTHORIZE_EXISTING
    )
}

pub(super) fn reauthorization_shape_is_valid(item: &ComputePluginPlanItem) -> bool {
    item.expected_current_release.is_some()
        && item.expected_candidate_release.is_none()
        && item
            .expected_install_generation
            .is_some_and(|value| value > 0)
        && item.target_release.is_none()
        && item.downloads.is_empty()
        && item.grant.is_some()
        && item.target_activation == PLAN_TARGET_ENABLED
}

pub(super) fn validate_reauthorization_source(
    item: &ComputePluginPlanItem,
    record: &ComputePluginLocalRecord,
) -> Result<()> {
    if item.action != PLAN_ACTION_REAUTHORIZE_EXISTING {
        return Ok(());
    }
    let active_slot_ref = record
        .active_slot_ref
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_REAUTHORIZE_ACTIVE_SLOT_MISSING"))?;
    let active_slot = record
        .slots
        .iter()
        .find(|slot| slot.slot_ref == active_slot_ref)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_REAUTHORIZE_ACTIVE_SLOT_MISSING"))?;
    if active_slot.phase != SLOT_INSTALLED
        || Some(&active_slot.release) != item.expected_current_release.as_ref()
        || record.candidate_slot_ref.is_some()
        || record.install_generation <= 0
        || item.expected_install_generation != Some(record.install_generation)
        || record.runtime.phase != RUNTIME_STOPPED
        || record.runtime.slot_ref.is_some()
        || record.runtime.runner_digest.is_some()
        || record.health.is_some()
        || record.active_attempts != 0
    {
        bail!("COMPUTE_PLUGIN_REAUTHORIZE_SOURCE_NOT_QUIESCENT");
    }
    Ok(())
}

fn reauthorize(
    item: &ComputePluginPlanItem,
    record: Option<&mut ComputePluginLocalRecord>,
    plan_id: &str,
    observed_at: &str,
) -> Result<()> {
    let record =
        record.ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REAUTHORIZE_MISSING"))?;
    validate_reauthorization_source(item, record)?;
    let grant = item
        .grant
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_REAUTHORIZE_GRANT_MISSING"))?;
    record.last_plan_id = Some(plan_id.to_string());
    record.desired_presence = DESIRED_PRESENCE_PRESENT.to_string();
    record.desired_activation = ACTIVATION_ENABLED.to_string();
    record.admission = ADMISSION_ALLOWED.to_string();
    record.permission_grant_digest = Some(grant.grant_digest.clone());
    record.state_changed_at = observed_at.to_string();
    Ok(())
}

pub(super) fn apply_existing_record_action(
    item: &ComputePluginPlanItem,
    record: Option<&mut ComputePluginLocalRecord>,
    plan_id: &str,
    observed_at: &str,
) -> Result<()> {
    if item.action == PLAN_ACTION_REAUTHORIZE_EXISTING {
        return reauthorize(item, record, plan_id, observed_at);
    }
    let record = record.ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_KEEP_MISSING"))?;
    record.desired_activation = if item.target_activation == PLAN_TARGET_ENABLED {
        ACTIVATION_ENABLED.to_string()
    } else {
        ACTIVATION_DISABLED.to_string()
    };
    record.desired_presence = DESIRED_PRESENCE_PRESENT.to_string();
    touch_plan_record(record, plan_id, observed_at);
    Ok(())
}

pub(super) fn touch_plan_record(
    record: &mut ComputePluginLocalRecord,
    plan_id: &str,
    observed_at: &str,
) {
    record.last_plan_id = Some(plan_id.to_string());
    record.state_changed_at = observed_at.to_string();
}

pub(super) fn validate_reauthorization_projection(
    item: &ComputePluginPlanItem,
    record: &ComputePluginLocalRecord,
) -> Result<()> {
    if item.action != PLAN_ACTION_REAUTHORIZE_EXISTING {
        return Ok(());
    }
    validate_reauthorization_source(item, record)?;
    let expected_grant = item.grant.as_ref().map(|grant| grant.grant_digest.as_str());
    if record.admission != ADMISSION_ALLOWED
        || record.permission_grant_digest.as_deref() != expected_grant
        || record.desired_presence != DESIRED_PRESENCE_PRESENT
        || record.desired_activation != ACTIVATION_ENABLED
    {
        bail!("COMPUTE_PLUGIN_REAUTHORIZE_PROJECTION_MISMATCH");
    }
    Ok(())
}

pub(super) fn validate_replayed_inventory_intent(
    signed_plan: &SignedComputePluginInstallPlan,
    inventory_after: &ComputePluginInventorySnapshot,
) -> Result<()> {
    for item in &signed_plan.plan.items {
        let plugin_id = item
            .target_release
            .as_ref()
            .or(item.expected_candidate_release.as_ref())
            .or(item.expected_current_release.as_ref())
            .map(|release| release.plugin_id.as_str())
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_INVENTORY_ITEM"))?;
        let record = inventory_after
            .plugins
            .iter()
            .find(|record| record.plugin_id == plugin_id)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_INVENTORY_RECORD"))?;
        let expected_presence = match item.action.as_str() {
            PLAN_ACTION_REMOVE => DESIRED_PRESENCE_ABSENT,
            PLAN_ACTION_CANCEL_CANDIDATE if record.active_slot_ref.is_none() => {
                DESIRED_PRESENCE_ABSENT
            }
            PLAN_ACTION_INSTALL
            | PLAN_ACTION_UPGRADE
            | PLAN_ACTION_KEEP
            | PLAN_ACTION_REAUTHORIZE_EXISTING
            | PLAN_ACTION_DISABLE
            | PLAN_ACTION_CANCEL_CANDIDATE => DESIRED_PRESENCE_PRESENT,
            _ => bail!("COMPUTE_PLUGIN_PLAN_REPLAY_INVENTORY_ACTION"),
        };
        let expected_activation = if item.action == PLAN_ACTION_REMOVE {
            ACTIVATION_DISABLED
        } else {
            item.target_activation.as_str()
        };
        if record.last_plan_id.as_deref() != Some(signed_plan.plan.plan_id.as_str())
            || record.desired_presence != expected_presence
            || record.desired_activation != expected_activation
        {
            bail!("COMPUTE_PLUGIN_PLAN_REPLAY_INVENTORY_INTENT_BINDING");
        }
        validate_reauthorization_projection(item, record)?;
    }
    Ok(())
}
