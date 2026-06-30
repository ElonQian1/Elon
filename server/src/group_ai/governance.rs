use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};

use crate::{
    group_ai::{
        review_gate::{review_gate_summary, ReviewGateSummary},
        types::{ProjectAiMatter, ProjectAiMergeRequest, ProjectAiReview},
    },
    types::AppState,
};

#[derive(Debug, Serialize)]
pub(crate) struct MatterGovernanceSummary {
    pub reviews: Vec<ProjectAiReview>,
    pub merge_requests: Vec<ProjectAiMergeRequest>,
    pub review_gate: ReviewGateSummary,
    pub task_graph: MatterTaskGraph,
    pub policy: MatterPolicySummary,
    pub budget: MatterBudgetSummary,
}

#[derive(Debug, Serialize)]
pub(crate) struct MatterTaskGraph {
    pub nodes: Vec<MatterTaskNode>,
    pub edges: Vec<MatterTaskEdge>,
}

#[derive(Debug, Serialize)]
pub(crate) struct MatterTaskNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct MatterTaskEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct MatterPolicySummary {
    pub node_policy: Value,
    pub permission_levels: Vec<String>,
    pub allowed_clis: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct MatterBudgetSummary {
    pub status: String,
    pub compute_call_count: usize,
    pub billed_cost_rmb_fen: i64,
    pub max_billed_cost_rmb_fen: Option<i64>,
    pub remaining_billed_cost_rmb_fen: Option<i64>,
    pub provider_earned_fen: i64,
    pub warnings: Vec<String>,
}

pub(crate) fn matter_governance_summary(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
) -> Result<MatterGovernanceSummary> {
    let matter = state
        .store
        .get_project_ai_matter(project_id, matter_id)?
        .ok_or_else(|| anyhow!("Matter 不存在"))?;
    let assignments = state.store.list_project_ai_matter_assignments(matter_id)?;
    let reviews = state.store.list_project_ai_reviews(matter_id)?;
    let merge_requests = state
        .store
        .list_project_ai_merge_requests(project_id, matter_id)?;
    let events = state
        .store
        .list_project_ai_matter_events(project_id, matter_id)?;
    let review_gate = review_gate_summary(state, project_id, matter_id)?;
    let authorizations = state
        .store
        .list_project_ai_node_authorizations(project_id)?;
    let task_graph = task_graph(&assignments, &reviews, &merge_requests);
    let policy = MatterPolicySummary {
        node_policy: matter.node_policy.clone(),
        permission_levels: authorizations
            .iter()
            .map(|auth| auth.permission_level.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        allowed_clis: authorizations
            .iter()
            .flat_map(|auth| auth.allowed_clis.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        warnings: policy_warnings(&authorizations),
    };
    let budget = budget_summary(state, &events, &matter.node_policy);
    Ok(MatterGovernanceSummary {
        reviews,
        merge_requests,
        review_gate,
        task_graph,
        policy,
        budget,
    })
}

pub(crate) fn budget_dispatch_blocker(
    state: &AppState,
    matter: &ProjectAiMatter,
) -> Result<Option<String>> {
    if !pause_on_budget_exceeded(&matter.node_policy) {
        return Ok(None);
    }
    let events = state
        .store
        .list_project_ai_matter_events(&matter.project_id, &matter.id)?;
    let budget = budget_summary(state, &events, &matter.node_policy);
    if matches!(
        budget.status.as_str(),
        "budget_exceeded" | "budget_exhausted"
    ) {
        Ok(Some(format!(
            "Matter 预算已达到上限：已计费 {} 分，上限 {} 分",
            budget.billed_cost_rmb_fen,
            budget.max_billed_cost_rmb_fen.unwrap_or_default()
        )))
    } else {
        Ok(None)
    }
}

fn task_graph(
    assignments: &[crate::group_ai::types::ProjectAiMatterAssignment],
    reviews: &[ProjectAiReview],
    merge_requests: &[ProjectAiMergeRequest],
) -> MatterTaskGraph {
    let mut nodes = assignments
        .iter()
        .map(|assignment| MatterTaskNode {
            id: assignment.id.clone(),
            label: assignment.role.clone(),
            kind: if is_review_role(&assignment.role) {
                "review_assignment".to_string()
            } else {
                "assignment".to_string()
            },
            status: assignment.status.clone(),
        })
        .collect::<Vec<_>>();
    nodes.extend(reviews.iter().map(|review| MatterTaskNode {
        id: review.id.clone(),
        label: format!("review {}", review.severity),
        kind: "review_result".to_string(),
        status: review.status.clone(),
    }));
    nodes.extend(merge_requests.iter().map(|request| {
        MatterTaskNode {
            id: request.id.clone(),
            label: request
                .branch_name
                .clone()
                .unwrap_or_else(|| "merge request".to_string()),
            kind: "merge_request".to_string(),
            status: request.status.clone(),
        }
    }));

    let review_assignment_ids = assignments
        .iter()
        .filter(|assignment| is_review_role(&assignment.role))
        .map(|assignment| assignment.id.clone())
        .collect::<Vec<_>>();
    let mut edges = Vec::new();
    for assignment in assignments {
        if !is_review_role(&assignment.role) {
            for review_assignment_id in &review_assignment_ids {
                edges.push(MatterTaskEdge {
                    from: assignment.id.clone(),
                    to: review_assignment_id.clone(),
                    relation: "implementation_to_review".to_string(),
                });
            }
        }
    }
    for review in reviews {
        if let Some(target) = review.target_assignment_id.as_ref() {
            edges.push(MatterTaskEdge {
                from: target.clone(),
                to: review.id.clone(),
                relation: "reviewed_by".to_string(),
            });
        }
    }
    for request in merge_requests {
        edges.push(MatterTaskEdge {
            from: request.assignment_id.clone(),
            to: request.id.clone(),
            relation: "merge_candidate".to_string(),
        });
    }
    MatterTaskGraph { nodes, edges }
}

fn policy_warnings(
    authorizations: &[crate::group_ai::types::ProjectAiNodeAuthorization],
) -> Vec<String> {
    let mut warnings = Vec::new();
    if authorizations.is_empty() {
        warnings.push("当前项目没有授权 PC 节点".to_string());
    }
    if authorizations
        .iter()
        .any(|auth| auth.permission_level == "danger_full_access")
    {
        warnings.push("存在 danger_full_access 节点授权，建议只给可信节点使用".to_string());
    }
    if authorizations
        .iter()
        .any(|auth| auth.allowed_clis.is_empty())
    {
        warnings.push("存在未限制 CLI 类型的节点授权".to_string());
    }
    warnings
}

fn budget_summary(
    state: &AppState,
    events: &[crate::group_ai::types::ProjectAiEvent],
    node_policy: &Value,
) -> MatterBudgetSummary {
    let mut seen = HashSet::new();
    let mut billed_cost_rmb_fen = 0;
    let mut provider_earned_fen = 0;
    for event in events {
        if let Some(compute_call_id) = string_payload(&event.payload, "compute_call_id") {
            if !seen.insert(compute_call_id.clone()) {
                continue;
            }
            if let Ok(Some(run)) = state
                .store
                .get_node_compute_run_by_compute_call_id(&compute_call_id)
            {
                billed_cost_rmb_fen += run.billed_cost_rmb_fen;
                provider_earned_fen += run.provider_earned_fen;
            }
        }
    }
    let max_billed_cost_rmb_fen = max_billed_cost_rmb_fen(node_policy);
    let remaining_billed_cost_rmb_fen =
        max_billed_cost_rmb_fen.map(|max| max - billed_cost_rmb_fen);
    let status = budget_status(seen.len(), billed_cost_rmb_fen, max_billed_cost_rmb_fen);
    let mut warnings = if seen.is_empty() {
        vec!["还没有可对账的 compute_call_id".to_string()]
    } else {
        Vec::new()
    };
    if matches!(status, "budget_exceeded" | "budget_exhausted") {
        warnings.push("Matter 已达到预算上限，自动派发会暂停。".to_string());
    }
    MatterBudgetSummary {
        status: status.to_string(),
        compute_call_count: seen.len(),
        billed_cost_rmb_fen,
        max_billed_cost_rmb_fen,
        remaining_billed_cost_rmb_fen,
        provider_earned_fen,
        warnings,
    }
}

fn budget_status(count: usize, billed: i64, max: Option<i64>) -> &'static str {
    match max {
        Some(max) if billed > max => "budget_exceeded",
        Some(max) if billed == max && max > 0 => "budget_exhausted",
        _ if count == 0 => "no_compute_runs",
        _ => "accounted",
    }
}

fn max_billed_cost_rmb_fen(node_policy: &Value) -> Option<i64> {
    node_policy
        .get("budget")
        .and_then(|value| value.get("max_billed_cost_rmb_fen"))
        .and_then(Value::as_i64)
        .filter(|value| *value >= 0)
}

fn pause_on_budget_exceeded(node_policy: &Value) -> bool {
    node_policy
        .get("budget")
        .and_then(|value| value.get("pause_on_budget_exceeded"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn string_payload(payload: &Value, key: &str) -> Option<String> {
    payload
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
