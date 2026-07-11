use anyhow::{anyhow, bail, Result};

use super::super::handoff::new_codex_handoff;
use super::super::model::{
    FitCommand, FitHandoffStatus, FitRunDocument, FitRunPhase, FitSessionContext, FitStopReason,
};
use super::FitRunService;

pub(super) async fn apply_command(
    service: &FitRunService,
    context: &FitSessionContext,
    run: &mut FitRunDocument,
    command: FitCommand,
) -> Result<bool> {
    match command {
        FitCommand::Start { .. } => {
            if run.phase != FitRunPhase::Created {
                bail!("只有新建 FitRun 可以 START");
            }
            Ok(true)
        }
        FitCommand::Pause { .. } => {
            run.pause(FitStopReason::UserPaused, None)?;
            Ok(false)
        }
        FitCommand::Resume { .. } => {
            run.resume()?;
            Ok(true)
        }
        FitCommand::Cancel { .. } => {
            if run.phase.is_terminal() {
                bail!("终态 FitRun 不可取消");
            }
            if run
                .best
                .as_ref()
                .is_some_and(|candidate| !candidate.operations.is_empty())
            {
                if let Err(error) = service.backend.revert_best(run.clone()).await {
                    run.pause(
                        FitStopReason::BackendError,
                        Some(format!(
                            "取消 FitRun 时无法安全撤销本任务的 Live Patch: {error:#}"
                        )),
                    )?;
                    return Ok(false);
                }
            }
            run.stop_reason = Some(FitStopReason::UserCancelled);
            run.transition(FitRunPhase::Cancelled)?;
            Ok(false)
        }
        FitCommand::RebindSession {
            new_session_id,
            new_runtime_node_id,
            new_current_rect,
            ..
        } => {
            if run.phase.is_terminal() {
                bail!("终态 FitRun 不可重新绑定或重放样式");
            }
            if new_session_id != context.session_id {
                bail!("newSessionId 必须与当前 Live Session 一致");
            }
            let source_changed = run.source_revision != context.source_revision;
            run.session_id = context.session_id.clone();
            if let Some(runtime_node_id) = new_runtime_node_id {
                run.pair.runtime_node_id = runtime_node_id;
            }
            if let Some(current_rect) = new_current_rect {
                run.pair.current_rect = current_rect;
            }
            run.device_id = context.device_id.clone();
            run.runtime_build_id = context.runtime_build_id.clone();
            run.tree_revision = context.tree_revision;
            if source_changed {
                run.pause(
                    FitStopReason::SourceChanged,
                    Some("源码 Revision 已变化；已保留历史，但不会重放旧 Patch".to_string()),
                )?;
            } else if run.best.is_some() {
                super::super::orchestrator::restore_best(run, service.backend.as_ref()).await?;
            }
            run.touch();
            Ok(false)
        }
        FitCommand::AcceptBest { .. } => {
            if run.phase != FitRunPhase::CandidateReady {
                bail!("只有 CANDIDATE_READY 可以确认写回");
            }
            run.transition(FitRunPhase::SourceVerifying)?;
            Ok(true)
        }
        FitCommand::CodexStarted {
            handoff_id,
            task_id,
            ..
        } => {
            if run.phase != FitRunPhase::AwaitingCodex {
                bail!("当前 FitRun 未等待 Codex");
            }
            let handoff = matching_handoff(run, &handoff_id)?;
            handoff.status = FitHandoffStatus::Running;
            handoff.task_id = Some(task_id);
            run.usage.codex_rounds = run.usage.codex_rounds.saturating_add(1);
            run.transition(FitRunPhase::CodexRunning)?;
            service.persist_handoff(run)?;
            Ok(false)
        }
        FitCommand::CodexCompleted {
            handoff_id,
            task_id,
            source_revision_before,
            source_revision_after,
            changed_files,
            commit_id,
            token_usage,
            ..
        } => {
            if run.phase != FitRunPhase::CodexRunning {
                bail!("当前 FitRun 没有运行中的 Codex 交接");
            }
            let current_revision = context
                .source_revision
                .clone()
                .ok_or_else(|| anyhow!("无法读取 Codex 完成后的源码 Revision"))?;
            if !source_revision_after.trim().is_empty() && source_revision_after != current_revision
            {
                bail!("Codex 回报的 sourceRevisionAfter 与当前工作区不一致");
            }
            let source_revision_after = current_revision;
            let handoff = matching_handoff(run, &handoff_id)?;
            handoff.status = FitHandoffStatus::Completed;
            if task_id.is_some() {
                handoff.task_id = task_id;
            }
            handoff.source_revision_before = source_revision_before;
            handoff.source_revision_after = Some(source_revision_after.clone());
            handoff.changed_files = changed_files;
            handoff.commit_id = commit_id;
            run.source_revision = Some(source_revision_after);
            run.usage.codex_tokens = match (run.usage.codex_tokens, token_usage) {
                (Some(left), Some(right)) => Some(left.saturating_add(right)),
                (None, Some(value)) | (Some(value), None) => Some(value),
                (None, None) => None,
            };
            run.transition(FitRunPhase::Rebuilding)?;
            service.persist_handoff(run)?;
            Ok(true)
        }
        FitCommand::CodexFailed {
            handoff_id, error, ..
        } => {
            if !matches!(
                run.phase,
                FitRunPhase::AwaitingCodex | FitRunPhase::CodexRunning
            ) {
                bail!("当前 FitRun 没有可失败的 Codex 交接");
            }
            let handoff = matching_handoff(run, &handoff_id)?;
            handoff.status = FitHandoffStatus::Failed;
            handoff.error = Some(error.clone());
            service.persist_handoff(run)?;
            if run.codex_available() {
                if run.phase == FitRunPhase::CodexRunning {
                    run.transition(FitRunPhase::AwaitingCodex)?;
                }
                run.handoff = Some(new_codex_handoff(
                    run,
                    format!("上一次 Codex 任务失败，需要重试: {error}"),
                ));
                service.persist_handoff(run)?;
            } else {
                run.stop_reason = Some(FitStopReason::CodexBudgetExhausted);
                run.transition(FitRunPhase::Plateau)?;
            }
            Ok(false)
        }
    }
}

fn matching_handoff<'a>(
    run: &'a mut FitRunDocument,
    handoff_id: &str,
) -> Result<&'a mut super::super::model::FitCodexHandoff> {
    run.handoff
        .as_mut()
        .filter(|value| value.handoff_id == handoff_id)
        .ok_or_else(|| anyhow!("handoffId 与当前 FitRun 不一致"))
}
