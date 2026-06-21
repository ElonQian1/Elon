// server/src/node_agent_runtime_approval.rs

use std::time::Duration;

use serde_json::Value;
use tokio::sync::watch;

use crate::{
    node_agent_tool_approval::{ToolApprovalDecision, ToolApprovalWaiter},
    node_agent_tool_guard::ToolGuard,
};

const TOOL_APPROVAL_TIMEOUT_SECS: u64 = 30 * 60;

pub(crate) enum ApprovalOutcome {
    Approved,
    Denied(String),
    TimedOut,
    Canceled,
}

pub(crate) fn requires_tool_approval(guard: &ToolGuard, action: &Value) -> bool {
    if guard.read_only() {
        return false;
    }
    let tool = action
        .get("tool")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match tool {
        "write_file" | "run_command" => true,
        "apply_patch" => !action
            .get("check_only")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        _ => false,
    }
}

pub(crate) async fn wait_for_tool_approval(
    waiter: &mut ToolApprovalWaiter,
    cancel_rx: &mut watch::Receiver<bool>,
) -> ApprovalOutcome {
    if *cancel_rx.borrow() {
        waiter.cleanup().await;
        return ApprovalOutcome::Canceled;
    }
    let timeout = tokio::time::sleep(Duration::from_secs(TOOL_APPROVAL_TIMEOUT_SECS));
    tokio::pin!(timeout);
    let outcome = loop {
        tokio::select! {
            _ = &mut timeout => break ApprovalOutcome::TimedOut,
            changed = cancel_rx.changed() => {
                if changed.is_ok() && *cancel_rx.borrow() {
                    break ApprovalOutcome::Canceled;
                }
            }
            changed = waiter.changed() => {
                if !changed {
                    break ApprovalOutcome::Denied("approval channel closed".to_string());
                }
                break match waiter.decision() {
                    Some(ToolApprovalDecision::Approve) => ApprovalOutcome::Approved,
                    Some(ToolApprovalDecision::Deny) => ApprovalOutcome::Denied("denied".to_string()),
                    None => ApprovalOutcome::Denied("approval decision missing".to_string()),
                };
            }
        }
    };
    waiter.cleanup().await;
    outcome
}
