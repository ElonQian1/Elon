use super::TaskApprovalJournalTracker;
use serde_json::json;

#[test]
fn records_required_and_decision_state() {
    let mut tracker = TaskApprovalJournalTracker::default();
    tracker.observe_event(
        3,
        &json!({
            "type": "tool_event",
            "event": {
                "type": "tool_approval_required",
                "approval_id": "tap_1_1",
                "tool": "write_file"
            }
        }),
    );
    tracker.observe_event(
        7,
        &json!({
            "type": "tool_event",
            "event": {
                "type": "tool_approval_decision",
                "approval_id": "tap_1_1",
                "tool": "write_file",
                "decision": "approve"
            }
        }),
    );

    let snapshot = tracker.finish();
    assert_eq!(snapshot.pending_count, 0);
    assert_eq!(snapshot.decided_count, 1);
    assert_eq!(snapshot.approvals[0].status, "approved");
    assert_eq!(snapshot.approvals[0].required_seq, Some(3));
    assert_eq!(snapshot.approvals[0].decision_seq, Some(7));
}

#[test]
fn pending_approval_resolves_to_actionable_when_waiter_is_live() {
    let mut tracker = TaskApprovalJournalTracker::default();
    tracker.observe_event(
        1,
        &json!({
            "type": "tool_approval_required",
            "approval_id": "tap_1_1",
            "tool": "run_command"
        }),
    );

    let state = tracker.finish().resolve_runtime_state_for_task_status(
        &["tap_1_1".to_string()],
        true,
        None,
    );
    assert_eq!(state.actionable_count, 1);
    assert_eq!(state.approvals[0].status, "actionable");
    assert!(state.approvals[0].actionable);
    assert_eq!(state.approvals[0].next_action, "approve_or_deny");
    assert!(!state.approvals[0].requires_new_task);
}

#[test]
fn pending_approval_resolves_to_unavailable_without_live_waiter() {
    let mut tracker = TaskApprovalJournalTracker::default();
    tracker.observe_event(
        1,
        &json!({
            "type": "tool_approval_required",
            "approval_id": "tap_1_1",
            "tool": "run_command"
        }),
    );

    let state = tracker
        .finish()
        .resolve_runtime_state_for_task_status(&[], false, None);
    assert_eq!(state.unavailable_count, 1);
    assert_eq!(state.approvals[0].status, "unavailable");
    assert!(!state.approvals[0].actionable);
    assert_eq!(state.approvals[0].next_action, "continue_from_snapshot");
    assert!(state.approvals[0].requires_new_task);
}

#[test]
fn required_approval_keeps_restart_checkpoint() {
    let mut tracker = TaskApprovalJournalTracker::default();
    tracker.observe_event(
        1,
        &json!({
            "type": "tool_approval_required",
            "approval_id": "tap_1_1",
            "tool": "write_file",
            "approval_checkpoint": {
                "schema": "elon.routebc.tool_approval_checkpoint.v1",
                "registered_at_ms": 10,
                "expires_at_ms": 20,
                "action_sha256": "a".repeat(64),
                "restart_recovery": {
                    "supported": false,
                    "next_action": "continue_from_snapshot"
                }
            }
        }),
    );

    let snapshot = tracker.finish();
    assert_eq!(
        snapshot.approvals[0].checkpoint.as_ref().unwrap()["schema"],
        "elon.routebc.tool_approval_checkpoint.v1"
    );

    let state = snapshot.resolve_runtime_state_for_task_status(&[], false, None);
    let checkpoint = state.approvals[0]
        .checkpoint
        .as_ref()
        .expect("checkpoint should be exposed to local task journal API");
    assert_eq!(checkpoint["expires_at_ms"], 20);
    assert_eq!(
        checkpoint["restart_recovery"]["next_action"],
        "continue_from_snapshot"
    );
    assert_eq!(state.approvals[0].next_action, "continue_from_snapshot");
}

#[test]
fn pending_approval_ids_only_include_undecided_items() {
    let mut tracker = TaskApprovalJournalTracker::default();
    tracker.observe_event(
        1,
        &json!({
            "type": "tool_approval_required",
            "approval_id": "tap_pending",
            "tool": "run_command"
        }),
    );
    tracker.observe_event(
        2,
        &json!({
            "type": "tool_approval_required",
            "approval_id": "tap_approved",
            "tool": "write_file"
        }),
    );
    tracker.observe_event(
        3,
        &json!({
            "type": "tool_approval_decision",
            "approval_id": "tap_approved",
            "decision": "approve"
        }),
    );

    let snapshot = tracker.finish();

    assert_eq!(snapshot.pending_approval_ids(), vec!["tap_pending"]);
    assert_eq!(snapshot.pending_count, 1);
    assert_eq!(snapshot.decided_count, 1);
}

#[test]
fn pending_approval_resolves_to_closed_when_task_is_terminal() {
    let mut tracker = TaskApprovalJournalTracker::default();
    tracker.observe_event(
        1,
        &json!({
            "type": "tool_approval_required",
            "approval_id": "tap_1_1",
            "tool": "apply_patch"
        }),
    );

    let state = tracker
        .finish()
        .resolve_runtime_state_for_task_status(&[], false, Some("failed"));
    assert_eq!(state.decided_count, 0);
    assert_eq!(state.unavailable_count, 1);
    assert_eq!(state.approvals[0].status, "closed");
    assert_eq!(state.approvals[0].label, "已关闭");
    assert_eq!(state.approvals[0].meta, "任务已结束，审批已关闭");
    assert_eq!(state.approvals[0].next_action, "none");
    assert!(!state.approvals[0].requires_new_task);
    assert!(!state.approvals[0].actionable);
}
