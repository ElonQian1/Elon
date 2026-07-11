use std::future::Future;
use std::pin::Pin;

use anyhow::{bail, Result};
use chrono::Utc;

use super::handoff::new_codex_handoff;
use super::model::{
    FitCandidate, FitRunDocument, FitRunPhase, FitStopReason, FitTrial, FitTrialKind,
};

pub(crate) type FitRunBackendFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

#[derive(Debug, Clone)]
pub(crate) struct FitBackendResult {
    pub(crate) candidate: FitCandidate,
    pub(crate) evaluations: u32,
    pub(crate) duration_ms: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct FitSourceVerifyResult {
    pub(crate) candidate: FitCandidate,
    pub(crate) duration_ms: u64,
}

pub(crate) trait FitRunBackend: Send + Sync {
    fn capture_baseline<'a>(
        &'a self,
        run: FitRunDocument,
    ) -> FitRunBackendFuture<'a, FitBackendResult>;

    fn solve_local<'a>(&'a self, run: FitRunDocument) -> FitRunBackendFuture<'a, FitBackendResult>;

    fn evaluate_after_codex<'a>(
        &'a self,
        run: FitRunDocument,
    ) -> FitRunBackendFuture<'a, FitBackendResult>;

    fn verify_source<'a>(
        &'a self,
        run: FitRunDocument,
    ) -> FitRunBackendFuture<'a, FitSourceVerifyResult>;

    fn reapply_best<'a>(&'a self, run: FitRunDocument) -> FitRunBackendFuture<'a, ()>;

    fn revert_best<'a>(&'a self, run: FitRunDocument) -> FitRunBackendFuture<'a, ()>;
}

#[derive(Debug)]
pub(crate) struct OrchestratorStep {
    pub(crate) boundary: bool,
    pub(crate) trial: Option<FitTrial>,
    pub(crate) handoff_created: bool,
}

impl OrchestratorStep {
    fn continue_with(trial: Option<FitTrial>) -> Self {
        Self {
            boundary: false,
            trial,
            handoff_created: false,
        }
    }

    fn boundary(trial: Option<FitTrial>, handoff_created: bool) -> Self {
        Self {
            boundary: true,
            trial,
            handoff_created,
        }
    }
}

