use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use tokio::sync::{Mutex, RwLock};

use super::handoff::handoff_payload;
use super::model::{
    CreateFitRunRequest, FitCommand, FitCommandResult, FitRunDocument, FitRunPhase,
    FitSessionContext, FitStopReason,
};
use super::orchestrator::{advance_one, FitRunBackend};
use super::store::FitRunStore;
use crate::node_agent_android_live::broker::LiveUiBroker;
use crate::node_agent_android_live::fit_learning::{record_and_promote, FitUserDecision};

mod batch_accept;
#[cfg(test)]
mod batch_accept_tests;
mod command_application;
mod state_replay_attachment;

pub(crate) use batch_accept::{BatchAcceptRequest, BatchAcceptResult};
use command_application::apply_command;

const MAX_INTERNAL_STEPS: usize = 32;

pub(crate) struct FitRunService {
    store: FitRunStore,
    backend: Arc<dyn FitRunBackend>,
    run_locks: RwLock<HashMap<String, Arc<Mutex<()>>>>,
    batch_lock: Mutex<()>,
    live_broker: Option<Arc<LiveUiBroker>>,
}

impl FitRunService {
    pub(crate) fn new(store: FitRunStore, backend: Arc<dyn FitRunBackend>) -> Self {
        Self {
            store,
            backend,
            run_locks: RwLock::new(HashMap::new()),
            batch_lock: Mutex::new(()),
            live_broker: None,
        }
    }

    pub(crate) fn with_live_broker(mut self, broker: Arc<LiveUiBroker>) -> Self {
        self.live_broker = Some(broker);
        self
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
            if interrupted_command_should_resume(&command, run.phase) {
                run = self.drive(run).await?;
            }
            self.record_terminal_learning(&run);
            return Ok(FitCommandResult {
                run,
                idempotent: true,
            });
        }
        let command_id = command.command_id().to_string();
        let should_drive = apply_command(self, &context, &mut run, command).await?;
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

fn interrupted_command_should_resume(command: &FitCommand, phase: FitRunPhase) -> bool {
    matches!(
        (command, phase),
        (
            FitCommand::Start { .. },
            FitRunPhase::Baselining | FitRunPhase::LocalSolving
        ) | (FitCommand::AcceptBest { .. }, FitRunPhase::SourceVerifying)
            | (
                FitCommand::CodexCompleted { .. },
                FitRunPhase::Rebuilding | FitRunPhase::Evaluating
            )
    )
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
    let lifecycle_only = matches!(
        command,
        FitCommand::Pause { .. } | FitCommand::Cancel { .. }
    );
    let codex_result = matches!(
        command,
        FitCommand::CodexCompleted { .. } | FitCommand::CodexFailed { .. }
    );
    let resumes_interrupted_source_verification = matches!(
        (command, &run.phase),
        (FitCommand::AcceptBest { .. }, FitRunPhase::SourceVerifying)
    );
    if !lifecycle_only
        && !codex_result
        && !resumes_interrupted_source_verification
        && run.source_revision != context.source_revision
    {
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
