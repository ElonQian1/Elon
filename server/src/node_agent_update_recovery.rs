//! Durable protocol shared by local and remote node update recovery paths.

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

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

    fn can_transition_to(self, next: Self) -> bool {
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
                    | Self::Verified
                    | Self::Paused
                    | Self::ApprovalRequired
                    | Self::Conflict
                    | Self::Timeout
            ),
            Self::Resumed => matches!(
                next,
                Self::Verified
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
        }
    }

    #[cfg(test)]
    pub(crate) fn remote_v1() -> Self {
        Self {
            kind: "remote_relay".to_string(),
            protocol: "elon.node.v1".to_string(),
            capabilities: Vec::new(),
        }
    }

    pub(crate) fn supports(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|item| item == capability)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct WorkspaceGitFingerprint {
    #[serde(default)]
    pub(crate) workspace_path: String,
    #[serde(default)]
    pub(crate) git_head: Option<String>,
    #[serde(default)]
    pub(crate) git_status_sha256: Option<String>,
    #[serde(default)]
    pub(crate) git_status_clean: Option<bool>,
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
    pub(crate) workspace: WorkspaceGitFingerprint,
    #[serde(default)]
    pub(crate) transport: RecoveryTransport,
    #[serde(default)]
    pub(crate) recovery_policy: RecoveryPolicy,
    #[serde(default)]
    pub(crate) state: UpdateRecoveryState,
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
            original_task_id: original_task_id.into(),
            resume_task_id: None,
            from_release: ReleaseIdentity::default(),
            to_release: ReleaseIdentity::default(),
            codex_session_id: None,
            codex_session_scope: None,
            sidecar_session_id: None,
            journal_cursor: 0,
            workspace: WorkspaceGitFingerprint::default(),
            transport: RecoveryTransport::local(),
            recovery_policy: RecoveryPolicy::default(),
            state: UpdateRecoveryState::Planned,
            final_reason: None,
            created_at_ms: now,
            updated_at_ms: now,
            events: Vec::new(),
        };
        receipt.push_event(UpdateRecoveryState::Planned, Some("update planned"));
        receipt
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
pub(crate) struct UpdateRecoveryLedger {
    #[serde(default = "default_schema_version")]
    pub(crate) schema_version: u32,
    #[serde(default = "default_protocol")]
    pub(crate) protocol: String,
    #[serde(default)]
    pub(crate) receipts: Vec<UpdateRecoveryReceipt>,
}

impl Default for UpdateRecoveryLedger {
    fn default() -> Self {
        Self {
            schema_version: UPDATE_RECOVERY_SCHEMA_VERSION,
            protocol: UPDATE_RECOVERY_PROTOCOL.to_string(),
            receipts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct UpdateRecoveryStore {
    path: PathBuf,
}

impl UpdateRecoveryStore {
    pub(crate) fn default() -> Self {
        Self::new(super::state_path().with_file_name("node-update-recovery.json"))
    }

    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub(crate) fn load(&self) -> Result<UpdateRecoveryLedger> {
        if !self.path.exists() {
            return Ok(UpdateRecoveryLedger::default());
        }
        let bytes = std::fs::read(&self.path)
            .with_context(|| format!("read update recovery ledger {}", self.path.display()))?;
        serde_json::from_slice(&bytes)
            .with_context(|| format!("parse update recovery ledger {}", self.path.display()))
    }

    pub(crate) fn save(&self, ledger: &UpdateRecoveryLedger) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(ledger)?;
        crate::node_agent_atomic_file::write(&self.path, &bytes)
    }

    pub(crate) fn upsert(&self, receipt: UpdateRecoveryReceipt) -> Result<()> {
        let mut ledger = self.load()?;
        match ledger.receipts.iter_mut().find(|current| {
            current.update_id == receipt.update_id
                && current.original_task_id == receipt.original_task_id
        }) {
            Some(current) => *current = receipt,
            None => ledger.receipts.push(receipt),
        }
        self.save(&ledger)
    }

    pub(crate) fn transition(
        &self,
        update_id: &str,
        original_task_id: &str,
        next: UpdateRecoveryState,
        reason: Option<&str>,
    ) -> Result<bool> {
        let mut ledger = self.load()?;
        let receipt = ledger
            .receipts
            .iter_mut()
            .find(|receipt| {
                receipt.update_id == update_id && receipt.original_task_id == original_task_id
            })
            .context("update recovery receipt not found")?;
        let changed = receipt.transition(next, reason)?;
        if changed {
            self.save(&ledger)?;
        }
        Ok(changed)
    }

    pub(crate) fn active(&self) -> Result<Vec<UpdateRecoveryReceipt>> {
        Ok(self
            .load()?
            .receipts
            .into_iter()
            .filter(|receipt| !receipt.state.is_terminal())
            .collect())
    }
}

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

fn default_recovery_mode() -> String {
    "prefer_sidecar_then_codex_session_then_snapshot_continue".to_string()
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

#[cfg(test)]
#[path = "node_agent_update_recovery_tests.rs"]
mod tests;
