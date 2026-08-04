use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::identity::ComputePluginReleaseRef;

pub(crate) const COMPUTE_PLUGIN_INVENTORY_SCHEMA: &str = "elon.compute_plugin.inventory.v1";

pub(crate) const SLOT_DOWNLOADING: &str = "downloading";
pub(crate) const SLOT_VERIFYING: &str = "verifying";
pub(crate) const SLOT_STAGED: &str = "staged";
pub(crate) const SLOT_INSTALLED: &str = "installed";
pub(crate) const SLOT_REMOVING: &str = "removing";
pub(crate) const SLOT_FAILED: &str = "failed";

pub(crate) const ACTIVATION_ENABLED: &str = "enabled";
pub(crate) const ACTIVATION_DISABLED: &str = "disabled";

pub(crate) const ADMISSION_ALLOWED: &str = "allowed";
pub(crate) const ADMISSION_QUARANTINED: &str = "quarantined";
pub(crate) const ADMISSION_REVOKED: &str = "revoked";

pub(crate) const RUNTIME_STOPPED: &str = "stopped";
pub(crate) const RUNTIME_STARTING: &str = "starting";
pub(crate) const RUNTIME_READY: &str = "ready";
pub(crate) const RUNTIME_DRAINING: &str = "draining";
pub(crate) const RUNTIME_CRASHED: &str = "crashed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginInventorySnapshot {
    pub schema: String,
    pub inventory_revision: i64,
    pub desired_policy_revision: i64,
    pub sharing_enabled: bool,
    pub plugins: Vec<ComputePluginLocalRecord>,
    pub observed_at: String,
}

