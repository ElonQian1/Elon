//! Durable, non-overstated performance segmentation for supervised CLI tasks.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::{
    node_agent_task_journal::{TaskJournal, TaskJournalEventView, TaskJournalRecord},
    node_agent_task_journal_events::is_terminal_status,
};

pub(crate) fn payload(journal: &TaskJournal, record: Option<&TaskJournalRecord>) -> Value {
    let Some(record) = record else {
        return Value::Null;
    };
    match journal.task_events(&record.req_id) {
        Ok(events) => build(record, &events),
        Err(error) => json!({
            "schema": "elon.supervision_performance_timing.v1",
            "available": false,
            "error": error.to_string(),
        }),
    }
}

fn build(record: &TaskJournalRecord, events: &[TaskJournalEventView]) -> Value {
    let terminal = is_terminal_status(&record.status);
    // Use only persisted journal observations. Active tasks advance this bound
    // through durable progress/heartbeat writes; a read itself never invents
    // execution time and heartbeat-only gaps stay in the unattributed bucket.
    let observed_at_ms = record.updated_at_ms.max(record.started_at_ms);
    let contract_at_ms = first_event_at(events, |kind, _| kind == "supervision_contract");
    let submitted_at_ms = contract_at_ms.unwrap_or(record.started_at_ms);
    let accepted_at_ms =
        first_event_at(events, |kind, _| kind == "started").unwrap_or(record.started_at_ms);
    let process_starts = event_times(events, |kind, _| kind == "process_started");
    let process_started_at_ms = process_starts
        .first()
        .copied()
        .or(record.process_started_at_ms);
    let latest_process_started_at_ms = process_starts
        .last()
        .copied()
        .or(record.process_started_at_ms);
    let first_structured_at_ms = first_event_at(events, |kind, _| kind == "codex_item");
    let first_effective_at_ms = first_event_at(events, effective_progress_event);
    let review_at_ms = first_event_at(events, |kind, _| kind == "supervision_review");

    let active_intervals = structured_item_intervals(events, observed_at_ms);
    let recovery_intervals = recovery_intervals(events, observed_at_ms);
    let active_execution_ms = merged_duration(&active_intervals);
    let recovery_ms = merged_duration(&recovery_intervals);
    let mut accounted = active_intervals.clone();
    accounted.extend(recovery_intervals.iter().copied());
    let process_wall_ms =
        process_started_at_ms.map(|started| observed_at_ms.saturating_sub(started));
    let accounted_ms = merged_duration(&accounted);
    let external_or_unattributed_wait_ms =
        process_wall_ms.map(|wall| wall.saturating_sub(accounted_ms));
    let repeat_dispatch_ms = match (process_started_at_ms, latest_process_started_at_ms) {
        (Some(first), Some(last)) if process_starts.len() > 1 => Some(last.saturating_sub(first)),
        _ => None,
    };

    json!({
        "schema": "elon.supervision_performance_timing.v1",
        "available": true,
        "status": record.status,
        "terminal": terminal,
        "submitted_at_ms": submitted_at_ms,
        "accepted_at_ms": accepted_at_ms,
        "process_started_at_ms": process_started_at_ms,
        "first_structured_output_at_ms": first_structured_at_ms,
        "first_effective_progress_at_ms": first_effective_at_ms,
        "terminal_at_ms": terminal.then_some(observed_at_ms),
        "supervisor_review_at_ms": review_at_ms,
        "segments": {
            "queue_ms": accepted_at_ms.saturating_sub(submitted_at_ms),
            "cli_startup_ms": process_started_at_ms.map(|at| at.saturating_sub(accepted_at_ms)),
            "submit_to_process_start_ms": process_started_at_ms.map(|at| at.saturating_sub(submitted_at_ms)),
            "submit_to_first_structured_output_ms": first_structured_at_ms.map(|at| at.saturating_sub(submitted_at_ms)),
            "submit_to_first_effective_progress_ms": first_effective_at_ms.map(|at| at.saturating_sub(submitted_at_ms)),
            "active_execution_ms": active_execution_ms,
            "recovery_ms": recovery_ms,
            "process_wall_ms": process_wall_ms,
            "external_or_unattributed_wait_ms": external_or_unattributed_wait_ms,
            "repeat_dispatch_ms": repeat_dispatch_ms,
            "submit_to_terminal_ms": terminal.then_some(observed_at_ms.saturating_sub(submitted_at_ms)),
            "supervisor_review_wait_ms": review_at_ms
                .filter(|_| terminal)
                .map(|at| at.saturating_sub(observed_at_ms)),
        },
        "counts": {
            "process_starts": process_starts.len(),
            "repeat_dispatches": process_starts.len().saturating_sub(1),
            "structured_item_intervals": active_intervals.len(),
            "recovery_intervals": recovery_intervals.len(),
        },
        "accounting": {
            "heartbeat_wait_counted_as_active_execution": false,
            "active_execution_definition": "union of durable codex_item started/completed intervals",
            "recovery_definition": "union of durable recovery markers until process/progress/terminal transition",
            "external_or_unattributed_wait_definition": "process wall time not backed by active item or recovery intervals; includes model, provider, and idle waits",
            "clock": "persisted journal epoch milliseconds only",
        },
        "dispatch": record.dispatch,
    })
}

