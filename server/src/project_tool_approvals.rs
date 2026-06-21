// server/src/project_tool_approvals.rs

use serde_json::Value;
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    sync::{LazyLock, Mutex},
};

static TOOL_APPROVALS: LazyLock<Mutex<HashMap<String, ToolApprovalRecord>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolApprovalTarget {
    pub project_id: String,
    pub channel_id: String,
    pub task_id: String,
    pub req_id: String,
    pub approval_id: String,
    pub message_id: Option<String>,
    pub decision: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ToolApprovalClaim {
    Dispatch(ToolApprovalTarget),
    AlreadyDecided { decision: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolApprovalErrorKind {
    BadRequest,
    Conflict,
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolApprovalError {
    kind: ToolApprovalErrorKind,
    message: String,
}

impl ToolApprovalError {
    pub(crate) fn kind(&self) -> ToolApprovalErrorKind {
        self.kind
    }
}

impl fmt::Display for ToolApprovalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ToolApprovalError {}

type ToolApprovalResult<T> = std::result::Result<T, ToolApprovalError>;

#[derive(Clone)]
struct ToolApprovalRecord {
    project_id: String,
    channel_id: String,
    task_id: String,
    req_id: String,
    approval_id: String,
    message_id: Option<String>,
    status: ApprovalStatus,
    decision: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApprovalStatus {
    Pending,
    Dispatching,
    Decided,
}

pub(crate) fn register_required(
    project_id: &str,
    channel_id: &str,
    task_id: &str,
    event: &Value,
) -> bool {
    if event.get("type").and_then(Value::as_str) != Some("tool_approval_required") {
        return false;
    }
    let Some(req_id) = event
        .get("req_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let Some(approval_id) = event
        .get("approval_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let record = ToolApprovalRecord {
        project_id: project_id.to_string(),
        channel_id: channel_id.to_string(),
        task_id: task_id.to_string(),
        req_id: req_id.to_string(),
        approval_id: approval_id.to_string(),
        message_id: optional_trimmed_string(event, "message_id"),
        status: ApprovalStatus::Pending,
        decision: None,
    };
    if let Ok(mut approvals) = TOOL_APPROVALS.lock() {
        // 事件流可能重放同一个审批卡；已有状态不能被重置成 Pending。
        let inserted = approvals
            .entry(approval_key(task_id, approval_id))
            .or_insert(record);
        return inserted.project_id == project_id
            && inserted.channel_id == channel_id
            && inserted.task_id == task_id
            && inserted.approval_id == approval_id;
    }
    false
}

pub(crate) fn register_decision_event(
    project_id: &str,
    channel_id: &str,
    task_id: &str,
    event: &Value,
) -> bool {
    if event.get("type").and_then(Value::as_str) != Some("tool_approval_decision") {
        return false;
    }
    let Some(approval_id) = event
        .get("approval_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let Some(decision) = terminal_decision(event) else {
        return false;
    };
    let req_id = event
        .get("req_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let key = approval_key(task_id, approval_id);
    let Ok(mut approvals) = TOOL_APPROVALS.lock() else {
        return false;
    };
    if let Some(record) = approvals.get_mut(&key) {
        if record.project_id != project_id
            || record.channel_id != channel_id
            || record.task_id != task_id
            || record.approval_id != approval_id
        {
            return false;
        }
        record.status = ApprovalStatus::Decided;
        record.decision = Some(decision);
        return true;
    }
    let Some(req_id) = req_id else {
        return false;
    };
    approvals.insert(
        key,
        ToolApprovalRecord {
            project_id: project_id.to_string(),
            channel_id: channel_id.to_string(),
            task_id: task_id.to_string(),
            req_id: req_id.to_string(),
            approval_id: approval_id.to_string(),
            message_id: optional_trimmed_string(event, "message_id"),
            status: ApprovalStatus::Decided,
            decision: Some(decision),
        },
    );
    true
}

pub(crate) fn claim_decision_target(
    project_id: &str,
    channel_id: &str,
    task_id: &str,
    approval_id: &str,
    decision: &str,
) -> ToolApprovalResult<ToolApprovalClaim> {
    let decision = normalize_decision(decision)?;
    let mut approvals = TOOL_APPROVALS
        .lock()
        .map_err(|_| approval_error(ToolApprovalErrorKind::Conflict, "审批状态锁已损坏"))?;
    let record = approvals
        .get_mut(&approval_key(task_id, approval_id))
        .ok_or_else(|| approval_error(ToolApprovalErrorKind::NotFound, "审批请求不存在或已过期"))?;
    if record.project_id != project_id
        || record.channel_id != channel_id
        || record.task_id != task_id
        || record.approval_id != approval_id
    {
        return Err(approval_error(
            ToolApprovalErrorKind::BadRequest,
            "审批请求不属于当前项目频道",
        ));
    }

    match record.status {
        ApprovalStatus::Pending => {
            // 关键并发点：认领和状态切换必须在同一把锁里完成，避免 approve/deny 同时派发。
            record.status = ApprovalStatus::Dispatching;
            record.decision = Some(decision.clone());
            Ok(ToolApprovalClaim::Dispatch(ToolApprovalTarget {
                project_id: record.project_id.clone(),
                channel_id: record.channel_id.clone(),
                task_id: record.task_id.clone(),
                req_id: record.req_id.clone(),
                approval_id: record.approval_id.clone(),
                message_id: record.message_id.clone(),
                decision,
            }))
        }
        ApprovalStatus::Dispatching => Err(approval_error(
            ToolApprovalErrorKind::Conflict,
            "审批请求正在发送，请勿重复操作",
        )),
        ApprovalStatus::Decided if record.decision.as_deref() == Some(decision.as_str()) => {
            Ok(ToolApprovalClaim::AlreadyDecided { decision })
        }
        ApprovalStatus::Decided => Err(approval_error(
            ToolApprovalErrorKind::Conflict,
            "审批请求已经被处理",
        )),
    }
}

pub(crate) fn mark_decided(task_id: &str, approval_id: &str, decision: &str) -> bool {
    let Ok(mut approvals) = TOOL_APPROVALS.lock() else {
        return false;
    };
    let Ok(decision) = normalize_decision(decision) else {
        return false;
    };
    if let Some(record) = approvals.get_mut(&approval_key(task_id, approval_id)) {
        if record.status == ApprovalStatus::Dispatching
            && record.decision.as_deref() == Some(decision.as_str())
        {
            record.status = ApprovalStatus::Decided;
            return true;
        }
    }
    false
}

pub(crate) fn mark_dispatch_failed(task_id: &str, approval_id: &str, decision: &str) -> bool {
    let Ok(mut approvals) = TOOL_APPROVALS.lock() else {
        return false;
    };
    let Ok(decision) = normalize_decision(decision) else {
        return false;
    };
    if let Some(record) = approvals.get_mut(&approval_key(task_id, approval_id)) {
        if record.status == ApprovalStatus::Dispatching
            && record.decision.as_deref() == Some(decision.as_str())
        {
            // 发送失败说明决定还没有交给 PC 节点，释放认领让用户可以重试。
            record.status = ApprovalStatus::Pending;
            record.decision = None;
            return true;
        }
    }
    false
}

pub(crate) fn clear_task(task_id: &str) {
    if let Ok(mut approvals) = TOOL_APPROVALS.lock() {
        approvals.retain(|_, record| record.task_id != task_id);
    }
}

fn normalize_decision(value: &str) -> ToolApprovalResult<String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "approve" | "approved" => Ok("approve".to_string()),
        "deny" | "denied" | "reject" | "rejected" => Ok("deny".to_string()),
        _ => Err(approval_error(
            ToolApprovalErrorKind::BadRequest,
            "decision 只能是 approve 或 deny",
        )),
    }
}

fn terminal_decision(event: &Value) -> Option<String> {
    let raw = event
        .get("decision")
        .and_then(Value::as_str)
        .or_else(|| event.get("status").and_then(Value::as_str))?
        .trim()
        .to_ascii_lowercase();
    match raw.as_str() {
        "approve" | "approved" => Some("approve".to_string()),
        "deny" | "denied" | "reject" | "rejected" => Some("deny".to_string()),
        // 运行时已经超时或取消时，后续点击不能重新派发；保留终态即可。
        "timeout" | "expired" => Some("timeout".to_string()),
        "cancel" | "canceled" | "cancelled" => Some("canceled".to_string()),
        _ => None,
    }
}

fn approval_error(kind: ToolApprovalErrorKind, message: impl Into<String>) -> ToolApprovalError {
    ToolApprovalError {
        kind,
        message: message.into(),
    }
}

fn optional_trimmed_string(event: &Value, field: &str) -> Option<String> {
    event
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn approval_key(task_id: &str, approval_id: &str) -> String {
    format!("{task_id}:{approval_id}")
}

#[cfg(test)]
mod tests {
    use super::{
        claim_decision_target, clear_task, mark_decided, mark_dispatch_failed, register_required,
        ToolApprovalClaim, ToolApprovalErrorKind,
    };
    use serde_json::json;

    #[test]
    fn register_and_claim_dispatch_target() {
        let task_id = "task-register-and-resolve";
        clear_task(task_id);
        register_required(
            "project",
            "channel",
            task_id,
            &json!({
                "type": "tool_approval_required",
                "req_id": "req",
                "message_id": "msg-1",
                "approval_id": "tap_1_1"
            }),
        );

        let claim =
            claim_decision_target("project", "channel", task_id, "tap_1_1", "approve").unwrap();
        let ToolApprovalClaim::Dispatch(target) = claim else {
            panic!("first approval decision must dispatch");
        };
        assert_eq!(target.project_id, "project");
        assert_eq!(target.channel_id, "channel");
        assert_eq!(target.task_id, task_id);
        assert_eq!(target.req_id, "req");
        assert_eq!(target.approval_id, "tap_1_1");
        assert_eq!(target.message_id.as_deref(), Some("msg-1"));
        assert_eq!(target.decision, "approve");
        clear_task(task_id);
    }

    #[test]
    fn claim_prevents_conflicting_concurrent_decision() {
        let task_id = "task-approve-then-deny-cannot-claim-again";
        clear_task(task_id);
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

        assert!(claim_decision_target("project", "channel", task_id, "tap_1_1", "approve").is_ok());

        let err =
            claim_decision_target("project", "channel", task_id, "tap_1_1", "deny").unwrap_err();
        assert_eq!(err.kind(), ToolApprovalErrorKind::Conflict);
        clear_task(task_id);
    }

    #[test]
    fn duplicate_decision_is_idempotent_after_dispatch_success() {
        let task_id = "task-duplicate-decision-does-not-return-dispatch-target";
        clear_task(task_id);
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

        assert!(
            claim_decision_target("project", "channel", task_id, "tap_1_1", "approved").is_ok()
        );
        assert!(
            claim_decision_target("project", "channel", task_id, "tap_1_1", "approve").is_err()
        );
        assert!(mark_decided(task_id, "tap_1_1", "approve"));

        let claim =
            claim_decision_target("project", "channel", task_id, "tap_1_1", "approve").unwrap();
        assert_eq!(
            claim,
            ToolApprovalClaim::AlreadyDecided {
                decision: "approve".to_string()
            }
        );
        let err =
            claim_decision_target("project", "channel", task_id, "tap_1_1", "deny").unwrap_err();
        assert_eq!(err.kind(), ToolApprovalErrorKind::Conflict);
        clear_task(task_id);
    }

    #[test]
    fn failed_dispatch_releases_claim_for_retry() {
        let task_id = "task-dispatch-failure-retry";
        clear_task(task_id);
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

        assert!(claim_decision_target("project", "channel", task_id, "tap_1_1", "approve").is_ok());
        assert!(mark_dispatch_failed(task_id, "tap_1_1", "approve"));
        assert!(claim_decision_target("project", "channel", task_id, "tap_1_1", "deny").is_ok());
        clear_task(task_id);
    }

    #[test]
    fn stale_dispatch_result_cannot_override_new_claim() {
        let task_id = "task-stale-dispatch-result";
        clear_task(task_id);
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

        assert!(claim_decision_target("project", "channel", task_id, "tap_1_1", "approve").is_ok());
        assert!(mark_dispatch_failed(task_id, "tap_1_1", "approve"));
        assert!(claim_decision_target("project", "channel", task_id, "tap_1_1", "deny").is_ok());

        assert!(!mark_decided(task_id, "tap_1_1", "approve"));
        assert!(mark_decided(task_id, "tap_1_1", "deny"));
        clear_task(task_id);
    }
}
