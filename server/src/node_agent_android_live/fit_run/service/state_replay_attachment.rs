use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use super::super::model::{
    AttachStateReplayRequest, AttachStateReplayResult, FitRunAuditEvent, FitRunAuditOutcome,
    FitRunDocument, FitSessionContext,
};
use super::FitRunService;
use crate::node_agent_android_live::protocol::LiveUiNode;

impl FitRunService {
    pub(crate) async fn attach_state_replay(
        &self,
        context: FitSessionContext,
        run_id: &str,
        request: AttachStateReplayRequest,
    ) -> Result<AttachStateReplayResult> {
        request.validate_identity()?;
        let lock = self.run_lock(run_id).await;
        let _guard = lock.lock().await;
        let mut run = self.store.load(&context.project_root, run_id)?;
        validate_attachment_context(&run, &context, &request)?;
        let requested_replay_sha256 = replay_sha256(&request.state_replay)?;
        let previous_replay_sha256 = run
            .environment
            .state_replay
            .as_ref()
            .map(replay_sha256)
            .transpose()?;

        if run.phase.is_terminal() {
            self.persist_attachment_audit(
                &mut run,
                &request,
                FitRunAuditOutcome::RejectedImmutable,
                requested_replay_sha256.clone(),
                previous_replay_sha256,
                "FIT_STATE_REPLAY_RUN_IMMUTABLE: 终态 FitRun 不可附加 stateReplay",
            )?;
            bail!("FIT_STATE_REPLAY_RUN_IMMUTABLE: 终态 FitRun 不可附加 stateReplay");
        }

        if let Err(error) = request
            .state_replay
            .validate(Some(request.scenario.as_str()))
        {
            let detail = format!("{error:#}");
            self.persist_attachment_audit(
                &mut run,
                &request,
                FitRunAuditOutcome::RejectedInvalid,
                requested_replay_sha256.clone(),
                previous_replay_sha256,
                &detail,
            )?;
            bail!("{detail}");
        }

        if let Err(error) = validate_requested_target(&run, &request) {
            let detail = format!("{error:#}");
            self.persist_attachment_audit(
                &mut run,
                &request,
                FitRunAuditOutcome::RejectedInvalid,
                requested_replay_sha256.clone(),
                previous_replay_sha256,
                &detail,
            )?;
            bail!("{detail}");
        }

        let identical = run.environment.scenario.as_deref() == Some(request.scenario.as_str())
            && run.environment.state_replay.as_ref() == Some(&request.state_replay);
        if identical {
            self.persist_attachment_audit(
                &mut run,
                &request,
                FitRunAuditOutcome::Idempotent,
                requested_replay_sha256.clone(),
                Some(requested_replay_sha256.clone()),
                "FIT_STATE_REPLAY_IDEMPOTENT: 已持久化相同 stateReplay",
            )?;
            return Ok(AttachStateReplayResult {
                run,
                idempotent: true,
                replay_sha256: requested_replay_sha256,
            });
        }

        if run.environment.scenario.is_some() || run.environment.state_replay.is_some() {
            self.persist_attachment_audit(
                &mut run,
                &request,
                FitRunAuditOutcome::RejectedConflict,
                requested_replay_sha256.clone(),
                previous_replay_sha256,
                "FIT_STATE_REPLAY_CONFLICT: manifest 已绑定不同 scenario 或 stateReplay",
            )?;
            bail!("FIT_STATE_REPLAY_CONFLICT: manifest 已绑定不同 scenario 或 stateReplay");
        }

        if let Err(error) = self.validate_live_target(&context, &run, &request).await {
            let detail = format!("{error:#}");
            self.persist_attachment_audit(
                &mut run,
                &request,
                FitRunAuditOutcome::RejectedTargetMissing,
                requested_replay_sha256.clone(),
                None,
                &detail,
            )?;
            bail!("{detail}");
        }

        run.environment.scenario = Some(request.scenario.clone());
        run.environment.state_replay = Some(request.state_replay.clone());
        run.record_audit_event(FitRunAuditEvent::state_replay_attachment(
            &request,
            FitRunAuditOutcome::Attached,
            requested_replay_sha256.clone(),
            None,
            "FIT_STATE_REPLAY_ATTACHED: scenario 与 stateReplay 已原子持久化",
        ));
        self.store.save(&run)?;
        Ok(AttachStateReplayResult {
            run,
            idempotent: false,
            replay_sha256: requested_replay_sha256,
        })
    }

