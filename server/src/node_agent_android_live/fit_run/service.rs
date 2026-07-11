use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use tokio::sync::{Mutex, RwLock};

use super::handoff::{handoff_payload, new_codex_handoff};
use super::model::{
    CreateFitRunRequest, FitCommand, FitCommandResult, FitHandoffStatus, FitRunDocument,
    FitRunPhase, FitSessionContext, FitStopReason,
};
use super::orchestrator::{advance_one, restore_best, FitRunBackend};
use super::store::FitRunStore;
use crate::node_agent_android_live::fit_learning::{record_and_promote, FitUserDecision};

const MAX_INTERNAL_STEPS: usize = 32;

pub(crate) struct FitRunService {
    store: FitRunStore,
    backend: Arc<dyn FitRunBackend>,
    run_locks: RwLock<HashMap<String, Arc<Mutex<()>>>>,
}

impl FitRunService {
    pub(crate) fn new(store: FitRunStore, backend: Arc<dyn FitRunBackend>) -> Self {
        Self {
            store,
            backend,
            run_locks: RwLock::new(HashMap::new()),
        }
    }

    pub(crate) async fn create_run(
        &self,
        context: FitSessionContext,
        request: CreateFitRunRequest,
    ) -> Result<FitRunDocument> {
        let auto_start = request.auto_start;
        let run = FitRunDocument::new(context.clone(), request)?;
        self.store.save(&run)?;
        if !auto_start {
            return Ok(run);
        }
        let command = FitCommand::Start {
            command_id: format!("auto_start: {}", run.run_id).replace(' ', ""),
        };
        Ok(self.command(context, &run.run_id, command).await?.run)
    }

    pub(crate) fn get_run(
        &self,
        context: &FitSessionContext,
        run_id: &str,
    ) -> Result<FitRunDocument> {
        let run = self.store.load(&context.project_root, run_id)?;
        validate_project_context(&run, context)?;
        Ok(run)
    }

    pub(crate) fn list_runs(&self, context: &FitSessionContext) -> Result<Vec<FitRunDocument>> {
        Ok(self
            .store
            .list_for_project(&context.project_root)?
            .into_iter()
            .filter(|run| run.package_name == context.package_name)
            .collect())
    }

    pub(crate) async fn command(
        &self,
        context: FitSessionContext,
        run_id: &str,
        command: FitCommand,
    ) -> Result<FitCommandResult> {
        command.validate()?;
        let lock = self.run_lock(run_id).await;
        let _guard = lock.lock().await;
        let mut run = self.store.load(&context.project_root, run_id)?;
        validate_command_context(&run, &context, &command)?;
        if run.has_command(command.command_id()) {
            self.record_terminal_learning(&run);
            return Ok(FitCommandResult {
                run,
                idempotent: true,
            });
        }
        let command_id = command.command_id().to_string();
        let should_drive = self.apply_command(&context, &mut run, command).await?;
        run.record_command(command_id);
        self.store.save(&run)?;
        if should_drive {
            run = self.drive(run).await?;
        }
        self.record_terminal_learning(&run);
        Ok(FitCommandResult {
            run,
            idempotent: false,
        })
    }

    async fn apply_command(
        &self,
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
                    if let Err(error) = self.backend.revert_best(run.clone()).await {
                        run.pause(
                            FitStopReason::BackendError,
                            Some(format!("取消 FitRun 时无法安全撤销本任务的 Live Patch: {error:#}")),
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
                    restore_best(run, self.backend.as_ref()).await?;
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
                self.persist_handoff(run)?;
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
                if !source_revision_after.trim().is_empty()
                    && source_revision_after != current_revision
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
                self.persist_handoff(run)?;
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
                self.persist_handoff(run)?;
                if run.codex_available() {
                    if run.phase == FitRunPhase::CodexRunning {
                        run.transition(FitRunPhase::AwaitingCodex)?;
                    }
                    run.handoff = Some(new_codex_handoff(
                        run,
                        format!("上一次 Codex 任务失败，需要重试: {error}"),
                    ));
                    self.persist_handoff(run)?;
                } else {
                    run.stop_reason = Some(FitStopReason::CodexBudgetExhausted);
                    run.transition(FitRunPhase::Plateau)?;
                }
                Ok(false)
            }
        }
    }