pub(crate) async fn advance_one(
    run: &mut FitRunDocument,
    backend: &dyn FitRunBackend,
) -> Result<OrchestratorStep> {
    run.refresh_elapsed();
    let budget_stop = match run.phase {
        FitRunPhase::Created
        | FitRunPhase::Baselining
        | FitRunPhase::LocalSolving
        | FitRunPhase::Evaluating => run.budget_stop(),
        FitRunPhase::Rebuilding | FitRunPhase::SourceVerifying
            if run.usage.elapsed_ms >= run.budget.max_duration_ms =>
        {
            Some(FitStopReason::DurationBudgetExhausted)
        }
        _ => None,
    };
    if let Some(reason) = budget_stop {
        return stop_for_budget(run, reason);
    }
    match run.phase {
        FitRunPhase::Created => {
            run.transition(FitRunPhase::Baselining)?;
            Ok(OrchestratorStep::continue_with(None))
        }
        FitRunPhase::Baselining => {
            let result = backend.capture_baseline(run.clone()).await?;
            run.usage.local_evaluations = run
                .usage
                .local_evaluations
                .saturating_add(result.evaluations);
            run.usage.elapsed_ms = run.usage.elapsed_ms.saturating_add(result.duration_ms);
            run.baseline = Some(result.candidate.clone());
            let improved = run.consider_candidate(result.candidate.clone());
            run.transition(FitRunPhase::LocalSolving)?;
            let trial = trial(run, FitTrialKind::Baseline, result, improved);
            Ok(OrchestratorStep::continue_with(Some(trial)))
        }
        FitRunPhase::LocalSolving => {
            if run.target_reached() {
                run.stop_reason = Some(FitStopReason::TargetReached);
                run.transition(FitRunPhase::CandidateReady)?;
                return Ok(OrchestratorStep::boundary(None, false));
            }
            if run.properties.is_empty() {
                return move_to_codex_or_plateau(run, None);
            }
            let result = backend.solve_local(run.clone()).await?;
            run.usage.local_evaluations = run
                .usage
                .local_evaluations
                .saturating_add(result.evaluations);
            run.usage.elapsed_ms = run.usage.elapsed_ms.saturating_add(result.duration_ms);
            let improved = run.consider_candidate(result.candidate.clone());
            let trial = trial(run, FitTrialKind::LiveApply, result, improved);
            if run.target_reached() {
                run.stop_reason = Some(FitStopReason::TargetReached);
                run.transition(FitRunPhase::CandidateReady)?;
                Ok(OrchestratorStep::boundary(Some(trial), false))
            } else if local_plateau(run) {
                move_to_codex_or_plateau(run, Some(trial))
            } else {
                Ok(OrchestratorStep::continue_with(Some(trial)))
            }
        }
        FitRunPhase::AwaitingCodex
        | FitRunPhase::CodexRunning
        | FitRunPhase::CandidateReady
        | FitRunPhase::Paused
        | FitRunPhase::Accepted
        | FitRunPhase::Plateau
        | FitRunPhase::Failed
        | FitRunPhase::Cancelled => Ok(OrchestratorStep::boundary(None, false)),
        FitRunPhase::Rebuilding => {
            if !run.build_available() {
                run.stop_reason = Some(FitStopReason::BuildBudgetExhausted);
                run.transition(FitRunPhase::Failed)?;
                return Ok(OrchestratorStep::boundary(None, false));
            }
            run.usage.build_rounds = run.usage.build_rounds.saturating_add(1);
            let result = backend.evaluate_after_codex(run.clone()).await?;
            run.usage.elapsed_ms = run.usage.elapsed_ms.saturating_add(result.duration_ms);
            run.transition(FitRunPhase::Evaluating)?;
            if result.candidate.runtime_build_id.is_some() {
                run.runtime_build_id = result.candidate.runtime_build_id.clone();
            }
            let improved = run.consider_candidate(result.candidate.clone());
            let trial = trial(run, FitTrialKind::BuildVerify, result, improved);
            Ok(OrchestratorStep::continue_with(Some(trial)))
        }
        FitRunPhase::Evaluating => {
            if run.target_reached() {
                run.stop_reason = Some(FitStopReason::TargetReached);
                run.transition(FitRunPhase::CandidateReady)?;
                Ok(OrchestratorStep::boundary(None, false))
            } else if local_plateau(run) {
                move_to_codex_or_plateau(run, None)
            } else {
                run.transition(FitRunPhase::LocalSolving)?;
                Ok(OrchestratorStep::continue_with(None))
            }
        }
        FitRunPhase::SourceVerifying => {
            if !run.build_available() {
                run.stop_reason = Some(FitStopReason::BuildBudgetExhausted);
                run.transition(FitRunPhase::Failed)?;
                return Ok(OrchestratorStep::boundary(None, false));
            }
            run.usage.build_rounds = run.usage.build_rounds.saturating_add(1);
            let result = backend.verify_source(run.clone()).await?;
            run.usage.elapsed_ms = run.usage.elapsed_ms.saturating_add(result.duration_ms);
            let candidate = result.candidate;
            if candidate.runtime_build_id.is_some() {
                run.runtime_build_id = candidate.runtime_build_id.clone();
            }
            if candidate.source_revision.is_some() {
                run.source_revision = candidate.source_revision.clone();
            }
            let improved = run.consider_candidate(candidate.clone());
            let backend_result = FitBackendResult {
                candidate,
                evaluations: 0,
                duration_ms: result.duration_ms,
            };
            if run.source_verified() {
                run.stop_reason = Some(FitStopReason::SourceVerified);
                run.transition(FitRunPhase::Accepted)?;
                let trial = trial(run, FitTrialKind::SourceVerify, backend_result, improved);
                Ok(OrchestratorStep::boundary(Some(trial), false))
            } else if run.codex_available() {
                run.transition(FitRunPhase::AwaitingCodex)?;
                run.handoff = Some(new_codex_handoff(
                    run,
                    "源码构建结果尚未同时满足目标图和 Source Parity",
                ));
                let trial = trial(run, FitTrialKind::SourceVerify, backend_result, improved);
                Ok(OrchestratorStep::boundary(Some(trial), true))
            } else {
                run.stop_reason = Some(FitStopReason::CodexBudgetExhausted);
                run.transition(FitRunPhase::Failed)?;
                let trial = trial(run, FitTrialKind::SourceVerify, backend_result, improved);
                Ok(OrchestratorStep::boundary(Some(trial), false))
            }
        }
    }
}

