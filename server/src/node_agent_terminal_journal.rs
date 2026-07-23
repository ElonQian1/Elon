use anyhow::Result;
use serde_json::Value;

use crate::node_agent_runtime::NodeRuntime;

pub(crate) fn has_finished_success(runtime: &NodeRuntime, task_id: &str) -> Result<bool> {
    let current_record_is_success = runtime.task_journal.record(task_id)?.is_some_and(|record| {
        matches!(record.status.as_str(), "finished" | "done") && record.phase == "done"
    });
    if current_record_is_success {
        return Ok(true);
    }
    Ok(runtime
        .task_journal
        .task_events(task_id)?
        .iter()
        .any(|event| is_finished_success_event(&event.event)))
}

fn is_finished_success_event(event: &Value) -> bool {
    event.get("type").and_then(Value::as_str) == Some("finished")
        && event
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| matches!(status, "finished" | "done"))
        && event
            .get("error")
            .and_then(Value::as_str)
            .is_none_or(|error| error.trim().is_empty())
}
