use serde_json::json;

use crate::{
    group_ai::{review_gate::ReviewGateSummary, types::ProjectAiMatter},
    store::MEMORY_SCOPE_PROJECT,
    types::AppState,
};

pub(crate) fn record_matter_decision_learning(
    state: &AppState,
    matter: &ProjectAiMatter,
    actor_user_id: &str,
    decision: &str,
    comment: Option<&str>,
    gate: Option<&ReviewGateSummary>,
) {
    let content = json!({
        "type": "project_ai_matter_decision",
        "matter_id": matter.id,
        "title": matter.title,
        "decision": decision,
        "comment": clean(comment),
        "review_gate": gate.map(|gate| json!({
            "status": &gate.status,
            "passed_reviews": gate.passed_reviews,
            "blocking_reviews": gate.blocking_reviews,
            "pending_merge_requests": gate.pending_merge_requests,
            "unfinished_assignments": gate.unfinished_assignments,
            "blockers": &gate.blockers,
        })),
        "lesson": lesson(decision, comment, gate),
    })
    .to_string();
    if let Err(error) = state.store.insert_user_memory_scoped(
        actor_user_id,
        &content,
        "project_ai_decision",
        importance(decision),
        None,
        MEMORY_SCOPE_PROJECT,
        Some(&matter.project_id),
    ) {
        tracing::warn!(matter_id = matter.id, "群体 AI 决策记忆写入失败: {error:#}");
    }
}

fn lesson(decision: &str, comment: Option<&str>, gate: Option<&ReviewGateSummary>) -> String {
    let comment = clean(comment).unwrap_or_else(|| "无人工备注".to_string());
    match decision {
        "accepted" => format!(
            "该类群体 AI Matter 可在 Review 门禁通过、合并项清空后验收；验收备注：{comment}"
        ),
        "changes_requested" => {
            let blockers = gate
                .map(|gate| gate.blockers.join("；"))
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "人工要求补充".to_string());
            format!("该类群体 AI Matter 返工原因：{blockers}；人工备注：{comment}")
        }
        _ => format!("群体 AI Matter 决策为 {decision}；人工备注：{comment}"),
    }
}

fn importance(decision: &str) -> i64 {
    match decision {
        "accepted" | "changes_requested" => 8,
        _ => 6,
    }
}

fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(240).collect())
}