fn local_plateau(run: &FitRunDocument) -> bool {
    run.usage.no_improvement_trials >= run.thresholds.plateau_window
        || run.usage.no_improvement_trials >= run.budget.max_no_improvement_trials
        || run.usage.local_evaluations >= run.budget.max_local_evaluations
}

fn move_to_codex_or_plateau(
    run: &mut FitRunDocument,
    trial: Option<FitTrial>,
) -> Result<OrchestratorStep> {
    if run.codex_available() {
        run.transition(FitRunPhase::AwaitingCodex)?;
        run.handoff = Some(new_codex_handoff(
            run,
            "本地 LIVE 数值求解进入平台期，需要检查布局结构或源码 Binding",
        ));
        Ok(OrchestratorStep::boundary(trial, true))
    } else {
        run.stop_reason = Some(FitStopReason::CodexBudgetExhausted);
        run.transition(FitRunPhase::Plateau)?;
        Ok(OrchestratorStep::boundary(trial, false))
    }
}

fn stop_for_budget(run: &mut FitRunDocument, reason: FitStopReason) -> Result<OrchestratorStep> {
    if matches!(
        run.phase,
        FitRunPhase::LocalSolving | FitRunPhase::Evaluating
    ) && run.codex_available()
    {
        run.stop_reason = Some(reason);
        run.transition(FitRunPhase::AwaitingCodex)?;
        run.handoff = Some(new_codex_handoff(run, format!("本地预算停止: {reason:?}")));
        Ok(OrchestratorStep::boundary(None, true))
    } else if matches!(
        run.phase,
        FitRunPhase::LocalSolving | FitRunPhase::Evaluating
    ) {
        run.stop_reason = Some(reason);
        run.transition(FitRunPhase::Plateau)?;
        Ok(OrchestratorStep::boundary(None, false))
    } else {
        run.pause(
            reason,
            Some("FitRun 预算已耗尽，可调整预算后恢复".to_string()),
        )?;
        Ok(OrchestratorStep::boundary(None, false))
    }
}

fn trial(
    run: &mut FitRunDocument,
    kind: FitTrialKind,
    result: FitBackendResult,
    accepted_as_best: bool,
) -> FitTrial {
    FitTrial {
        sequence: run.next_sequence(),
        trial_id: result.candidate.trial_id.clone(),
        kind,
        created_at: Utc::now().to_rfc3339(),
        duration_ms: result.duration_ms,
        evaluations: result.evaluations,
        candidate: Some(result.candidate),
        accepted_as_best,
        error: None,
        checkpoint: run.checkpoint(),
    }
}

pub(crate) async fn restore_best(run: &FitRunDocument, backend: &dyn FitRunBackend) -> Result<()> {
    if run.best.is_none() {
        bail!("FitRun 尚无可恢复的最佳结果");
    }
    backend.reapply_best(run.clone()).await
}
