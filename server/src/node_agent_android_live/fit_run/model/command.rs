use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{validate_identifier, FitRect, FitRunDocument};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum FitCommand {
    Start {
        #[serde(rename = "commandId")]
        command_id: String,
    },
    Pause {
        #[serde(rename = "commandId")]
        command_id: String,
    },
    Resume {
        #[serde(rename = "commandId")]
        command_id: String,
    },
    Cancel {
        #[serde(rename = "commandId")]
        command_id: String,
    },
    RebindSession {
        #[serde(rename = "commandId")]
        command_id: String,
        #[serde(rename = "newSessionId")]
        new_session_id: String,
        #[serde(default, rename = "newRuntimeNodeId")]
        new_runtime_node_id: Option<String>,
        #[serde(default, rename = "newCurrentRect")]
        new_current_rect: Option<FitRect>,
    },
    AcceptBest {
        #[serde(rename = "commandId")]
        command_id: String,
    },
    CodexStarted {
        #[serde(rename = "commandId")]
        command_id: String,
        #[serde(rename = "handoffId")]
        handoff_id: String,
        #[serde(rename = "taskId")]
        task_id: String,
    },
    CodexCompleted {
        #[serde(rename = "commandId")]
        command_id: String,
        #[serde(rename = "handoffId")]
        handoff_id: String,
        #[serde(rename = "taskId")]
        task_id: Option<String>,
        #[serde(rename = "sourceRevisionBefore")]
        source_revision_before: Option<String>,
        #[serde(rename = "sourceRevisionAfter")]
        source_revision_after: String,
        #[serde(default, rename = "changedFiles")]
        changed_files: Vec<String>,
        #[serde(rename = "commitId")]
        commit_id: Option<String>,
        #[serde(rename = "tokenUsage")]
        token_usage: Option<u64>,
    },
    CodexFailed {
        #[serde(rename = "commandId")]
        command_id: String,
        #[serde(rename = "handoffId")]
        handoff_id: String,
        error: String,
    },
}

impl FitCommand {
    pub(crate) fn command_id(&self) -> &str {
        match self {
            Self::Start { command_id }
            | Self::Pause { command_id }
            | Self::Resume { command_id }
            | Self::Cancel { command_id }
            | Self::RebindSession { command_id, .. }
            | Self::AcceptBest { command_id }
            | Self::CodexStarted { command_id, .. }
            | Self::CodexCompleted { command_id, .. }
            | Self::CodexFailed { command_id, .. } => command_id,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        validate_identifier(self.command_id(), "commandId")?;
        if let Self::RebindSession {
            new_runtime_node_id,
            new_current_rect,
            ..
        } = self
        {
            if new_runtime_node_id
                .as_deref()
                .is_some_and(|value| value.trim().is_empty() || value.len() > 256)
            {
                anyhow::bail!("newRuntimeNodeId 非法");
            }
            if let Some(rect) = new_current_rect {
                rect.validate("newCurrentRect")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitCommandResult {
    pub(crate) run: FitRunDocument,
    pub(crate) idempotent: bool,
}
