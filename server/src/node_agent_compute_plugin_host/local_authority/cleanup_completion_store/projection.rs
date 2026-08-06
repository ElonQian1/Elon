use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};

use super::super::plan_application::AuthorityPlanApplicationState;
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    install_plan_admission::validate_inventory,
    lifecycle::{is_valid_slot_transition, ComputePluginInventorySnapshot, SLOT_FAILED},
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) struct CandidateCleanupCompletionProjection {
    pub(super) inventory: ComputePluginInventorySnapshot,
    pub(super) inventory_json: String,
    pub(super) inventory_digest: String,
    pub(super) state_revision: i64,
    pub(super) authority_epoch: i64,
}

pub(super) fn project_candidate_cleanup_completion(
    authority: &AuthorityPlanApplicationState,
    plugin_id: &str,
    slot_ref: &str,
    release: &ComputePluginReleaseRef,
    trusted_now: &DateTime<Utc>,
) -> Result<CandidateCleanupCompletionProjection> {
    if !is_valid_slot_transition(Some(SLOT_FAILED), None) {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_SLOT_TRANSITION_INVALID");
    }
    let mut inventory = authority.inventory.clone();
    inventory.inventory_revision =
        inventory.inventory_revision.checked_add(1).ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_INVENTORY_EXHAUSTED")
        })?;
    let observed_at = trusted_now.to_rfc3339_opts(SecondsFormat::Millis, true);
    inventory.observed_at = observed_at.clone();

    remove_failed_candidate(&mut inventory, plugin_id, slot_ref, release, &observed_at)?;

    validate_inventory(&inventory, trusted_now.clone())?;
    let inventory_json = serde_json::to_string(&inventory)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_INVENTORY_JSON")?;
    let inventory_digest = jcs_sha256_hex(&inventory)?;
    if inventory_digest == authority.inventory_digest {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_INVENTORY_UNCHANGED");
    }
    Ok(CandidateCleanupCompletionProjection {
        inventory,
        inventory_json,
        inventory_digest,
        state_revision: authority.state_revision.checked_add(1).ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_STATE_EXHAUSTED")
        })?,
        authority_epoch: authority.authority_epoch.checked_add(1).ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_EPOCH_EXHAUSTED")
        })?,
    })
}

fn remove_failed_candidate(
    inventory: &mut ComputePluginInventorySnapshot,
    plugin_id: &str,
    slot_ref: &str,
    release: &ComputePluginReleaseRef,
    observed_at: &str,
) -> Result<()> {
    let matching_records = inventory
        .plugins
        .iter()
        .enumerate()
        .filter(|(_, record)| record.plugin_id == plugin_id)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if matching_records.len() != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECORD_CHANGED");
    }
    let record = &mut inventory.plugins[matching_records[0]];
    if record.candidate_slot_ref.as_deref() != Some(slot_ref) {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_CANDIDATE_POINTER_CHANGED");
    }
    let matching_slots = record
        .slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.slot_ref == slot_ref)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if matching_slots.len() != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_SLOT_CHANGED");
    }
    let slot_index = matching_slots[0];
    let slot = &record.slots[slot_index];
    if slot.phase != SLOT_FAILED || &slot.release != release {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_SLOT_CHANGED");
    }

    record.slots.remove(slot_index);
    record.candidate_slot_ref = None;
    record.state_changed_at = observed_at.to_string();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_compute_plugin_host::lifecycle::{
        ComputePluginLocalRecord, ComputePluginRuntimeObservation, ComputePluginSlotRecord,
        ACTIVATION_ENABLED, ADMISSION_ALLOWED, COMPUTE_PLUGIN_INVENTORY_SCHEMA,
        DESIRED_PRESENCE_PRESENT, RUNTIME_STOPPED, SLOT_INSTALLED,
    };

    fn release(version: &str) -> ComputePluginReleaseRef {
        ComputePluginReleaseRef {
            plugin_id: "merchant_plugin".to_string(),
            plugin_version: version.to_string(),
            target_id: "windows_x86_64".to_string(),
            manifest_digest: "a".repeat(64),
            package_digest: "b".repeat(64),
        }
    }

    fn inventory() -> ComputePluginInventorySnapshot {
        let active = release("1.0.0");
        let candidate = release("2.0.0");
        ComputePluginInventorySnapshot {
            schema: COMPUTE_PLUGIN_INVENTORY_SCHEMA.to_string(),
            inventory_revision: 7,
            desired_policy_revision: 3,
            sharing_enabled: true,
            plugins: vec![ComputePluginLocalRecord {
                plugin_id: "merchant_plugin".to_string(),
                last_plan_id: Some("plan_1".to_string()),
                install_generation: 2,
                activation_generation: 1,
                active_slot_ref: Some("active_a".to_string()),
                candidate_slot_ref: Some("candidate_b".to_string()),
                slots: vec![
                    ComputePluginSlotRecord {
                        slot_ref: "active_a".to_string(),
                        release: active,
                        phase: SLOT_INSTALLED.to_string(),
                        phase_changed_at: "2026-08-07T00:00:00.000Z".to_string(),
                        installed_at: Some("2026-08-07T00:00:00.000Z".to_string()),
                    },
                    ComputePluginSlotRecord {
                        slot_ref: "candidate_b".to_string(),
                        release: candidate,
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
                permission_grant_digest: None,
                active_attempts: 0,
                health: None,
                last_error: None,
                state_changed_at: "2026-08-07T00:01:00.000Z".to_string(),
            }],
            observed_at: "2026-08-07T00:01:00.000Z".to_string(),
        }
    }

    #[test]
    fn cleanup_removes_only_failed_candidate_and_preserves_active_slot() {
        let mut inventory = inventory();
        let failed_release = release("2.0.0");
        remove_failed_candidate(
            &mut inventory,
            "merchant_plugin",
            "candidate_b",
            &failed_release,
            "2026-08-07T00:02:00.000Z",
        )
        .unwrap();

        let record = &inventory.plugins[0];
        assert_eq!(record.active_slot_ref.as_deref(), Some("active_a"));
        assert_eq!(record.candidate_slot_ref, None);
        assert_eq!(record.slots.len(), 1);
        assert_eq!(record.slots[0].slot_ref, "active_a");
        validate_removed_candidate_inventory(&inventory, "merchant_plugin", "candidate_b").unwrap();
    }

    #[test]
    fn cleanup_rejects_duplicate_candidate_slot_identity() {
        let mut inventory = inventory();
        let duplicate = inventory.plugins[0].slots[1].clone();
        inventory.plugins[0].slots.push(duplicate);
        let error = remove_failed_candidate(
            &mut inventory,
            "merchant_plugin",
            "candidate_b",
            &release("2.0.0"),
            "2026-08-07T00:02:00.000Z",
        )
        .unwrap_err();

        assert!(format!("{error:#}").contains("SLOT_CHANGED"));
    }
}

pub(super) fn validate_removed_candidate_inventory(
    inventory: &ComputePluginInventorySnapshot,
    plugin_id: &str,
    slot_ref: &str,
) -> Result<()> {
    let matching = inventory
        .plugins
        .iter()
        .filter(|record| record.plugin_id == plugin_id)
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_RECORD_CHANGED");
    }
    let record = matching[0];
    if record.candidate_slot_ref.is_some()
        || record.slots.iter().any(|slot| slot.slot_ref == slot_ref)
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_COMPLETION_SLOT_NOT_REMOVED");
    }
    Ok(())
}
