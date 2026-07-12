use serde_json::json;

use super::model::{FitCodexHandoff, FitHandoffStatus, FitRunDocument};
use crate::node_agent_android_live::fit_learning::top_k_for_run;

pub(crate) fn new_codex_handoff(
    run: &FitRunDocument,
    reason: impl Into<String>,
) -> FitCodexHandoff {
    FitCodexHandoff {
        handoff_id: format!("handoff_{}", uuid::Uuid::new_v4().simple()),
        run_id: run.run_id.clone(),
        reason: reason.into(),
        status: FitHandoffStatus::Pending,
        created_at: chrono::Utc::now().to_rfc3339(),
        task_id: None,
        artifact_path: None,
        source_revision_before: run.source_revision.clone(),
        source_revision_after: None,
        changed_files: Vec::new(),
        commit_id: None,
        error: None,
    }
}

pub(crate) fn handoff_payload(
    run: &FitRunDocument,
    handoff: &FitCodexHandoff,
) -> serde_json::Value {
    let learning_priors = top_k_for_run(run, 3)
        .unwrap_or_default()
        .into_iter()
        .map(|matched| {
            json!({
                "priorId": matched.prior.prior_id,
                "scope": matched.prior.scope,
                "matchScore": matched.score,
                "confidence": matched.prior.confidence,
                "successRate": matched.prior.success_rate,
                "successCount": matched.prior.success_count,
                "failureCount": matched.prior.failure_count,
                "medianDeltas": matched.prior.median_deltas,
                "medianFactors": matched.prior.median_factors,
                "translationFeatures": matched.prior.translation_features,
                "runIds": matched.prior.run_ids,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schemaVersion": 1,
        "kind": "yilong_ui_fit_codex_handoff",
        "runId": run.run_id,
        "handoffId": handoff.handoff_id,
        "reason": handoff.reason,
        "pair": run.pair,
        "environment": run.environment,
        "properties": run.properties,
        "baseline": run.baseline,
        "current": run.current,
        "best": run.best,
        "sourceRevision": run.source_revision,
        "learningPriors": learning_priors,
        "instructions": [
            "先使用 yilong-ui-live MCP 读取最新节点、局部截图和局部源码。",
            "数值 LIVE 属性已由本地求解器尝试；优先判断父布局、组件结构、样式来源或 Binding 是否错误。",
            "只修改与目标节点及其必要父级相关的源码，不扩大范围。",
            "完成后回报 sourceRevisionBefore/sourceRevisionAfter、changedFiles 和可选 commitId。"
        ],
        "expectedReturn": {
            "handoffId": "string",
            "taskId": "string?",
            "sourceRevisionBefore": "string?",
            "sourceRevisionAfter": "string",
            "changedFiles": ["string"],
            "commitId": "string?",
            "tokenUsage": "number?"
        },
        "security": {
            "mcpTokenPersisted": false,
            "arbitraryShellInstructionsAllowed": false
        }
    })
}
