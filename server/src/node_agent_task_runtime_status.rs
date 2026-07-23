use serde_json::{json, Value};

use crate::node_agent_task_journal_events::is_terminal_status;

pub(crate) fn payload(record: Option<&crate::node_agent_task_journal::TaskJournalRecord>) -> Value {
    let Some(record) = record else {
        return Value::Null;
    };
    let now = crate::node_agent_cli_sidecar::now_ms();
    let terminal = is_terminal_status(&record.status);
    // `updated_at_ms` is durably written with every journal terminal transition.
    // Freeze every elapsed/idle/phase observation at that timestamp so an old
    // completed task never accrues wall time merely because it is inspected.
    let observed_at_ms = if terminal {
        record.updated_at_ms.max(record.started_at_ms)
    } else {
        now
    };
    let last_progress = record
        .last_progress_ms
        .unwrap_or(record.started_at_ms)
        .min(observed_at_ms);
    let heartbeat = record
        .heartbeat_at_ms
        .unwrap_or(record.updated_at_ms)
        .min(observed_at_ms);
    let elapsed_ms = observed_at_ms.saturating_sub(record.started_at_ms);
    let queue_ms = record
        .process_started_at_ms
        .map(|started| started.saturating_sub(record.started_at_ms));
    let execution_ms = record
        .process_started_at_ms
        .map(|started| observed_at_ms.saturating_sub(started));
    let (total_deadline_ms, idle_deadline_ms, remaining_ms, timeout_reason) =
        match record.timeout_policy.as_ref().filter(|_| !terminal) {
            Some(policy) => {
                let total = record
                    .started_at_ms
                    .saturating_add(policy.total_timeout_secs as u128 * 1_000);
                let idle = last_progress.saturating_add(policy.idle_timeout_secs as u128 * 1_000);
                let effective = total.min(idle);
                let reason = if observed_at_ms >= total {
                    Some("total_timeout")
                } else if observed_at_ms >= idle {
                    Some("idle_timeout")
                } else {
                    None
                };
                (
                    Some(total),
                    Some(idle),
                    Some(effective.saturating_sub(observed_at_ms)),
                    reason,
                )
            }
            None => (None, None, None, None),
        };
    json!({
        "timing_schema": "elon.task_runtime_timing.v2",
        "phase": record.phase,
        "current_command": record.current_command,
        "last_progress": last_progress,
        "heartbeat": heartbeat,
        "idle_duration": observed_at_ms.saturating_sub(last_progress) / 1000,
        "observed_at_ms": observed_at_ms,
        "terminal_at_ms": terminal.then_some(observed_at_ms),
        "terminal_at_source": terminal.then_some("journal_record_updated_at"),
        "elapsed_ms": elapsed_ms,
        "queue_ms": queue_ms,
        "execution_ms": execution_ms,
        "phase_elapsed_ms": observed_at_ms.saturating_sub(last_progress),
        "total_deadline_ms": total_deadline_ms,
        "idle_deadline_ms": idle_deadline_ms,
        "remaining_before_timeout_ms": remaining_ms,
        "eta_kind": remaining_ms.map(|_| "timeout_upper_bound"),
        "timeout_reason": timeout_reason,
        "timeout_policy": (!terminal).then_some(record.timeout_policy.as_ref()).flatten(),
        "dispatch": record.dispatch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        node_agent_cli_runtime_policy::CliRuntimePolicy, node_agent_task_journal::TaskJournalRecord,
    };

    fn record(status: &str) -> TaskJournalRecord {
        TaskJournalRecord {
            req_id: format!("task-{status}"),
            cli_name: "codex".to_string(),
            route: Some("route_a_external_cli".to_string()),
            run_handle_id: Some(format!("task-{status}")),
            cwd: Some("D:/isolated".to_string()),
            runtime_permission: Some("full_access".to_string()),
            os_pid: None,
            process_started_at_ms: Some(2_000),
            process_identity: None,
            codex_session_id: None,
            codex_session_scope_key: None,
            codex_session_updated_at_ms: None,
            status: status.to_string(),
            phase: status.to_string(),
            current_command: None,
            last_progress_ms: Some(4_000),
            heartbeat_at_ms: Some(4_500),
            timeout_policy: Some(CliRuntimePolicy::fixed(10)),
            dispatch: None,
            started_at_ms: 1_000,
            updated_at_ms: 5_000,
            cancel_requested_at_ms: None,
            cancel_intent: None,
        }
    }

    #[test]
    fn all_public_terminal_states_freeze_at_the_durable_terminal_time() {
        for status in ["done", "failed", "canceled", "resume_required"] {
            let runtime = payload(Some(&record(status)));
            assert_eq!(runtime["terminal_at_ms"], 5_000);
            assert_eq!(runtime["elapsed_ms"], 4_000);
            assert_eq!(runtime["execution_ms"], 3_000);
            assert_eq!(runtime["idle_duration"], 1);
            assert_eq!(runtime["phase_elapsed_ms"], 1_000);
            assert!(runtime["timeout_policy"].is_null());
            assert!(runtime["total_deadline_ms"].is_null());
            assert!(runtime["remaining_before_timeout_ms"].is_null());
            assert!(runtime["timeout_reason"].is_null());
        }
    }

    #[test]
    fn stale_running_state_remains_live_until_recovery_marks_a_terminal_state() {
        let runtime = payload(Some(&record("running")));
        assert!(runtime["terminal_at_ms"].is_null());
        assert!(runtime["timeout_policy"].is_object());
        assert!(runtime["elapsed_ms"].as_u64().unwrap_or_default() > 4_000);
    }
}
