use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub(crate) struct FitSessionContext {
    pub(crate) session_id: String,
    pub(crate) project_root: String,
    pub(crate) package_name: String,
    pub(crate) device_id: String,
    pub(crate) runtime_build_id: Option<String>,
    pub(crate) tree_revision: u64,
    pub(crate) source_revision: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum FitRunPhase {
    Created,
    Baselining,
    LocalSolving,
    AwaitingCodex,
    CodexRunning,
    Rebuilding,
    Evaluating,
    CandidateReady,
    SourceVerifying,
    Paused,
    Accepted,
    Plateau,
    Failed,
    Cancelled,
}

impl FitRunPhase {
    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Accepted | Self::Plateau | Self::Failed | Self::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum FitStopReason {
    TargetReached,
    SourceVerified,
    DurationBudgetExhausted,
    LocalEvaluationBudgetExhausted,
    CodexBudgetExhausted,
    BuildBudgetExhausted,
    NoImprovementPlateau,
    RuntimeChanged,
    SourceChanged,
    UserPaused,
    UserCancelled,
    BackendError,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitRect {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

impl FitRect {
    pub(crate) fn validate(self, label: &str) -> Result<()> {
        if self.right <= self.left || self.bottom <= self.top {
            bail!("{label} 必须是非空矩形");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitTargetPair {
    pub(crate) target_design_id: String,
    pub(crate) target_sha256: String,
    pub(crate) target_rect: FitRect,
    pub(crate) runtime_node_id: String,
    pub(crate) definition_id: String,
    #[serde(default)]
    pub(crate) component_kind: Option<String>,
    #[serde(default)]
    pub(crate) parent_layout_kind: Option<String>,
    pub(crate) instance_key: Option<String>,
    pub(crate) current_rect: FitRect,
    pub(crate) projected_target_rect: FitRect,
    pub(crate) calibration_id: Option<String>,
    pub(crate) confidence: Option<f64>,
}

impl FitTargetPair {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.target_design_id.trim().is_empty()
            || self.target_sha256.trim().is_empty()
            || self.runtime_node_id.trim().is_empty()
            || self.definition_id.trim().is_empty()
        {
            bail!("设计稿、目标节点和 definitionId 不能为空");
        }
        self.target_rect.validate("targetRect")?;
        self.current_rect.validate("currentRect")?;
        self.projected_target_rect.validate("projectedTargetRect")?;
        if self
            .confidence
            .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            bail!("confidence 必须在 0..1");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitInsets {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitEnvironment {
    pub(crate) screen_id: Option<String>,
    pub(crate) scenario: Option<String>,
    pub(crate) theme: Option<String>,
    pub(crate) locale: Option<String>,
    pub(crate) viewport_width: Option<u32>,
    pub(crate) viewport_height: Option<u32>,
    pub(crate) density: Option<f32>,
    pub(crate) font_scale: Option<f32>,
    pub(crate) rotation: Option<i32>,
    pub(crate) insets: Option<FitInsets>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum FitHandoffStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitCodexHandoff {
    pub(crate) handoff_id: String,
    pub(crate) run_id: String,
    pub(crate) reason: String,
    pub(crate) status: FitHandoffStatus,
    pub(crate) created_at: String,
    pub(crate) task_id: Option<String>,
    pub(crate) artifact_path: Option<String>,
    pub(crate) source_revision_before: Option<String>,
    pub(crate) source_revision_after: Option<String>,
    #[serde(default)]
    pub(crate) changed_files: Vec<String>,
    pub(crate) commit_id: Option<String>,
    pub(crate) error: Option<String>,
}

pub(crate) fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':'))
    {
        bail!("{label} 非法");
    }
    Ok(())
}