fn event_at(view: &TaskJournalEventView) -> Option<u128> {
    view.event
        .get("at_ms")
        .and_then(Value::as_u64)
        .map(u128::from)
}

fn first_event_at(
    events: &[TaskJournalEventView],
    predicate: impl Fn(&str, &Value) -> bool,
) -> Option<u128> {
    events.iter().find_map(|view| {
        let kind = view.event.get("type").and_then(Value::as_str)?;
        predicate(kind, &view.event)
            .then(|| event_at(view))
            .flatten()
    })
}

fn event_times(
    events: &[TaskJournalEventView],
    predicate: impl Fn(&str, &Value) -> bool,
) -> Vec<u128> {
    events
        .iter()
        .filter_map(|view| {
            let kind = view.event.get("type").and_then(Value::as_str)?;
            predicate(kind, &view.event)
                .then(|| event_at(view))
                .flatten()
        })
        .collect()
}

fn effective_progress_event(kind: &str, event: &Value) -> bool {
    if kind == "tool_event" {
        return event.pointer("/event/type").and_then(Value::as_str) == Some("tool_result");
    }
    if kind != "codex_item" || event.get("lifecycle").and_then(Value::as_str) != Some("completed") {
        return false;
    }
    event.pointer("/item/status").and_then(Value::as_str) != Some("in_progress")
}

fn structured_item_intervals(
    events: &[TaskJournalEventView],
    observed_at_ms: u128,
) -> Vec<(u128, u128)> {
    let mut starts = BTreeMap::<String, u128>::new();
    let mut intervals = Vec::new();
    for view in events {
        if view.event.get("type").and_then(Value::as_str) != Some("codex_item") {
            continue;
        }
        let Some(id) = view.event.pointer("/item/id").and_then(Value::as_str) else {
            continue;
        };
        let Some(at_ms) = event_at(view) else {
            continue;
        };
        match view.event.get("lifecycle").and_then(Value::as_str) {
            Some("started") => {
                starts.entry(id.to_string()).or_insert(at_ms);
            }
            Some("completed") => {
                if let Some(started) = starts.remove(id) {
                    intervals.push((started, at_ms.max(started)));
                }
            }
            _ => {}
        }
    }
    intervals.extend(
        starts
            .into_values()
            .map(|started| (started, observed_at_ms.max(started))),
    );
    intervals
}

fn recovery_intervals(events: &[TaskJournalEventView], observed_at_ms: u128) -> Vec<(u128, u128)> {
    let mut started = None;
    let mut intervals = Vec::new();
    for view in events {
        let Some(kind) = view.event.get("type").and_then(Value::as_str) else {
            continue;
        };
        let Some(at_ms) = event_at(view) else {
            continue;
        };
        let recovery_marker = kind.contains("recovery") || kind.contains("resume_required");
        if recovery_marker {
            started.get_or_insert(at_ms);
            continue;
        }
        if started.is_some()
            && (kind == "process_started"
                || kind == "finished"
                || effective_progress_event(kind, &view.event))
        {
            intervals.push((started.take().unwrap_or(at_ms), at_ms));
        }
    }
    if let Some(started) = started {
        intervals.push((started, observed_at_ms.max(started)));
    }
    intervals
}

