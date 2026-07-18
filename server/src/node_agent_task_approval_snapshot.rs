use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct TaskApprovalJournalSnapshot {
    pub approvals: Vec<TaskApprovalJournalItem>,
    pub pending_count: usize,
    pub decided_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskApprovalJournalItem {
    pub approval_id: String,
    pub tool: Option<String>,
    pub status: &'static str,
    pub decision: Option<String>,
    pub checkpoint: Option<Value>,
    pub required_seq: Option<usize>,
    pub decision_seq: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct TaskApprovalStateSnapshot {
    pub approvals: Vec<TaskApprovalStateItem>,
    pub total_count: usize,
    pub actionable_count: usize,
    pub decided_count: usize,
    pub unavailable_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskApprovalStateItem {
    pub approval_id: String,
    pub tool: Option<String>,
    pub status: &'static str,
    pub decision: Option<String>,
    pub actionable: bool,
    pub next_action: &'static str,
    pub requires_new_task: bool,
    pub checkpoint: Option<Value>,
    pub label: &'static str,
    pub tone: &'static str,
    pub meta: &'static str,
    pub required_seq: Option<usize>,
    pub decision_seq: Option<usize>,
}

#[derive(Debug, Default)]
pub(crate) struct TaskApprovalJournalTracker {
    approvals: BTreeMap<String, ApprovalAccumulator>,
}

#[derive(Debug, Default)]
struct ApprovalAccumulator {
    approval_id: String,
    tool: Option<String>,
    required_seq: Option<usize>,
    decision_seq: Option<usize>,
    decision: Option<String>,
    checkpoint: Option<Value>,
}

impl TaskApprovalJournalTracker {
    pub(crate) fn observe_event(&mut self, seq: usize, event: &Value) {
        let Some(inner) = tool_event_payload(event) else {
            return;
        };
        match inner.get("type").and_then(Value::as_str) {
            Some("tool_approval_required") => self.observe_required(seq, inner),
            Some("tool_approval_decision") => self.observe_decision(seq, inner),
            _ => {}
        }
    }

    pub(crate) fn finish(self) -> TaskApprovalJournalSnapshot {
        let mut approvals: Vec<_> = self
            .approvals
            .into_values()
            .map(|approval| approval.into_item())
            .collect();
        approvals.sort_by(|left, right| {
            first_seq(left)
                .cmp(&first_seq(right))
                .then_with(|| left.approval_id.cmp(&right.approval_id))
        });
        let pending_count = approvals
            .iter()
            .filter(|approval| approval.status == "pending")
            .count();
        let decided_count = approvals.len().saturating_sub(pending_count);
        TaskApprovalJournalSnapshot {
            approvals,
            pending_count,
            decided_count,
        }
    }

    fn observe_required(&mut self, seq: usize, event: &Value) {
        let Some(approval_id) = approval_id(event) else {
            return;
        };
        let entry = self
            .approvals
            .entry(approval_id.to_string())
            .or_insert_with(|| ApprovalAccumulator {
                approval_id: approval_id.to_string(),
                ..ApprovalAccumulator::default()
            });
        entry.required_seq = Some(entry.required_seq.map_or(seq, |current| current.min(seq)));
        if entry.tool.as_deref().unwrap_or_default().is_empty() {
            entry.tool = optional_string(event.get("tool"));
        }
        if entry.checkpoint.is_none() {
            entry.checkpoint = approval_checkpoint(event);
        }
    }

    fn observe_decision(&mut self, seq: usize, event: &Value) {
        let Some(approval_id) = approval_id(event) else {
            return;
        };
        let entry = self
            .approvals
            .entry(approval_id.to_string())
            .or_insert_with(|| ApprovalAccumulator {
                approval_id: approval_id.to_string(),
                ..ApprovalAccumulator::default()
            });
        entry.decision_seq = Some(seq);
        entry.decision = decision_value(event);
        if entry.tool.as_deref().unwrap_or_default().is_empty() {
            entry.tool = optional_string(event.get("tool"));
        }
    }
}

impl TaskApprovalJournalSnapshot {
    pub(crate) fn pending_approval_ids(&self) -> Vec<String> {
        self.approvals
            .iter()
            .filter(|approval| approval.status == "pending")
            .map(|approval| approval.approval_id.clone())
            .collect()
    }

    pub(crate) fn resolve_runtime_state_for_task_status(
        &self,
        active_approval_ids: &[String],
        can_approve_tools: bool,
        task_status: Option<&str>,
    ) -> TaskApprovalStateSnapshot {
        let active: BTreeSet<&str> = active_approval_ids.iter().map(String::as_str).collect();
        let task_terminal = task_status.is_some_and(task_status_is_terminal);
        let approvals: Vec<_> = self
            .approvals
            .iter()
            .map(|approval| approval.resolve(&active, can_approve_tools, task_terminal))
            .collect();
        let actionable_count = approvals.iter().filter(|item| item.actionable).count();
        let decided_count = approvals
            .iter()
            .filter(|item| {
                matches!(
                    item.status,
                    "approved" | "denied" | "processed" | "canceled"
                )
            })
            .count();
        let unavailable_count = approvals
            .iter()
            .filter(|item| matches!(item.status, "unavailable" | "expired" | "closed"))
            .count();
        TaskApprovalStateSnapshot {
            total_count: approvals.len(),
            approvals,
            actionable_count,
            decided_count,
            unavailable_count,
        }
    }
}

impl TaskApprovalJournalItem {
    fn resolve(
        &self,
        active_approval_ids: &BTreeSet<&str>,
        can_approve_tools: bool,
        task_terminal: bool,
    ) -> TaskApprovalStateItem {
        if self.status != "pending" {
            return state_item(self, self.status, false);
        }
        if can_approve_tools && active_approval_ids.contains(self.approval_id.as_str()) {
            return state_item(self, "actionable", true);
        }
        if task_terminal {
            return state_item(self, "closed", false);
        }
        state_item(self, "unavailable", false)
    }
}

impl ApprovalAccumulator {
    fn into_item(self) -> TaskApprovalJournalItem {
        let status = decision_status(self.decision.as_deref());
        TaskApprovalJournalItem {
            approval_id: self.approval_id,
            tool: self.tool,
            status,
            decision: self.decision,
            checkpoint: self.checkpoint,
            required_seq: self.required_seq,
            decision_seq: self.decision_seq,
        }
    }
}

fn state_item(
    approval: &TaskApprovalJournalItem,
    status: &'static str,
    actionable: bool,
) -> TaskApprovalStateItem {
    let policy = state_policy(status);
    TaskApprovalStateItem {
        approval_id: approval.approval_id.clone(),
        tool: approval.tool.clone(),
        status,
        decision: approval.decision.clone(),
        actionable,
        next_action: policy.next_action,
        requires_new_task: policy.requires_new_task,
        checkpoint: approval.checkpoint.clone(),
        label: policy.label,
        tone: policy.tone,
        meta: policy.meta,
        required_seq: approval.required_seq,
        decision_seq: approval.decision_seq,
    }
}

#[derive(Debug, Clone, Copy)]
struct ApprovalStatePolicy {
    label: &'static str,
    tone: &'static str,
    meta: &'static str,
    next_action: &'static str,
    requires_new_task: bool,
}

fn state_policy(status: &str) -> ApprovalStatePolicy {
    match status {
        "actionable" => ApprovalStatePolicy {
            label: "等待确认",
            tone: "approval",
            meta: "可在本机继续审批",
            next_action: "approve_or_deny",
            requires_new_task: false,
        },
        "approved" => ApprovalStatePolicy {
            label: "已批准",
            tone: "done",
            meta: "继续执行工具",
            next_action: "none",
            requires_new_task: false,
        },
        "denied" => ApprovalStatePolicy {
            label: "已拒绝",
            tone: "canceled",
            meta: "工具不会执行",
            next_action: "none",
            requires_new_task: false,
        },
        "expired" => ApprovalStatePolicy {
            label: "已过期",
            tone: "canceled",
            meta: "审批已过期",
            next_action: "continue_from_snapshot",
            requires_new_task: true,
        },
        "canceled" => ApprovalStatePolicy {
            label: "已取消",
            tone: "canceled",
            meta: "任务已停止",
            next_action: "none",
            requires_new_task: false,
        },
        "closed" => ApprovalStatePolicy {
            label: "已关闭",
            tone: "canceled",
            meta: "任务已结束，审批已关闭",
            next_action: "none",
            requires_new_task: false,
        },
        "processed" => ApprovalStatePolicy {
            label: "已处理",
            tone: "done",
            meta: "审批已处理",
            next_action: "none",
            requires_new_task: false,
        },
        _ => ApprovalStatePolicy {
            label: "已失效",
            tone: "failed",
            meta: "本机没有活动审批等待器",
            next_action: "continue_from_snapshot",
            requires_new_task: true,
        },
    }
}

fn task_status_is_terminal(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "finished"
            | "done"
            | "failed"
            | "canceled"
            | "cancelled"
            | "interrupted"
            | "resume_required"
    )
}

fn decision_status(decision: Option<&str>) -> &'static str {
    match decision.map(|value| value.trim().to_ascii_lowercase()) {
        None => "pending",
        Some(value) if matches!(value.as_str(), "approve" | "approved" | "allow" | "allowed") => {
            "approved"
        }
        Some(value)
            if matches!(
                value.as_str(),
                "deny" | "denied" | "reject" | "rejected" | "disallow" | "disallowed"
            ) =>
        {
            "denied"
        }
        Some(value) if matches!(value.as_str(), "timeout" | "expired") => "expired",
        Some(value) if matches!(value.as_str(), "cancel" | "canceled" | "cancelled") => "canceled",
        Some(_) => "processed",
    }
}

fn decision_value(event: &Value) -> Option<String> {
    optional_string(event.get("decision"))
        .or_else(|| optional_string(event.get("status")))
        .or_else(|| optional_string(event.get("result")))
}

fn tool_event_payload(event: &Value) -> Option<&Value> {
    match event.get("type").and_then(Value::as_str) {
        Some("tool_event") => event.get("event"),
        Some("tool_approval_required" | "tool_approval_decision") => Some(event),
        _ => None,
    }
}

fn approval_id(event: &Value) -> Option<&str> {
    event
        .get("approval_id")
        .or_else(|| event.get("approvalId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn approval_checkpoint(event: &Value) -> Option<Value> {
    event
        .get("approval_checkpoint")
        .or_else(|| event.get("approvalCheckpoint"))
        .filter(|value| value.is_object())
        .cloned()
}

fn optional_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn first_seq(approval: &TaskApprovalJournalItem) -> usize {
    approval
        .required_seq
        .or(approval.decision_seq)
        .unwrap_or(usize::MAX)
}

#[cfg(test)]
#[path = "node_agent_task_approval_snapshot_tests.rs"]
mod tests;
