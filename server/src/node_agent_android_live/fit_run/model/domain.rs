use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
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
    #[serde(default)]
    pub(crate) state_replay: Option<FitStateReplay>,
}

impl FitEnvironment {
    pub(crate) fn validated_state_replay(&self) -> Result<Option<FitStateReplay>> {
        let scenario = self
            .scenario
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let replay = self.state_replay.as_ref();
        if scenario_requires_state_replay(scenario) && replay.is_none() {
            bail!(
                "FIT_STATE_REPLAY_MISSING: 非根页面 scenario={} 缺少持久化 stateReplay trace",
                scenario.unwrap_or("unknown")
            );
        }
        let Some(replay) = replay else {
            return Ok(None);
        };
        replay.validate(scenario)?;
        Ok(Some(replay.clone()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitStateReplay {
    #[serde(default = "state_replay_schema_version")]
    pub(crate) schema_version: u32,
    pub(crate) scenario_id: String,
    pub(crate) captured_at: String,
    pub(crate) expires_at: String,
    pub(crate) steps: Vec<FitStateReplayStep>,
}

impl FitStateReplay {
    fn validate(&self, environment_scenario: Option<&str>) -> Result<()> {
        if self.schema_version != state_replay_schema_version() {
            bail!(
                "FIT_STATE_REPLAY_SCHEMA_UNSUPPORTED: schemaVersion={}",
                self.schema_version
            );
        }
        validate_identifier(&self.scenario_id, "stateReplay.scenarioId")?;
        if environment_scenario != Some(self.scenario_id.as_str()) {
            bail!(
                "FIT_STATE_REPLAY_SCENARIO_MISMATCH: environment scenario 与 stateReplay.scenarioId 不一致"
            );
        }
        if self.steps.is_empty() || self.steps.len() > 16 {
            bail!("FIT_STATE_REPLAY_INVALID: steps 数量必须为 1..16");
        }
        let captured_at = parse_replay_time(&self.captured_at, "capturedAt")?;
        let expires_at = parse_replay_time(&self.expires_at, "expiresAt")?;
        if expires_at <= captured_at {
            bail!("FIT_STATE_REPLAY_INVALID: expiresAt 必须晚于 capturedAt");
        }
        if Utc::now() > expires_at {
            bail!(
                "FIT_STATE_REPLAY_EXPIRED: scenario={} expiresAt={}",
                self.scenario_id,
                self.expires_at
            );
        }
        for step in &self.steps {
            step.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitStateReplayStep {
    pub(crate) name: String,
    pub(crate) action: FitStateReplayAction,
}

impl FitStateReplayStep {
    fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() || self.name.chars().count() > 80 {
            bail!("FIT_STATE_REPLAY_INVALID: step.name 必须为 1..80 字");
        }
        match &self.action {
            FitStateReplayAction::ActivateNode {
                definition_id,
                instance_key,
                occurrence,
            } => {
                if definition_id.trim().is_empty() || definition_id.chars().count() > 500 {
                    bail!("FIT_STATE_REPLAY_INVALID: ACTIVATE_NODE definitionId 必须为 1..500 字");
                }
                if instance_key
                    .as_deref()
                    .is_some_and(|value| value.chars().count() > 500)
                    || *occurrence > 50
                {
                    bail!("FIT_STATE_REPLAY_INVALID: ACTIVATE_NODE instanceKey/occurrence 超限");
                }
            }
            FitStateReplayAction::Back => {}
            FitStateReplayAction::Wait { duration_ms } => {
                if !(100..=5_000).contains(duration_ms) {
                    bail!("FIT_STATE_REPLAY_INVALID: WAIT durationMs 必须为 100..5000");
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum FitStateReplayAction {
    ActivateNode {
        #[serde(rename = "definitionId")]
        definition_id: String,
        #[serde(rename = "instanceKey")]
        instance_key: Option<String>,
        #[serde(default)]
        occurrence: usize,
    },
    Back,
    Wait {
        #[serde(rename = "durationMs")]
        duration_ms: u64,
    },
}

fn scenario_requires_state_replay(scenario: Option<&str>) -> bool {
    let Some(scenario) = scenario else {
        return false;
    };
    !matches!(
        scenario.trim().to_ascii_uppercase().as_str(),
        "" | "HOME"
            | "HOME_PAGE"
            | "ROOT"
            | "ROOT_PAGE"
            | "DEFAULT"
            | "NORMAL"
            | "LOADING"
            | "EMPTY"
            | "ERROR"
            | "LAUNCH"
    )
}

fn parse_replay_time(value: &str, field: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("FIT_STATE_REPLAY_INVALID: {field} 不是 RFC3339 时间"))
        .map(|value| value.with_timezone(&Utc))
}

const fn state_replay_schema_version() -> u32 {
    1
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