fn merged_duration(intervals: &[(u128, u128)]) -> u128 {
    let mut intervals = intervals.to_vec();
    intervals.sort_unstable();
    let mut total = 0u128;
    let mut current: Option<(u128, u128)> = None;
    for (start, end) in intervals {
        match current {
            Some((current_start, current_end)) if start <= current_end => {
                current = Some((current_start, current_end.max(end)));
            }
            Some((current_start, current_end)) => {
                total = total.saturating_add(current_end.saturating_sub(current_start));
                current = Some((start, end));
            }
            None => current = Some((start, end)),
        }
    }
    if let Some((start, end)) = current {
        total = total.saturating_add(end.saturating_sub(start));
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_task_journal::{TaskJournal, TaskJournalStart};

    fn event(seq: usize, value: Value) -> TaskJournalEventView {
        TaskJournalEventView { seq, event: value }
    }

    #[test]
    fn terminal_segments_freeze_and_heartbeat_is_not_active_execution() {
        let root = std::env::temp_dir().join(format!(
            "elon-performance-timing-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let journal = TaskJournal::new(&root);
        journal
            .record_started(TaskJournalStart {
                req_id: "task",
                cli_name: "codex",
                route: Some("route_a_external_cli"),
                run_handle_id: Some("task"),
                cwd: Some("D:/demo"),
                runtime_permission: Some("full_access"),
            })
            .unwrap();
        journal.record_process_started("task", 42).unwrap();
        journal.record_runtime_heartbeat("task").unwrap();
        journal
            .record_finished_with_outcome("task", "done", None)
            .unwrap();
        let record = journal.record("task").unwrap().unwrap();
        let value = build(&record, &journal.task_events("task").unwrap());
        assert_eq!(value["terminal"], true);
        assert_eq!(value["segments"]["active_execution_ms"], 0);
        assert_eq!(
            value["accounting"]["heartbeat_wait_counted_as_active_execution"],
            false
        );
        assert!(value["terminal_at_ms"].as_u64().is_some());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn queue_startup_recovery_repeat_execution_and_review_are_independent() {
        let root = std::env::temp_dir().join(format!(
            "elon-performance-segments-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let journal = TaskJournal::new(&root);
        journal
            .record_started(TaskJournalStart {
                req_id: "task",
                cli_name: "codex",
                route: Some("route_a_external_cli"),
                run_handle_id: Some("task"),
                cwd: Some("D:/demo"),
                runtime_permission: Some("full_access"),
            })
            .unwrap();
        let mut record = journal.record("task").unwrap().unwrap();
        record.started_at_ms = 1_000;
        record.process_started_at_ms = Some(3_000);
        record.updated_at_ms = 10_000;
        record.status = "done".to_string();
        let events = vec![
            event(1, json!({"type":"supervision_contract","at_ms":900})),
            event(2, json!({"type":"started","at_ms":1_000})),
            event(3, json!({"type":"process_started","at_ms":3_000})),
            event(4, json!({"type":"recovery_running","at_ms":3_500})),
            event(5, json!({"type":"process_started","at_ms":4_500})),
            event(
                6,
                json!({
                    "type":"codex_item","lifecycle":"started","at_ms":5_000,
                    "item":{"id":"command-1","type":"command_execution","status":"in_progress"}
                }),
            ),
            event(
                7,
                json!({
                    "type":"codex_item","lifecycle":"completed","at_ms":6_000,
                    "item":{"id":"command-1","type":"command_execution","status":"completed"}
                }),
            ),
            event(8, json!({"type":"finished","status":"done","at_ms":10_000})),
            event(9, json!({"type":"supervision_review","at_ms":11_000})),
        ];
        let value = build(&record, &events);
        assert_eq!(value["segments"]["queue_ms"], 100);
        assert_eq!(value["segments"]["cli_startup_ms"], 2_000);
        assert_eq!(value["segments"]["submit_to_process_start_ms"], 2_100);
        assert_eq!(value["segments"]["active_execution_ms"], 1_000);
        assert_eq!(value["segments"]["recovery_ms"], 1_000);
        assert_eq!(value["segments"]["repeat_dispatch_ms"], 1_500);
        assert_eq!(value["segments"]["external_or_unattributed_wait_ms"], 5_000);
        assert_eq!(value["segments"]["supervisor_review_wait_ms"], 1_000);
        assert_eq!(value["counts"]["repeat_dispatches"], 1);
        let _ = std::fs::remove_dir_all(root);
    }
}
