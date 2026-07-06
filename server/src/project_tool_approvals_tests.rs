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
