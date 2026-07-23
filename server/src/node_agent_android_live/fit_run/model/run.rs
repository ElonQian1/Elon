use anyhow::{bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::transitions::legal_transition;
use super::{
    validate_identifier, FitBudget, FitBudgetUsage, FitCandidate, FitCodexHandoff, FitEnvironment,
    FitRunAuditEvent, FitRunPhase, FitSessionContext, FitStopReason, FitTargetPair, FitThresholds,
    FitTrialCheckpoint, FitVisualMask,
};

const FIT_RUN_SCHEMA_VERSION: u32 = 1;
const MAX_PROCESSED_COMMANDS: usize = 128;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateFitRunRequest {
    #[serde(default)]
    pub(crate) task_id: Option<String>,
    pub(crate) pair: FitTargetPair,
    #[serde(default)]
    pub(crate) environment: FitEnvironment,
    #[serde(default)]
    pub(crate) properties: Vec<String>,
    #[serde(default)]
    pub(crate) budget: FitBudget,
    #[serde(default)]
    pub(crate) thresholds: FitThresholds,
    #[serde(default)]
    pub(crate) visual_mask: FitVisualMask,
    #[serde(default)]
    pub(crate) auto_start: bool,
}

impl CreateFitRunRequest {
    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(task_id) = self.task_id.as_deref() {
            validate_identifier(task_id, "taskId")?;
        }
        self.pair.validate()?;
        self.budget.validate()?;
        self.thresholds.validate()?;
        self.visual_mask.validate(self.pair.target_rect)?;
        self.environment.validated_state_replay()?;
        if self.properties.len() > 64
            || self
                .properties
                .iter()
                .any(|value| value.trim().is_empty() || value.len() > 120)
        {
            bail!("properties 数量或长度超限");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitRunDocument {
    pub(crate) schema_version: u32,
    pub(crate) run_id: String,
    #[serde(default)]
    pub(crate) task_id: Option<String>,
    pub(crate) session_id: String,
    pub(crate) project_root: String,
    pub(crate) package_name: String,
    pub(crate) device_id: String,
    pub(crate) phase: FitRunPhase,
    pub(crate) stop_reason: Option<FitStopReason>,
    pub(crate) pair: FitTargetPair,
    pub(crate) environment: FitEnvironment,
    pub(crate) properties: Vec<String>,
    pub(crate) budget: FitBudget,
    pub(crate) usage: FitBudgetUsage,
    pub(crate) thresholds: FitThresholds,
    #[serde(default)]
    pub(crate) visual_mask: FitVisualMask,
    pub(crate) baseline: Option<FitCandidate>,
    pub(crate) current: Option<FitCandidate>,
    pub(crate) best: Option<FitCandidate>,
    pub(crate) handoff: Option<FitCodexHandoff>,
    pub(crate) resume_phase: Option<FitRunPhase>,
    pub(crate) runtime_build_id: Option<String>,
    pub(crate) tree_revision: u64,
    pub(crate) source_revision: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) last_sequence: u64,
    pub(crate) last_error: Option<String>,
    #[serde(default)]
    pub(crate) processed_commands: Vec<String>,
    #[serde(default)]
    pub(crate) audit_events: Vec<FitRunAuditEvent>,
}

impl FitRunDocument {
    pub(crate) fn new(context: FitSessionContext, request: CreateFitRunRequest) -> Result<Self> {
        request.validate()?;
        if context.project_root.trim().is_empty() {
            bail!("FitRun 需要项目目录，临时 Live Session 不可持久化");
        }
        let now = Utc::now().to_rfc3339();
        Ok(Self {
            schema_version: FIT_RUN_SCHEMA_VERSION,
            run_id: format!("fit_{}", uuid::Uuid::new_v4().simple()),
            task_id: request.task_id,
            session_id: context.session_id,
            project_root: context.project_root,
            package_name: context.package_name,
            device_id: context.device_id,
            phase: FitRunPhase::Created,
            stop_reason: None,
            pair: request.pair,
            environment: request.environment,
            properties: request.properties,
            budget: request.budget,
            usage: FitBudgetUsage::default(),
            thresholds: request.thresholds,
            visual_mask: request.visual_mask,
            baseline: None,
            current: None,
            best: None,
            handoff: None,
            resume_phase: None,
            runtime_build_id: context.runtime_build_id,
            tree_revision: context.tree_revision,
            source_revision: context.source_revision,
            created_at: now.clone(),
            updated_at: now,
            last_sequence: 0,
            last_error: None,
            processed_commands: Vec::new(),
            audit_events: Vec::new(),
        })
    }

    pub(crate) fn validate_loaded(&self) -> Result<()> {
        if self.schema_version != FIT_RUN_SCHEMA_VERSION {
            bail!("不支持的 FitRun schemaVersion: {}", self.schema_version);
        }
        validate_identifier(&self.run_id, "runId")?;
        if let Some(task_id) = self.task_id.as_deref() {
            validate_identifier(task_id, "taskId")?;
        }
        self.pair.validate()?;
        self.budget.validate()?;
        self.thresholds.validate()?;
        self.visual_mask.validate(self.pair.target_rect)?;
        Ok(())
    }

    pub(crate) fn reconcile_score_thresholds(&mut self) {
        for candidate in [&mut self.baseline, &mut self.current, &mut self.best]
            .into_iter()
            .flatten()
        {
            candidate
                .score
                .reconcile_threshold_failures(&self.thresholds);
        }
    }

    pub(crate) fn transition(&mut self, next: FitRunPhase) -> Result<()> {
        if self.phase == next {
            return Ok(());
        }
        if !legal_transition(self.phase, next) {
            bail!("非法 FitRun 状态转换: {:?} -> {:?}", self.phase, next);
        }
        self.phase = next;
        self.touch();
        Ok(())
    }

    pub(crate) fn pause(&mut self, reason: FitStopReason, error: Option<String>) -> Result<()> {
        if self.phase.is_terminal() {
            bail!("终态 FitRun 不可暂停");
        }
        if self.phase != FitRunPhase::Paused {
            self.resume_phase = Some(self.phase);
            self.transition(FitRunPhase::Paused)?;
        }
        self.stop_reason = Some(reason);
        self.last_error = error;
        self.touch();
        Ok(())
    }

    pub(crate) fn resume(&mut self) -> Result<()> {
        if self.phase != FitRunPhase::Paused {
            bail!("只有暂停中的 FitRun 可以恢复");
        }
        let next = self.resume_phase.take().unwrap_or(FitRunPhase::Baselining);
        if next.is_terminal() || next == FitRunPhase::Paused {
            bail!("FitRun 恢复目标状态非法");
        }
        self.phase = next;
        self.stop_reason = None;
        self.last_error = None;
        self.touch();
        Ok(())
    }

    pub(crate) fn consider_candidate(&mut self, candidate: FitCandidate) -> bool {
        let improved = self.best.as_ref().map_or(true, |best| {
            candidate
                .score
                .better_than(&best.score, self.thresholds.min_meaningful_improvement)
        });
        self.current = Some(candidate.clone());
        if improved {
            self.best = Some(candidate);
            self.usage.no_improvement_trials = 0;
        } else {
            self.usage.no_improvement_trials = self.usage.no_improvement_trials.saturating_add(1);
        }
        self.touch();
        improved
    }

    pub(crate) fn target_reached(&self) -> bool {
        self.best
            .as_ref()
            .is_some_and(|candidate| candidate.score.passes(&self.thresholds))
    }

    pub(crate) fn source_verified(&self) -> bool {
        self.current.as_ref().is_some_and(|candidate| {
            candidate.score.passes(&self.thresholds)
                && candidate.source_parity_verified
                && candidate
                    .source_parity_loss
                    .is_some_and(|loss| loss <= self.thresholds.max_source_parity_loss)
        })
    }

    pub(crate) fn budget_stop(&self) -> Option<FitStopReason> {
        if self.usage.elapsed_ms >= self.budget.max_duration_ms {
            Some(FitStopReason::DurationBudgetExhausted)
        } else if self.usage.local_evaluations >= self.budget.max_local_evaluations {
            Some(FitStopReason::LocalEvaluationBudgetExhausted)
        } else if self.usage.no_improvement_trials >= self.budget.max_no_improvement_trials {
            Some(FitStopReason::NoImprovementPlateau)
        } else {
            None
        }
    }

    pub(crate) fn codex_available(&self) -> bool {
        self.usage.codex_rounds < self.budget.max_codex_rounds
    }

    pub(crate) fn build_available(&self) -> bool {
        self.usage.build_rounds < self.budget.max_build_rounds
    }

    pub(crate) fn record_command(&mut self, command_id: String) {
        self.processed_commands.push(command_id);
        if self.processed_commands.len() > MAX_PROCESSED_COMMANDS {
            let overflow = self.processed_commands.len() - MAX_PROCESSED_COMMANDS;
            self.processed_commands.drain(0..overflow);
        }
        self.touch();
    }

    pub(crate) fn has_command(&self, command_id: &str) -> bool {
        self.processed_commands
            .iter()
            .any(|value| value == command_id)
    }

    pub(crate) fn next_sequence(&mut self) -> u64 {
        self.last_sequence = self.last_sequence.saturating_add(1);
        self.last_sequence
    }

    pub(crate) fn refresh_elapsed(&mut self) {
        // elapsedMs 只累计本地求解、构建和验收的主动计算时间。
        // 等待用户或 Codex 的墙钟时间不能消耗拟合预算。
        self.touch();
    }

    pub(crate) fn checkpoint(&self) -> FitTrialCheckpoint {
        FitTrialCheckpoint {
            phase: self.phase,
            stop_reason: self.stop_reason,
            usage: self.usage.clone(),
            current: self.current.clone(),
            best: self.best.clone(),
        }
    }

    pub(crate) fn apply_checkpoint(&mut self, checkpoint: FitTrialCheckpoint, sequence: u64) {
        self.phase = checkpoint.phase;
        self.stop_reason = checkpoint.stop_reason;
        self.usage = checkpoint.usage;
        self.current = checkpoint.current;
        self.best = checkpoint.best;
        self.last_sequence = sequence;
        self.touch();
    }

    pub(crate) fn touch(&mut self) {
        self.updated_at = Utc::now().to_rfc3339();
    }
}