/// Slots allow an old release to serve while a candidate is downloaded and verified beside it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginLocalRecord {
    pub plugin_id: String,
    pub last_plan_id: Option<String>,
    pub install_generation: i64,
    pub activation_generation: i64,
    pub active_slot_ref: Option<String>,
    pub candidate_slot_ref: Option<String>,
    pub slots: Vec<ComputePluginSlotRecord>,
    pub desired_activation: String,
    pub admission: String,
    pub runtime: ComputePluginRuntimeObservation,
    pub permission_grant_digest: Option<String>,
    pub active_attempts: i64,
    pub health: Option<ComputePluginHealthObservation>,
    pub last_error: Option<ComputePluginSanitizedError>,
    pub state_changed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginSlotRecord {
    pub slot_ref: String,
    pub release: ComputePluginReleaseRef,
    pub phase: String,
    pub phase_changed_at: String,
    pub installed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginRuntimeObservation {
    pub phase: String,
    pub runtime_generation: i64,
    pub slot_ref: Option<String>,
    pub runner_digest: Option<String>,
    pub started_at: Option<String>,
    pub stopped_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginHealthObservation {
    pub status: String,
    pub observation_digest: String,
    pub runtime_generation: i64,
    pub slot_ref: String,
    pub runner_digest: String,
    pub reason_codes: Vec<String>,
    pub observed_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginSanitizedError {
    pub code: String,
    pub safe_message: String,
    pub observed_at: String,
}

pub(crate) fn is_valid_slot_transition(from: Option<&str>, to: Option<&str>) -> bool {
    from == to
        || matches!(
            (from, to),
            (None, Some(SLOT_DOWNLOADING))
                | (Some(SLOT_DOWNLOADING), Some(SLOT_VERIFYING))
                | (Some(SLOT_VERIFYING), Some(SLOT_STAGED))
                | (Some(SLOT_STAGED), Some(SLOT_INSTALLED))
                | (Some(SLOT_INSTALLED), Some(SLOT_REMOVING))
                | (Some(SLOT_DOWNLOADING), Some(SLOT_REMOVING))
                | (Some(SLOT_VERIFYING), Some(SLOT_REMOVING))
                | (Some(SLOT_STAGED), Some(SLOT_REMOVING))
                | (Some(SLOT_REMOVING), None)
                | (Some(SLOT_DOWNLOADING), Some(SLOT_FAILED))
                | (Some(SLOT_VERIFYING), Some(SLOT_FAILED))
                | (Some(SLOT_STAGED), Some(SLOT_FAILED))
                | (Some(SLOT_REMOVING), Some(SLOT_FAILED))
                | (Some(SLOT_FAILED), Some(SLOT_DOWNLOADING))
                | (Some(SLOT_FAILED), Some(SLOT_REMOVING))
                | (Some(SLOT_FAILED), None)
        )
}

pub(crate) fn is_valid_runtime_transition(from: &str, to: &str) -> bool {
    from == to
        || matches!(
            (from, to),
            (RUNTIME_STOPPED, RUNTIME_STARTING)
                | (RUNTIME_STARTING, RUNTIME_READY)
                | (RUNTIME_STARTING, RUNTIME_CRASHED)
                | (RUNTIME_READY, RUNTIME_DRAINING)
                | (RUNTIME_READY, RUNTIME_CRASHED)
                | (RUNTIME_DRAINING, RUNTIME_STOPPED)
                | (RUNTIME_DRAINING, RUNTIME_CRASHED)
                | (RUNTIME_CRASHED, RUNTIME_STOPPED)
                | (RUNTIME_CRASHED, RUNTIME_STARTING)
        )
}

pub(crate) fn local_record_shape_is_valid(record: &ComputePluginLocalRecord) -> bool {
    if record.install_generation < 0
        || record.activation_generation < 0
        || record.active_attempts < 0
        || record.active_slot_ref == record.candidate_slot_ref && record.active_slot_ref.is_some()
    {
        return false;
    }
    if !matches!(
        record.desired_activation.as_str(),
        ACTIVATION_ENABLED | ACTIVATION_DISABLED
    ) || !matches!(
        record.admission.as_str(),
        ADMISSION_ALLOWED | ADMISSION_QUARANTINED | ADMISSION_REVOKED
    ) || !matches!(
        record.runtime.phase.as_str(),
        RUNTIME_STOPPED | RUNTIME_STARTING | RUNTIME_READY | RUNTIME_DRAINING | RUNTIME_CRASHED
    ) {
        return false;
    }
    let mut slot_refs = HashSet::new();
    if !record.slots.iter().all(|slot| {
        slot.release.plugin_id == record.plugin_id
            && matches!(
                slot.phase.as_str(),
                SLOT_DOWNLOADING
                    | SLOT_VERIFYING
                    | SLOT_STAGED
                    | SLOT_INSTALLED
                    | SLOT_REMOVING
                    | SLOT_FAILED
            )
            && slot_refs.insert(slot.slot_ref.as_str())
    }) {
        return false;
    }
    let slot_with_phase = |slot_ref: &Option<String>, phase: &str| {
        slot_ref.as_ref().is_none_or(|wanted| {
            record
                .slots
                .iter()
                .any(|slot| &slot.slot_ref == wanted && slot.phase == phase)
        })
    };
    let transient_slots_are_owned = record.slots.iter().all(|slot| {
        !matches!(
            slot.phase.as_str(),
            SLOT_DOWNLOADING | SLOT_VERIFYING | SLOT_STAGED
        ) || record.candidate_slot_ref.as_deref() == Some(slot.slot_ref.as_str())
    });
    transient_slots_are_owned
        && slot_with_phase(&record.active_slot_ref, SLOT_INSTALLED)
        && record.candidate_slot_ref.as_ref().is_none_or(|wanted| {
            record.slots.iter().any(|slot| {
                &slot.slot_ref == wanted
                    && matches!(
                        slot.phase.as_str(),
                        SLOT_DOWNLOADING | SLOT_VERIFYING | SLOT_STAGED | SLOT_FAILED
                    )
            })
        })
        && (record.runtime.phase != RUNTIME_READY
            || (record.active_slot_ref.is_some()
                && record.runtime.slot_ref == record.active_slot_ref
                && record.runtime.runner_digest.is_some()))
}

pub(crate) fn can_remove_active_slot(record: &ComputePluginLocalRecord) -> bool {
    local_record_shape_is_valid(record)
        && record.active_slot_ref.is_some()
        && record.candidate_slot_ref.is_none()
        && record.desired_activation == ACTIVATION_DISABLED
        && record.runtime.phase == RUNTIME_STOPPED
        && record.active_attempts == 0
}