    async fn drive(&self, mut run: FitRunDocument) -> Result<FitRunDocument> {
        for _ in 0..MAX_INTERNAL_STEPS {
            let step = match advance_one(&mut run, self.backend.as_ref()).await {
                Ok(step) => step,
                Err(error) => {
                    run.pause(FitStopReason::BackendError, Some(format!("{error:#}")))?;
                    self.store.save(&run)?;
                    return Ok(run);
                }
            };
            if let Some(trial) = &step.trial {
                self.store.append_trial(&run, trial)?;
            }
            if step.handoff_created {
                self.persist_handoff(&mut run)?;
            }
            self.store.save(&run)?;
            if step.boundary {
                return Ok(run);
            }
        }
        run.pause(
            FitStopReason::BackendError,
            Some("FitRun 单次推进超过内部步数保护，已暂停".to_string()),
        )?;
        self.store.save(&run)?;
        Ok(run)
    }

    fn persist_handoff(&self, run: &mut FitRunDocument) -> Result<()> {
        let handoff = run
            .handoff
            .clone()
            .ok_or_else(|| anyhow!("FitRun 缺少 Codex handoff"))?;
        let payload = handoff_payload(run, &handoff);
        let path = self
            .store
            .write_handoff_artifact(run, &handoff.handoff_id, &payload)?;
        if let Some(current) = run.handoff.as_mut() {
            current.artifact_path = Some(path);
        }
        Ok(())
    }

    async fn run_lock(&self, run_id: &str) -> Arc<Mutex<()>> {
        if let Some(lock) = self.run_locks.read().await.get(run_id).cloned() {
            return lock;
        }
        self.run_locks
            .write()
            .await
            .entry(run_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    fn record_terminal_learning(&self, run: &FitRunDocument) {
        let decision = match run.phase {
            FitRunPhase::Accepted => FitUserDecision::Accepted,
            FitRunPhase::Cancelled => FitUserDecision::Rejected,
            FitRunPhase::Plateau | FitRunPhase::Failed => FitUserDecision::Pending,
            _ => return,
        };
        let result = self.store.read_trials(run).and_then(|trials| {
            record_and_promote(
                run,
                &trials,
                decision,
                run.stop_reason
                    .map(|reason| format!("FitRun 终止原因: {reason:?}")),
            )
        });
        if let Err(error) = result {
            tracing::warn!(
                run_id = %run.run_id,
                error = %error,
                "FitRun 已完成，但学习案例沉淀失败"
            );
        }
    }
}

fn matching_handoff<'a>(
    run: &'a mut FitRunDocument,
    handoff_id: &str,
) -> Result<&'a mut super::model::FitCodexHandoff> {
    run.handoff
        .as_mut()
        .filter(|value| value.handoff_id == handoff_id)
        .ok_or_else(|| anyhow!("handoffId 与当前 FitRun 不一致"))
}

fn validate_command_context(
    run: &FitRunDocument,
    context: &FitSessionContext,
    command: &FitCommand,
) -> Result<()> {
    validate_project_context(run, context)?;
    if matches!(command, FitCommand::RebindSession { .. }) {
        return Ok(());
    }
    if run.session_id != context.session_id || run.device_id != context.device_id {
        bail!("FitRun 仍绑定其他 Live Session；请先执行 REBIND_SESSION");
    }
    let lifecycle_only = matches!(command, FitCommand::Pause { .. } | FitCommand::Cancel { .. });
    let codex_result = matches!(
        command,
        FitCommand::CodexCompleted { .. } | FitCommand::CodexFailed { .. }
    );
    if !lifecycle_only && !codex_result && run.source_revision != context.source_revision {
        bail!("工作区 Source Revision 已变化；请先显式 REBIND_SESSION");
    }
    if !lifecycle_only
        && run.runtime_build_id.is_some()
        && run.runtime_build_id != context.runtime_build_id
    {
        bail!("Android Runtime Build 已变化；请先显式 REBIND_SESSION");
    }
    Ok(())
}

fn validate_project_context(run: &FitRunDocument, context: &FitSessionContext) -> Result<()> {
    let canonical_run = std::path::PathBuf::from(&run.project_root).canonicalize()?;
    let canonical_context = std::path::PathBuf::from(&context.project_root).canonicalize()?;
    if canonical_run != canonical_context || run.package_name != context.package_name {
        bail!("FitRun 不属于当前项目或 Android 包");
    }
    Ok(())
}
