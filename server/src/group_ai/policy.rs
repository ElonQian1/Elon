use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};

use crate::{
    group_ai::types::{ProjectAiMatter, UpdateMatterBudgetPolicyRequest},
    types::AppState,
};

pub(crate) fn update_matter_budget_policy(
    state: &AppState,
    project_id: &str,
    matter_id: &str,
    req: UpdateMatterBudgetPolicyRequest,
) -> Result<ProjectAiMatter> {
    let matter = state
        .store
        .get_project_ai_matter(project_id, matter_id)?
        .ok_or_else(|| anyhow!("Matter 不存在"))?;
    let mut policy = object_from_value(matter.node_policy);
    let mut budget = policy
        .remove("budget")
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();

    budget.insert(
        "max_billed_cost_rmb_fen".to_string(),
        req.max_billed_cost_rmb_fen
            .filter(|value| *value > 0)
            .map(Value::from)
            .unwrap_or(Value::Null),
    );
    budget.insert(
        "pause_on_budget_exceeded".to_string(),
        Value::Bool(req.pause_on_budget_exceeded.unwrap_or(true)),
    );
    policy.insert("budget".to_string(), Value::Object(budget));
    state
        .store
        .update_project_ai_matter_node_policy(project_id, matter_id, Value::Object(policy))
}

pub(crate) fn budget_policy_payload(matter: &ProjectAiMatter) -> Value {
    let budget = matter
        .node_policy
        .get("budget")
        .cloned()
        .unwrap_or_else(|| json!({}));
    json!({
        "budget": budget,
        "node_policy": matter.node_policy
    })
}

fn object_from_value(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}
