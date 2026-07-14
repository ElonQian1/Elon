// server/src/project_tool_approval_recovery.rs

use serde_json::Value;

use crate::project_tool_approvals;

pub(crate) fn recover_from_task_events(
    project_id: &str,
    channel_id: &str,
    task_id: &str,
    events: &[String],
) -> usize {
    let mut recovered = 0usize;
    for raw in events {
        let Ok(value) = serde_json::from_str::<Value>(raw) else {
            continue;
        };
        let Some(event_type) = value.get("type").and_then(Value::as_str) else {
            continue;
        };
        let ok = match event_type {
            "tool_approval_required" => {
                project_tool_approvals::register_required(project_id, channel_id, task_id, &value)
            }
            "tool_approval_decision" => project_tool_approvals::register_decision_event(
                project_id, channel_id, task_id, &value,
            ),
            _ => false,
        };
        if ok {
            recovered += 1;
        }
    }
    recovered
}

#[cfg(test)]
#[path = "project_tool_approval_recovery_tests.rs"]
mod tests;