    async fn validate_live_target(
        &self,
        context: &FitSessionContext,
        run: &FitRunDocument,
        request: &AttachStateReplayRequest,
    ) -> Result<()> {
        let broker = self
            .live_broker
            .as_ref()
            .context("FIT_STATE_REPLAY_TARGET_UNAVAILABLE: FitRun 服务未绑定 Live Runtime")?;
        let view = broker.session_view(&context.session_id).await?;
        if !view.connected {
            bail!("FIT_STATE_REPLAY_TARGET_MISSING: 当前 Live Session 未连接");
        }
        let (_, nodes) = broker.tree(&context.session_id).await?;
        validate_live_target_nodes(run, request, &nodes)
    }

    fn persist_attachment_audit(
        &self,
        run: &mut FitRunDocument,
        request: &AttachStateReplayRequest,
        outcome: FitRunAuditOutcome,
        replay_sha256: String,
        previous_replay_sha256: Option<String>,
        detail: &str,
    ) -> Result<()> {
        run.record_audit_event(FitRunAuditEvent::state_replay_attachment(
            request,
            outcome,
            replay_sha256,
            previous_replay_sha256,
            detail,
        ));
        self.store.save(run)
    }
}

fn validate_attachment_context(
    run: &FitRunDocument,
    context: &FitSessionContext,
    request: &AttachStateReplayRequest,
) -> Result<()> {
    let requested_root = PathBuf::from(&request.project_root)
        .canonicalize()
        .context("ATTACH_STATE_REPLAY projectRoot 不存在")?;
    let context_root = PathBuf::from(&context.project_root)
        .canonicalize()
        .context("当前 Live Session projectRoot 不存在")?;
    let run_root = PathBuf::from(&run.project_root)
        .canonicalize()
        .context("FitRun projectRoot 不存在")?;
    if requested_root != context_root || run_root != context_root {
        bail!("FIT_STATE_REPLAY_PROJECT_MISMATCH: projectRoot 与 FitRun/Live Session 不一致");
    }
    if run.package_name != context.package_name {
        bail!("FIT_STATE_REPLAY_PROJECT_MISMATCH: FitRun 不属于当前 Android 包");
    }
    if run.session_id != context.session_id || run.device_id != context.device_id {
        bail!("FIT_STATE_REPLAY_SESSION_MISMATCH: FitRun 不属于当前 Live Session");
    }
    if run.source_revision != context.source_revision {
        bail!("FIT_STATE_REPLAY_RUN_STALE: 工作区 Source Revision 已变化");
    }
    if run.runtime_build_id != context.runtime_build_id {
        bail!("FIT_STATE_REPLAY_RUN_STALE: Android Runtime Build 已变化");
    }
    Ok(())
}

fn validate_requested_target(
    run: &FitRunDocument,
    request: &AttachStateReplayRequest,
) -> Result<()> {
    if run.pair.runtime_node_id != request.target_runtime_node_id
        || run.pair.definition_id != request.target_definition_id
        || run.pair.instance_key != request.target_instance_key
    {
        bail!("FIT_STATE_REPLAY_TARGET_MISMATCH: 请求 target node 与 FitRun 持久化目标不一致");
    }
    Ok(())
}

fn validate_live_target_nodes(
    run: &FitRunDocument,
    request: &AttachStateReplayRequest,
    nodes: &[LiveUiNode],
) -> Result<()> {
    let count = nodes
        .iter()
        .filter(|node| {
            node.runtime_node_id == run.pair.runtime_node_id
                && node.definition_id == run.pair.definition_id
                && node.instance_key == run.pair.instance_key
                && node.geometry.visible
        })
        .count();
    match count {
        1 => Ok(()),
        0 => bail!(
            "FIT_STATE_REPLAY_TARGET_MISSING: 当前 Runtime 中不存在请求的可见 target node: runtimeNodeId={} definitionId={}",
            request.target_runtime_node_id,
            request.target_definition_id
        ),
        count => bail!(
            "FIT_STATE_REPLAY_TARGET_AMBIGUOUS: 当前 Runtime target node 匹配 {count} 次"
        ),
    }
}

fn replay_sha256(replay: &super::super::model::FitStateReplay) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(replay)?)))
}
