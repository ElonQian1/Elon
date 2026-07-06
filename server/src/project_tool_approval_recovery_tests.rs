    use super::recover_from_task_events;
    use crate::project_tool_approvals::{
        claim_decision_target, clear_task, ToolApprovalClaim, ToolApprovalErrorKind,
    };

    #[test]
    fn recovers_pending_approval_from_task_events() {
        let task_id = "task-recover-pending-approval";
        clear_task(task_id);
        let events = vec![serde_json::json!({
            "type": "tool_approval_required",
            "req_id": "req",
            "approval_id": "tap_1_1",
            "tool": "apply_patch",
            "status": "pending"
        })
        .to_string()];

        assert_eq!(
            recover_from_task_events("project", "channel", task_id, &events),
            1
        );
        let claim =
            claim_decision_target("project", "channel", task_id, "tap_1_1", "approve").unwrap();
        assert!(matches!(claim, ToolApprovalClaim::Dispatch(_)));
        clear_task(task_id);
    }

    #[test]
    fn recovered_decision_prevents_old_button_replay() {
        let task_id = "task-recover-decided-approval";
        clear_task(task_id);
        let events = vec![
            serde_json::json!({
                "type": "tool_approval_required",
                "req_id": "req",
                "approval_id": "tap_1_1",
                "tool": "write_file",
                "status": "pending"
            })
            .to_string(),
            serde_json::json!({
                "type": "tool_approval_decision",
                "req_id": "req",
                "approval_id": "tap_1_1",
                "tool": "write_file",
                "decision": "approve",
                "status": "approved"
            })
            .to_string(),
        ];

        assert_eq!(
            recover_from_task_events("project", "channel", task_id, &events),
            2
        );
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
    fn recovered_timeout_keeps_approval_closed() {
        let task_id = "task-recover-expired-approval";
        clear_task(task_id);
        let events = vec![
            serde_json::json!({
                "type": "tool_approval_required",
                "req_id": "req",
                "approval_id": "tap_1_1",
                "tool": "run_command"
            })
            .to_string(),
            serde_json::json!({
                "type": "tool_approval_decision",
                "req_id": "req",
                "approval_id": "tap_1_1",
                "tool": "run_command",
                "decision": "timeout",
                "status": "expired"
            })
            .to_string(),
        ];

        assert_eq!(
            recover_from_task_events("project", "channel", task_id, &events),
            2
        );
        let err =
            claim_decision_target("project", "channel", task_id, "tap_1_1", "approve").unwrap_err();
        assert_eq!(err.kind(), ToolApprovalErrorKind::Conflict);
        clear_task(task_id);
    }
