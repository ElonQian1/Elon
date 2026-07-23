//! Durable protocol shared by local and remote node update recovery paths.

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[path = "node_agent_update_recovery_receipt_merge.rs"]
mod receipt_merge;
use receipt_merge::canonical_terminal_receipt;
#[path = "node_agent_update_recovery_ledger_lock.rs"]
mod ledger_lock;
pub(crate) use ledger_lock::ledger_mutation_guard;
#[path = "node_agent_update_reconcile_receipt.rs"]
mod reconcile_receipt;
pub(crate) use reconcile_receipt::UpdateGateReconcileReceipt;

pub(crate) const UPDATE_RECOVERY_PROTOCOL: &str = "elon.node_update_recovery.v1";
pub(crate) const UPDATE_RECOVERY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UpdateRecoveryState {
    #[default]
    Planned,
    Downloaded,
    CheckpointSaved,
    Applying,
    RuntimeOnline,
    Reattaching,
    ResumeCreated,
    Resumed,
    Verified,
    Paused,
    ApprovalRequired,
    Conflict,
    Timeout,
    Failed,
}

impl UpdateRecoveryState {
    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Verified | Self::Failed)
    }

    pub(super) fn can_transition_to(self, next: Self) -> bool {
        if self == next {
            return true;
        }
        if next == Self::Failed {
            return self != Self::Verified;
        }
        match self {
            Self::Planned => matches!(next, Self::Downloaded | Self::Paused),
            Self::Downloaded => matches!(next, Self::CheckpointSaved | Self::Paused),
            Self::CheckpointSaved => matches!(next, Self::Applying | Self::Paused),
            Self::Applying => matches!(next, Self::RuntimeOnline | Self::Paused | Self::Timeout),
            Self::RuntimeOnline => matches!(
                next,
                Self::Reattaching
                    | Self::ResumeCreated
                    | Self::Resumed
                    | Self::Verified
                    | Self::Paused
                    | Self::ApprovalRequired
                    | Self::Conflict
                    | Self::Timeout
            ),
            Self::Reattaching => matches!(
                next,
                Self::Resumed
                    | Self::ResumeCreated
                    | Self::Paused
                    | Self::ApprovalRequired
                    | Self::Conflict
                    | Self::Timeout
            ),
            Self::ResumeCreated => matches!(
                next,
                Self::Resumed
                    | Self::Reattaching
                    | Self::Verified
                    | Self::Paused
                    | Self::ApprovalRequired
                    | Self::Conflict
                    | Self::Timeout
            ),
            Self::Resumed => matches!(
                next,
                Self::Verified
                    | Self::Reattaching
                    | Self::ResumeCreated
                    | Self::Paused
                    | Self::ApprovalRequired
                    | Self::Conflict
                    | Self::Timeout
            ),
            Self::Paused | Self::ApprovalRequired | Self::Conflict | Self::Timeout => matches!(
                next,
                Self::RuntimeOnline | Self::Reattaching | Self::ResumeCreated | Self::Resumed
            ),
            Self::Verified | Self::Failed => false,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ReleaseIdentity {
    #[serde(default)]
    pub(crate) version: String,
    #[serde(default)]
    pub(crate) git_sha: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct RecoveryTransport {
    #[serde(default = "default_transport_kind")]
    pub(crate) kind: String,
    #[serde(default = "default_transport_protocol")]
    pub(crate) protocol: String,
    #[serde(default)]
    pub(crate) capabilities: Vec<String>,
    #[serde(default = "default_auth_mode")]
    pub(crate) auth_mode: String,
    #[serde(default)]
    pub(crate) lease_id: Option<String>,
    #[serde(default)]
    pub(crate) lease_expires_at_ms: Option<u128>,
    #[serde(default = "default_true")]
    pub(crate) replay_from_cursor: bool,
}

impl Default for RecoveryTransport {
    fn default() -> Self {
        Self::local()
    }
}

impl RecoveryTransport {
    pub(crate) fn local() -> Self {
        Self {
            kind: default_transport_kind(),
            protocol: default_transport_protocol(),
            capabilities: vec![
                "update_recovery_v1".to_string(),
                "event_replay".to_string(),
                "sidecar_reattach".to_string(),
                "resume_original".to_string(),
            ],
            auth_mode: default_auth_mode(),
            lease_id: None,
            lease_expires_at_ms: None,
            replay_from_cursor: true,
        }
    }

    #[cfg(test)]
    pub(crate) fn remote_v1() -> Self {
        Self {
            kind: "remote_relay".to_string(),
            protocol: "elon.node.v1".to_string(),
            capabilities: Vec::new(),
            auth_mode: "remote_transport_auth".to_string(),
            lease_id: None,
            lease_expires_at_ms: None,
            replay_from_cursor: false,
        }
    }

    pub(crate) fn supports(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|item| item == capability)
    }

    pub(crate) fn allows_local_resume_rebuild(&self) -> bool {
        self.kind == default_transport_kind()
            && self.protocol == default_transport_protocol()
            && self.auth_mode == default_auth_mode()
            && self.replay_from_cursor
            && self.supports("update_recovery_v1")
            && self.supports("event_replay")
            && self.supports("resume_original")
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct WorkspaceGitFingerprint {
    #[serde(default)]
    pub(crate) base_workspace_path: Option<String>,
    #[serde(default)]
    pub(crate) workspace_path: String,
    #[serde(default)]
    pub(crate) isolated: bool,
    #[serde(default)]
    pub(crate) branch: Option<String>,
    #[serde(default)]
    pub(crate) git_head: Option<String>,
    #[serde(default)]
    pub(crate) git_status_sha256: Option<String>,
    #[serde(default)]
    pub(crate) git_status_clean: Option<bool>,
}

impl WorkspaceGitFingerprint {
    pub(crate) fn has_sufficient_identity(&self) -> bool {
        !self.workspace_path.trim().is_empty()
            && (!self.isolated
                || self
                    .base_workspace_path
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty()))
            && self
                .git_head
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            && self
                .git_status_sha256
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct RecoverySafetyEvidence {
    #[serde(default)]
    pub(crate) evidence_complete: bool,
    #[serde(default)]
    pub(crate) pending_approval_ids: Vec<String>,
    #[serde(default)]
    pub(crate) non_repeatable_action: Option<String>,
    #[serde(default)]
    pub(crate) journal_event_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct UpdateRecoveryReview {
    #[serde(default)]
    pub(crate) verdict: String,
    #[serde(default)]
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) reviewed_by: String,
    #[serde(default)]
    pub(crate) reviewed_at_ms: u128,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct RecoveryPolicy {
    #[serde(default = "default_recovery_mode")]
    pub(crate) mode: String,
    #[serde(default = "default_true")]
    pub(crate) allow_snapshot_continue: bool,
    #[serde(default)]
    pub(crate) deadline_ms: Option<u128>,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self {
            mode: default_recovery_mode(),
            allow_snapshot_continue: true,
            deadline_ms: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct UpdateRecoveryEvent {
    pub(crate) event_id: String,
    pub(crate) sequence: u64,
    pub(crate) state: UpdateRecoveryState,
    pub(crate) at_ms: u128,
    #[serde(default)]
    pub(crate) reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct UpdateRecoveryReceipt {
    #[serde(default = "default_schema_version")]
    pub(crate) schema_version: u32,
    #[serde(default = "default_protocol")]
    pub(crate) protocol: String,
    pub(crate) update_id: String,
    pub(crate) root_task_id: String,
    #[serde(default)]
    pub(crate) parent_task_id: Option<String>,
    pub(crate) original_task_id: String,
    #[serde(default)]
    pub(crate) resume_task_id: Option<String>,
    #[serde(default)]
    pub(crate) from_release: ReleaseIdentity,
    #[serde(default)]
    pub(crate) to_release: ReleaseIdentity,
    #[serde(default)]
    pub(crate) codex_session_id: Option<String>,
    #[serde(default)]
    pub(crate) codex_session_scope: Option<String>,
    #[serde(default)]
    pub(crate) sidecar_session_id: Option<String>,
    #[serde(default)]
    pub(crate) journal_cursor: u64,
    #[serde(default)]
    pub(crate) sidecar_output_offset: u64,
    #[serde(default)]
    pub(crate) sidecar_output_sequence: u64,
    #[serde(default = "default_expected_downtime_ms")]
    pub(crate) expected_downtime_ms: u64,
    #[serde(default)]
    pub(crate) workspace: WorkspaceGitFingerprint,
    #[serde(default)]
    pub(crate) transport: RecoveryTransport,
    #[serde(default)]
    pub(crate) recovery_policy: RecoveryPolicy,
    #[serde(default)]
    pub(crate) safety: RecoverySafetyEvidence,
    #[serde(default)]
    pub(crate) resume_strategy: Option<String>,
    #[serde(default)]
    pub(crate) final_review: Option<UpdateRecoveryReview>,
    #[serde(default)]
    pub(crate) completion_event_id: Option<String>,
    #[serde(default)]
    pub(crate) terminal_task_status: Option<String>,
    #[serde(default)]
    pub(crate) terminal_finished_at_ms: Option<u128>,
    #[serde(default)]
    pub(crate) terminal_success: Option<bool>,
    #[serde(default)]
    pub(crate) terminal_outcome: Option<String>,
    /// A later, explicitly targeted node update made this recovery generation
    /// historical. The original receipt and its events stay in the ledger.
    #[serde(default)]
    pub(crate) superseded_by_update_id: Option<String>,
    #[serde(default)]
    pub(crate) superseded_by_release: Option<ReleaseIdentity>,
    #[serde(default)]
    pub(crate) supersede_evidence: Option<String>,
    #[serde(default)]
    pub(crate) superseded_at_ms: Option<u128>,
    /// Multiple receipts matched this task but could not be losslessly merged.
    #[serde(default)]
    pub(crate) conflict_detected: bool,
    #[serde(default)]
    pub(crate) conflict_count: usize,
    #[serde(default)]
    pub(crate) conflict_reason: Option<String>,
    #[serde(default)]
    pub(crate) state: UpdateRecoveryState,
    #[serde(default)]
    pub(crate) state_reason: Option<String>,
    #[serde(default)]
    pub(crate) final_reason: Option<String>,
    #[serde(default)]
    pub(crate) created_at_ms: u128,
    #[serde(default)]
    pub(crate) updated_at_ms: u128,
    #[serde(default)]
    pub(crate) events: Vec<UpdateRecoveryEvent>,
}

impl UpdateRecoveryReceipt {
    pub(crate) fn planned(
        update_id: impl Into<String>,
        root_task_id: impl Into<String>,
        original_task_id: impl Into<String>,
    ) -> Self {
        let now = now_ms();
        let mut receipt = Self {
            schema_version: UPDATE_RECOVERY_SCHEMA_VERSION,
            protocol: UPDATE_RECOVERY_PROTOCOL.to_string(),
            update_id: update_id.into(),
            root_task_id: root_task_id.into(),
            parent_task_id: None,
            original_task_id: original_task_id.into(),
            resume_task_id: None,
            from_release: ReleaseIdentity::default(),
            to_release: ReleaseIdentity::default(),
            codex_session_id: None,
            codex_session_scope: None,
            sidecar_session_id: None,
            journal_cursor: 0,
            sidecar_output_offset: 0,
            sidecar_output_sequence: 0,
            expected_downtime_ms: default_expected_downtime_ms(),
            workspace: WorkspaceGitFingerprint::default(),
            transport: RecoveryTransport::local(),
            recovery_policy: RecoveryPolicy::default(),
            safety: RecoverySafetyEvidence::default(),
            resume_strategy: None,
            final_review: None,
            completion_event_id: None,
            terminal_task_status: None,
            terminal_finished_at_ms: None,
            terminal_success: None,
            terminal_outcome: None,
            superseded_by_update_id: None,
            superseded_by_release: None,
            supersede_evidence: None,
            superseded_at_ms: None,
            conflict_detected: false,
            conflict_count: 0,
            conflict_reason: None,
            state: UpdateRecoveryState::Planned,
            state_reason: None,
            final_reason: None,
            created_at_ms: now,
            updated_at_ms: now,
            events: Vec::new(),
        };
        receipt.push_event(UpdateRecoveryState::Planned, Some("update planned"));
        receipt
    }

    pub(crate) fn active_task_id(&self) -> &str {
        self.resume_task_id
            .as_deref()
            .unwrap_or(&self.original_task_id)
    }

    pub(crate) fn is_superseded(&self) -> bool {
        self.superseded_by_update_id.is_some()
    }

    pub(crate) fn allows_local_reconcile(&self) -> bool {
        self.protocol == UPDATE_RECOVERY_PROTOCOL
            && self.schema_version == UPDATE_RECOVERY_SCHEMA_VERSION
            && self.transport.allows_local_resume_rebuild()
            && self.transport.supports("sidecar_reattach")
    }

    pub(crate) fn transition(
        &mut self,
        next: UpdateRecoveryState,
        reason: Option<&str>,
    ) -> Result<bool> {
        if !self.state.can_transition_to(next) {
            bail!(
                "invalid update recovery transition {:?} -> {:?}",
                self.state,
                next
            );
        }
        if self.state == next {
            return Ok(false);
        }
        self.state = next;
        self.state_reason = reason.map(str::to_string);
        self.final_reason = if next.is_terminal() {
            reason.map(str::to_string)
        } else {
            None
        };
        self.push_event(next, reason);
        Ok(true)
    }

    fn push_event(&mut self, state: UpdateRecoveryState, reason: Option<&str>) {
        let sequence = self
            .events
            .last()
            .map(|event| event.sequence + 1)
            .unwrap_or(1);
        let at_ms = now_ms();
        self.updated_at_ms = at_ms;
        self.events.push(UpdateRecoveryEvent {
            event_id: format!("{}:{}:{state:?}", self.update_id, sequence).to_ascii_lowercase(),
            sequence,
            state,
            at_ms,
            reason: reason.map(str::to_string),
        });
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct UpdateInstallGate {
    #[serde(default = "default_install_gate_phase")]
    pub(crate) phase: String,
    #[serde(default)]
    pub(crate) target_git_sha: String,
    #[serde(default)]
    pub(crate) active_foreground_task_ids: Vec<String>,
    #[serde(default)]
    pub(crate) safe_checkpoint_count: usize,
    #[serde(default = "default_update_gate_capability")]
    pub(crate) capability: String,
    #[serde(default)]
    pub(crate) reason: Option<String>,
    #[serde(default)]
    pub(crate) excluded_terminal_history_count: usize,
    #[serde(default)]
    pub(crate) reconcile_id: Option<String>,
    #[serde(default)]
    pub(crate) reconciled_at_ms: Option<u128>,
    #[serde(default)]
    pub(crate) classifications: Vec<UpdateGateTaskClassification>,
    #[serde(default)]
    pub(crate) updated_at_ms: u128,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct UpdateGateTaskClassification {
    pub(crate) task_id: String,
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) finished_at_ms: Option<i64>,
    #[serde(default)]
    pub(crate) fresh_runtime_handle: bool,
    #[serde(default)]
    pub(crate) live_sidecar: bool,
    #[serde(default)]
    pub(crate) replayable_sidecar: bool,
    #[serde(default)]
    pub(crate) live_journal_process: bool,
    #[serde(default)]
    pub(crate) pending_approval_ids: Vec<String>,
    #[serde(default)]
    pub(crate) non_repeatable_action: Option<String>,
    #[serde(default)]
    pub(crate) terminal_recovery_receipt: bool,
    #[serde(default)]
    pub(crate) recovery_receipt_count: usize,
    #[serde(default)]
    pub(crate) terminal_recovery_receipt_count: usize,
    #[serde(default)]
    pub(crate) ambiguous_recovery_receipts: bool,
    #[serde(default)]
    pub(crate) cancel_intent_persisted: bool,
    #[serde(default)]
    pub(crate) cancel_side_effect_committed: bool,
    #[serde(default)]
    pub(crate) resume_eligible: Option<bool>,
    #[serde(default)]
    pub(crate) resume_ineligibility_proof: Option<String>,
    #[serde(default)]
    pub(crate) excluded_from_install_blockers: bool,
    pub(crate) reason: String,
}

impl Default for UpdateInstallGate {
    fn default() -> Self {
        Self {
            phase: default_install_gate_phase(),
            target_git_sha: String::new(),
            active_foreground_task_ids: Vec::new(),
            safe_checkpoint_count: 0,
            capability: default_update_gate_capability(),
            reason: None,
            excluded_terminal_history_count: 0,
            reconcile_id: None,
            reconciled_at_ms: None,
            classifications: Vec::new(),
            updated_at_ms: now_ms(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct UpdateRecoveryLedger {
    #[serde(default = "default_schema_version")]
    pub(crate) schema_version: u32,
    #[serde(default = "default_protocol")]
    pub(crate) protocol: String,
    #[serde(default)]
    pub(crate) install_gate: UpdateInstallGate,
    #[serde(default)]
    pub(crate) receipts: Vec<UpdateRecoveryReceipt>,
    #[serde(default)]
    pub(crate) reconcile_receipts: Vec<UpdateGateReconcileReceipt>,
}

impl Default for UpdateRecoveryLedger {
    fn default() -> Self {
        Self {
            schema_version: UPDATE_RECOVERY_SCHEMA_VERSION,
            protocol: UPDATE_RECOVERY_PROTOCOL.to_string(),
            install_gate: UpdateInstallGate::default(),
            receipts: Vec::new(),
            reconcile_receipts: Vec::new(),
        }
    }
}

impl UpdateRecoveryLedger {
    pub(crate) fn receipt_for_task(&self, task_id: &str) -> Result<Option<UpdateRecoveryReceipt>> {
        let matches = self
            .receipts
            .iter()
            .filter(|receipt| {
                receipt.original_task_id == task_id
                    || receipt.resume_task_id.as_deref() == Some(task_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        let current = matches
            .iter()
            .filter(|receipt| !receipt.is_superseded())
            .cloned()
            .collect::<Vec<_>>();
        let candidates = if current.is_empty() { matches } else { current };
        match candidates.len() {
            0 => Ok(None),
            1 => Ok(candidates.into_iter().next()),
            _ => canonical_terminal_receipt(&candidates).map(Some),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct UpdateRecoveryStore {
    path: PathBuf,
}

#[path = "node_agent_update_recovery_store.rs"]
mod store;

fn default_protocol() -> String {
    UPDATE_RECOVERY_PROTOCOL.to_string()
}

fn default_schema_version() -> u32 {
    UPDATE_RECOVERY_SCHEMA_VERSION
}

fn default_transport_kind() -> String {
    "local_loopback".to_string()
}

fn default_transport_protocol() -> String {
    "elon.desktop_pc_supervision.v1".to_string()
}

fn default_auth_mode() -> String {
    "loopback_admin_token".to_string()
}

fn default_expected_downtime_ms() -> u64 {
    45_000
}

fn default_recovery_mode() -> String {
    "prefer_sidecar_then_codex_session_then_snapshot_continue".to_string()
}

fn default_install_gate_phase() -> String {
    "idle".to_string()
}

fn default_update_gate_capability() -> String {
    "local_update_gate_v1".to_string()
}

fn default_true() -> bool {
    true
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[path = "node_agent_update_recovery_runtime.rs"]
mod runtime;

#[cfg(test)]
#[path = "node_agent_update_recovery_tests.rs"]
mod tests;
