use anyhow::{bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::{validate_identifier, FitRunDocument, FitStateReplay};

const MAX_AUDIT_EVENTS: usize = 128;

#[derive(Debug, Clone)]
pub(crate) struct AttachStateReplayRequest {
    pub(crate) command_id: String,
    pub(crate) project_root: String,
    pub(crate) scenario: String,
    pub(crate) state_replay: FitStateReplay,
    pub(crate) target_runtime_node_id: String,
    pub(crate) target_definition_id: String,
    pub(crate) target_instance_key: Option<String>,
}

impl AttachStateReplayRequest {
    pub(crate) fn validate_identity(&self) -> Result<()> {
        validate_identifier(&self.command_id, "commandId")?;
        validate_identifier(&self.scenario, "scenario")?;
        if self.project_root.trim().is_empty() || self.project_root.len() > 4096 {
            bail!("ATTACH_STATE_REPLAY 缺少 projectRoot");
        }
        if self.target_runtime_node_id.trim().is_empty()
            || self.target_runtime_node_id.len() > 500
            || self.target_definition_id.trim().is_empty()
            || self.target_definition_id.len() > 500
            || self
                .target_instance_key
                .as_deref()
                .is_some_and(|value| value.len() > 500)
        {
            bail!("ATTACH_STATE_REPLAY target node 非法");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttachStateReplayResult {
    pub(crate) run: FitRunDocument,
    pub(crate) idempotent: bool,
    pub(crate) replay_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitRunAuditEvent {
    pub(crate) event_id: String,
    pub(crate) action: String,
    pub(crate) outcome: FitRunAuditOutcome,
    pub(crate) command_id: String,
    pub(crate) session_id: String,
    pub(crate) scenario: String,
    pub(crate) replay_sha256: String,
    pub(crate) previous_replay_sha256: Option<String>,
    pub(crate) target_runtime_node_id: String,
    pub(crate) target_definition_id: String,
    pub(crate) target_instance_key: Option<String>,
    pub(crate) detail: String,
    pub(crate) recorded_at: String,
}

impl FitRunAuditEvent {
    pub(crate) fn state_replay_attachment(
        request: &AttachStateReplayRequest,
        outcome: FitRunAuditOutcome,
        replay_sha256: String,
        previous_replay_sha256: Option<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            event_id: format!("audit_{}", uuid::Uuid::new_v4().simple()),
            action: "ATTACH_STATE_REPLAY".to_string(),
            outcome,
            command_id: request.command_id.clone(),
            session_id: String::new(),
            scenario: request.scenario.clone(),
            replay_sha256,
            previous_replay_sha256,
            target_runtime_node_id: request.target_runtime_node_id.clone(),
            target_definition_id: request.target_definition_id.clone(),
            target_instance_key: request.target_instance_key.clone(),
            detail: detail.into(),
            recorded_at: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum FitRunAuditOutcome {
    Attached,
    Idempotent,
    RejectedConflict,
    RejectedInvalid,
    RejectedImmutable,
    RejectedTargetMissing,
}

impl FitRunDocument {
    pub(crate) fn record_audit_event(&mut self, mut event: FitRunAuditEvent) {
        event.session_id = self.session_id.clone();
        self.audit_events.push(event);
        if self.audit_events.len() > MAX_AUDIT_EVENTS {
            let overflow = self.audit_events.len() - MAX_AUDIT_EVENTS;
            self.audit_events.drain(0..overflow);
        }
        self.touch();
    }
}
