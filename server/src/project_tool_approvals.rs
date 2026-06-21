// server/src/project_tool_approvals.rs

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

static TOOL_APPROVALS: LazyLock<Mutex<HashMap<String, ToolApprovalRecord>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone)]
pub(crate) struct ToolApprovalTarget {
    pub req_id: String,
    pub decision: String,
}

#[derive(Clone)]
struct ToolApprovalRecord {
    project_id: String,
    channel_id: String,
    task_id: String,
    req_id: String,
    approval_id: String,
    status: ApprovalStatus,
    decision: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApprovalStatus {
    Pending,
    Decided,
}

pub(crate) fn register_required(project_id: &str, channel_id: &str, task_id: &str, event: &Value) {
    if event.get("type").and_then(Value::as_str) != Some("tool_approval_required") {
        return;
    }
    let Some(req_id) = event
        .get("req_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let Some(approval_id) = event
        .get("approval_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let record = ToolApprovalRecord {
        project_id: project_id.to_string(),
        channel_id: channel_id.to_string(),
        task_id: task_id.to_string(),
        req_id: req_id.to_string(),
        approval_id: approval_id.to_string(),
        status: ApprovalStatus::Pending,
        decision: None,
    };
    if let Ok(mut approvals) = TOOL_APPROVALS.lock() {
        approvals.insert(approval_key(task_id, approval_id), record);
    }
}

pub(crate) fn decision_target(
    project_id: &str,
    channel_id: &str,
    task_id: &str,
    approval_id: &str,
    decision: &str,
) -> Result<ToolApprovalTarget> {
    let decision = normalize_decision(decision)?;
    let approvals = TOOL_APPROVALS
        .lock()
        .map_err(|_| anyhow!("审批状态锁已损坏"))?;
    let record = approvals
        .get(&approval_key(task_id, approval_id))
        .ok_or_else(|| anyhow!("审批请求不存在或已过期"))?;
    if record.project_id != project_id
        || record.channel_id != channel_id
        || record.task_id != task_id
        || record.approval_id != approval_id
    {
        return Err(anyhow!("审批请求不属于当前项目频道"));
    }
    match record.status {
        ApprovalStatus::Pending => Ok(ToolApprovalTarget {
            req_id: record.req_id.clone(),
            decision,
        }),
        ApprovalStatus::Decided if record.decision.as_deref() == Some(decision.as_str()) => {
            Ok(ToolApprovalTarget {
                req_id: record.req_id.clone(),
                decision,
            })
        }
        ApprovalStatus::Decided => Err(anyhow!("审批请求已经被处理")),
    }
}

pub(crate) fn mark_decided(task_id: &str, approval_id: &str, decision: &str) {
    let Ok(mut approvals) = TOOL_APPROVALS.lock() else {
        return;
    };
    if let Some(record) = approvals.get_mut(&approval_key(task_id, approval_id)) {
        record.status = ApprovalStatus::Decided;
        record.decision = normalize_decision(decision).ok();
    }
}

pub(crate) fn clear_task(task_id: &str) {
    if let Ok(mut approvals) = TOOL_APPROVALS.lock() {
        approvals.retain(|_, record| record.task_id != task_id);
    }
}

fn normalize_decision(value: &str) -> Result<String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "approve" | "approved" => Ok("approve".to_string()),
        "deny" | "denied" | "reject" | "rejected" => Ok("deny".to_string()),
        _ => Err(anyhow!("decision 只能是 approve 或 deny")),
    }
}

fn approval_key(task_id: &str, approval_id: &str) -> String {
    format!("{task_id}:{approval_id}")
}

#[cfg(test)]
mod tests {
    use super::{decision_target, mark_decided, register_required};
    use serde_json::json;

    #[test]
    fn register_and_resolve_approval_target() {
        let task_id = "task-register-and-resolve";
        register_required(
            "project",
            "channel",
            task_id,
            &json!({
                "type": "tool_approval_required",
                "req_id": "req",
                "approval_id": "tap_1_1"
            }),
        );

        let target = decision_target("project", "channel", task_id, "tap_1_1", "approve").unwrap();
        assert_eq!(target.req_id, "req");
        assert_eq!(target.decision, "approve");
    }

    #[test]
    fn decided_approval_rejects_conflicting_decision() {
        let task_id = "task-conflicting-decision";
        register_required(
            "project",
            "channel",
            task_id,
            &json!({
                "type": "tool_approval_required",
                "req_id": "req",
                "approval_id": "tap_1_1"
            }),
        );
        mark_decided(task_id, "tap_1_1", "approve");

        assert!(decision_target("project", "channel", task_id, "tap_1_1", "deny").is_err());
    }
}
