use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use serde_json::Value;

use crate::{group_ai::types::ProjectAiMatter, types::AppState};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReviewGateSummary {
    pub status: String,
    pub passed_reviews: usize,
    pub blocking_reviews: usize,
    pub pending_merge_requests: usize,
    pub unfinished_assignments: usize,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

pub(crate) fn review_gate_summary(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
) -> Result<ReviewGateSummary> {
    review_gate_summary_inner(state, project_id, matter_id, None)
}

pub(crate) fn review_gate_summary_for_merge(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    merge_request_id: &str,
) -> Result<ReviewGateSummary> {
    review_gate_summary_inner(state, project_id, matter_id, Some(merge_request_id))
}

pub(crate) fn ensure_matter_acceptance_ready(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
) -> Result<ReviewGateSummary> {
    let summary = review_gate_summary(state, project_id, matter_id)?;
    if summary.status == "blocked" {
        bail!("Matter 验收门禁未通过: {}", summary.blockers.join("；"));
    }
    Ok(summary)
}

fn review_gate_summary_inner(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    exempt_merge_request_id: Option<&str>,
) -> Result<ReviewGateSummary> {
    let matter = require_matter(state, project_id, matter_id)?;
    let assignments = state.store.list_project_ai_matter_assignments(matter_id)?;
    let reviews = state.store.list_project_ai_reviews(matter_id)?;
    let merge_requests = state
        .store
        .list_project_ai_merge_requests(project_id, matter_id)?;
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let unfinished_assignments = assignments
        .iter()
        .filter(|assignment| !is_review_role(&assignment.role))
        .filter(|assignment| !is_finished_assignment_status(&assignment.status))
        .count();
    if unfinished_assignments > 0 {
        blockers.push(format!(
            "还有 {unfinished_assignments} 个实现 Assignment 未完成"
        ));
    }
    let passed_reviews = reviews
        .iter()
        .filter(|review| review_passed(review))
        .count();
    let blocking_reviews = reviews
        .iter()
        .filter(|review| review_blocking(review))
        .count();
    if blocking_reviews > 0 {
        blockers.push(format!("存在 {blocking_reviews} 个阻塞性 Review"));
    }
    if matter.collaboration_mode != "solo" && passed_reviews == 0 {
        blockers.push("非 solo Matter 至少需要一个 passed Review".to_string());
    } else if reviews.is_empty() {
        warnings.push("当前 Matter 还没有结构化 Review 结果".to_string());
    }
    let pending_merge_requests = merge_requests
        .iter()
        .filter(|request| Some(request.id.as_str()) != exempt_merge_request_id)
        .filter(|request| matches!(request.status.as_str(), "open" | "approved"))
        .count();
    if pending_merge_requests > 0 {
        blockers.push(format!("还有 {pending_merge_requests} 个合并项未处理"));
    }
    Ok(ReviewGateSummary {
        status: if blockers.is_empty() {
            "passed"
        } else {
            "blocked"
        }
        .to_string(),
        passed_reviews,
        blocking_reviews,
        pending_merge_requests,
        unfinished_assignments,
        blockers,
        warnings,
    })
}

fn require_matter(state: &AppState, project_id: &str, matter_id: &str) -> Result<ProjectAiMatter> {
    state
        .store
        .get_project_ai_matter(project_id, matter_id)?
        .ok_or_else(|| anyhow!("Matter 不存在"))
}

fn review_passed(review: &crate::group_ai::types::ProjectAiReview) -> bool {
    review.status == "passed"
        || review.finding.get("status").and_then(Value::as_str) == Some("passed")
}

fn review_blocking(review: &crate::group_ai::types::ProjectAiReview) -> bool {
    matches!(
        review.status.as_str(),
        "needs_changes" | "blocked" | "failed" | "rejected"
    ) || matches!(
        review.finding.get("status").and_then(Value::as_str),
        Some("needs_changes" | "blocked" | "failed" | "rejected")
    )
}

fn is_review_role(role: &str) -> bool {
    let role = role.trim().to_ascii_lowercase();
    role.contains("review") || role.contains("critic")
}

fn is_finished_assignment_status(status: &str) -> bool {
    matches!(status, "completed" | "settled" | "settled_no_provider")
}
