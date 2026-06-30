use serde_json::{json, Value};

use crate::{
    group_ai::types::{ProjectAiMatter, ProjectAiMatterAssignment, RecordReviewInput},
    types::AppState,
};

pub(crate) fn record_review_from_assignment_result(
    state: &AppState,
    matter: &ProjectAiMatter,
    assignment: &ProjectAiMatterAssignment,
    actor_user_id: &str,
    summary: &str,
) {
    if !is_review_role(&assignment.role) {
        return;
    }
    let finding = extract_review_json(summary).unwrap_or_else(|| {
        json!({
            "schema": "project_ai.review_result.v1",
            "status": "needs_human_review",
            "risk_level": "medium",
            "summary": summary.chars().take(2000).collect::<String>(),
            "fallback": true
        })
    });
    let status = json_string(&finding, "status").unwrap_or_else(|| "open".to_string());
    let severity = json_string(&finding, "risk_level")
        .or_else(|| json_string(&finding, "severity"))
        .unwrap_or_else(|| "medium".to_string());
    let target_assignment_id = json_string(&finding, "target_assignment_id");
    match state.store.record_project_ai_review(RecordReviewInput {
        matter_id: matter.id.clone(),
        reviewer_bot_id: Some(assignment.bot_id.clone()),
        reviewer_user_id: Some(actor_user_id.to_string()),
        target_assignment_id,
        severity,
        finding,
        status,
    }) {
        Ok(review) => {
            let _ = state.store.insert_project_ai_event(
                &matter.project_id,
                &matter.id,
                Some(actor_user_id),
                "review_result_recorded",
                json!({
                    "review_id": review.id,
                    "assignment_id": assignment.id,
                    "status": review.status,
                    "severity": review.severity,
                    "target_assignment_id": review.target_assignment_id
                }),
            );
        }
        Err(error) => {
            tracing::warn!(
                matter_id = matter.id,
                assignment_id = assignment.id,
                "Review 结果落库失败: {error:#}"
            );
        }
    }
}

fn extract_review_json(summary: &str) -> Option<Value> {
    let trimmed = summary.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Some(value);
    }
    let fenced = trimmed
        .split("```")
        .find(|part| part.trim_start().starts_with("json"))
        .and_then(|part| part.trim_start().strip_prefix("json"))
        .map(str::trim);
    if let Some(json_text) = fenced {
        if let Ok(value) = serde_json::from_str::<Value>(json_text) {
            return Some(value);
        }
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if start >= end {
        return None;
    }
    serde_json::from_str::<Value>(&trimmed[start..=end]).ok()
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn is_review_role(role: &str) -> bool {
    let role = role.trim().to_ascii_lowercase();
    role.contains("review") || role.contains("critic")
}
