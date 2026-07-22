use serde_json::{json, Value};

pub(crate) fn payload(record: Option<&crate::node_agent_task_journal::TaskJournalRecord>) -> Value {
    let Some(record) = record else {
        return Value::Null;
    };
    let now = crate::node_agent_cli_sidecar::now_ms();
    let last_progress = record.last_progress_ms.unwrap_or(record.started_at_ms);
    let heartbeat = record.heartbeat_at_ms.unwrap_or(record.updated_at_ms);
    let elapsed_ms = now.saturating_sub(record.started_at_ms);
    let queue_ms = record
        .process_started_at_ms
        .map(|started| started.saturating_sub(record.started_at_ms));
    let execution_ms = record
        .process_started_at_ms
        .map(|started| now.saturating_sub(started));
    let (total_deadline_ms, idle_deadline_ms, remaining_ms, timeout_reason) =
        match record.timeout_policy.as_ref() {
            Some(policy) => {
                let total = record
                    .started_at_ms
                    .saturating_add(policy.total_timeout_secs as u128 * 1_000);
                let idle = last_progress.saturating_add(policy.idle_timeout_secs as u128 * 1_000);
                let effective = total.min(idle);
                let reason = if now >= total {
                    Some("total_timeout")
                } else if now >= idle {
                    Some("idle_timeout")
                } else {
                    None
                };
                (
                    Some(total),
                    Some(idle),
                    Some(effective.saturating_sub(now)),
                    reason,
                )
            }
            None => (None, None, None, None),
        };
    json!({
        "timing_schema": "elon.task_runtime_timing.v1",
        "phase": record.phase,
        "current_command": record.current_command,
        "last_progress": last_progress,
        "heartbeat": heartbeat,
        "idle_duration": now.saturating_sub(last_progress) / 1000,
        "elapsed_ms": elapsed_ms,
        "queue_ms": queue_ms,
        "execution_ms": execution_ms,
        "phase_elapsed_ms": now.saturating_sub(last_progress),
        "total_deadline_ms": total_deadline_ms,
        "idle_deadline_ms": idle_deadline_ms,
        "remaining_before_timeout_ms": remaining_ms,
        "eta_kind": remaining_ms.map(|_| "timeout_upper_bound"),
        "timeout_reason": timeout_reason,
        "timeout_policy": record.timeout_policy,
        "dispatch": record.dispatch,
    })
}
