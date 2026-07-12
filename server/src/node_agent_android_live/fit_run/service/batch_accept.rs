use std::collections::BTreeSet;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::super::live_values::resolve_runtime_node;
use super::super::model::{FitRunDocument, FitRunPhase, FitSessionContext, FitStopReason};
use super::{validate_project_context, FitRunService};
use crate::node_agent_android_live::build_verify::{
    build_and_verify, BuildVerifyRequest, BuildVerifyResult,
};
use crate::node_agent_android_live::protocol::{
    LivePatchOperation, LivePatchTarget, LiveStylePatch,
};
use crate::node_agent_android_live::source_commit::{
    build_source_commit_plan_for_patches, commit_source_plan, SourceCommitRequest,
    SourceCommitResult,
};
use crate::node_agent_android_live::ui_ir::load_or_build_ui_ir;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BatchAcceptRequest {
    pub(crate) run_ids: Vec<String>,
    pub(crate) source_revision: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BatchAcceptResult {
    pub(crate) status: &'static str,
    pub(crate) run_ids: Vec<String>,
    pub(crate) plan: Value,
    pub(crate) commit: Option<SourceCommitResult>,
    pub(crate) build: Option<BuildVerifyResult>,
    pub(crate) codex_bundle: Option<Value>,
    pub(crate) runs: Vec<FitRunDocument>,
}

impl FitRunService {
    pub(crate) async fn accept_batch(
        &self,
        context: FitSessionContext,
        request: BatchAcceptRequest,
    ) -> Result<BatchAcceptResult> {
        validate_batch_request(&request)?;
        let _guard = self.batch_lock.lock().await;
        let broker = self
            .live_broker
            .as_ref()
            .ok_or_else(|| anyhow!("批量拟合验收只支持真实 Android Live Runtime"))?;
        let session = broker.session(&context.session_id).await?;
        let ir = load_or_build_ui_ir(broker, &context.session_id).await?;
        let mut runs = request
            .run_ids
            .iter()
            .map(|run_id| self.store.load(&context.project_root, run_id))
            .collect::<Result<Vec<_>>>()?;
        for run in &runs {
            validate_project_context(run, &context)?;
            if run.session_id != context.session_id || run.device_id != context.device_id {
                bail!("FitRun {} 不属于当前 Live Session", run.run_id);
            }
            if run.phase != FitRunPhase::CandidateReady {
                bail!("FitRun {} 尚未达到 CANDIDATE_READY", run.run_id);
            }
            if run.source_revision.as_deref() != Some(request.source_revision.as_str()) {
                bail!("FitRun {} 的源码基线已经变化", run.run_id);
            }
        }
        let patches = runs
            .iter()
            .map(|run| patch_for_run(run, &ir.nodes))
            .collect::<Result<Vec<_>>>()?;
        let plan = build_source_commit_plan_for_patches(session, patches).await?;
        if plan.source_revision != request.source_revision {
            bail!("源码已在批量拟合期间变化，拒绝覆盖");
        }
        let plan_value = serde_json::to_value(&plan)?;
        if plan.codex_count > 0 {
            return Ok(BatchAcceptResult {
                status: "CODEX_REQUIRED",
                run_ids: request.run_ids,
                codex_bundle: Some(codex_bundle(&runs, &plan_value)),
                plan: plan_value,
                commit: None,
                build: None,
                runs,
            });
        }
        let commit = commit_source_plan(
            plan,
            SourceCommitRequest {
                source_revision: request.source_revision,
            },
        )?;
        let host_port = crate::node_agent_admin_open::admin_port_from_env();
        let build = build_and_verify(
            broker,
            &context.session_id,
            BuildVerifyRequest::default(),
            host_port,
        )
        .await?;
        let verified = build.verification_gate.verified;
        for run in &mut runs {
            apply_batch_verification(run, &commit, &build, verified)?;
            self.store.save(run)?;
            self.record_terminal_learning(run);
        }
        Ok(BatchAcceptResult {
            status: if verified {
                "BUILD_VERIFIED"
            } else {
                "VERIFY_FAILED"
            },
            run_ids: request.run_ids,
            plan: plan_value,
            commit: Some(commit),
            build: Some(build),
            codex_bundle: None,
            runs,
        })
    }
}

fn validate_batch_request(request: &BatchAcceptRequest) -> Result<()> {
    if request.run_ids.is_empty() || request.run_ids.len() > 64 {
        bail!("runIds 必须包含 1..64 个拟合任务");
    }
    if request.source_revision.trim().is_empty() {
        bail!("sourceRevision 不能为空");
    }
    let unique = request.run_ids.iter().collect::<BTreeSet<_>>();
    if unique.len() != request.run_ids.len() {
        bail!("runIds 不允许重复");
    }
    Ok(())
}

fn patch_for_run(
    run: &FitRunDocument,
    nodes: &[crate::node_agent_android_live::protocol::LiveUiNode],
) -> Result<LiveStylePatch> {
    let node = resolve_runtime_node(run, nodes)?;
    let best = run
        .best
        .as_ref()
        .ok_or_else(|| anyhow!("FitRun {} 没有最佳候选", run.run_id))?;
    if best.operations.is_empty() {
        bail!("FitRun {} 没有可写回操作", run.run_id);
    }
    let operations = best
        .operations
        .iter()
        .cloned()
        .map(serde_json::from_value::<LivePatchOperation>)
        .collect::<serde_json::Result<Vec<_>>>()?;
    Ok(LiveStylePatch {
        protocol_version: 1,
        message_type: String::new(),
        session_id: String::new(),
        request_id: String::new(),
        gesture_id: Some(format!("fit-batch:{}", run.run_id)),
        sequence: 0,
        base_tree_revision: None,
        target: LivePatchTarget {
            scope: "INSTANCE".to_string(),
            runtime_node_id: Some(node.runtime_node_id.clone()),
            definition_id: Some(run.pair.definition_id.clone()),
            instance_key: run.pair.instance_key.clone(),
        },
        atomic: true,
        ephemeral: true,
        operations,
    })
}

fn apply_batch_verification(
    run: &mut FitRunDocument,
    commit: &SourceCommitResult,
    build: &BuildVerifyResult,
    verified: bool,
) -> Result<()> {
    run.transition(FitRunPhase::SourceVerifying)?;
    let mut candidate = run
        .best
        .clone()
        .ok_or_else(|| anyhow!("FitRun {} 没有最佳候选", run.run_id))?;
    candidate.source_revision = Some(commit.source_revision_after.clone());
    candidate.runtime_build_id = build.runtime_build_id.clone();
    candidate.source_parity_loss = Some(build.source_parity_diff.visual_loss);
    candidate.source_parity_verified = verified;
    run.current = Some(candidate.clone());
    run.best = Some(candidate);
    run.source_revision = Some(commit.source_revision_after.clone());
    run.runtime_build_id = build.runtime_build_id.clone();
    run.usage.build_rounds = run.usage.build_rounds.saturating_add(1);
    run.stop_reason = Some(if verified {
        FitStopReason::SourceVerified
    } else {
        FitStopReason::BackendError
    });
    run.transition(if verified {
        FitRunPhase::Accepted
    } else {
        FitRunPhase::Failed
    })?;
    Ok(())
}

fn codex_bundle(runs: &[FitRunDocument], plan: &Value) -> Value {
    json!({
        "schemaVersion": 1,
        "kind": "yilong_ui_fit_batch_codex_handoff",
        "runCount": runs.len(),
        "runs": runs.iter().map(|run| json!({
            "runId": run.run_id,
            "definitionId": run.pair.definition_id,
            "componentKind": run.pair.component_kind,
            "parentLayoutKind": run.pair.parent_layout_kind,
            "targetRect": run.pair.target_rect,
            "currentRect": run.pair.current_rect,
            "operations": run.best.as_ref().map(|candidate| &candidate.operations),
        })).collect::<Vec<_>>(),
        "sourcePlan": plan,
        "instructions": [
            "只读取 sourcePlan 中 CODEX 条目的必要源码 Symbol 和直接父布局。",
            "保持已确定的 LIVE 数值，不重新分析完整页面或完整仓库。",
            "一次性完成所有 deferred 条目，再调用批量验收接口重新执行双门禁。"
        ]
    })
}
