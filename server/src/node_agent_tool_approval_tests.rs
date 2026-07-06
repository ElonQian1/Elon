    use super::{ToolApprovalDecision, ToolApprovalState};

    #[tokio::test]
    async fn decision_wakes_registered_waiter() {
        let state = ToolApprovalState::default();
        let mut waiter = state.register("req", "tap_1_1").await;

        assert!(state.decide("req", "tap_1_1", "approve").await);
        assert!(waiter.changed().await);
        assert_eq!(waiter.decision(), Some(ToolApprovalDecision::Approve));
    }

    #[tokio::test]
    async fn unknown_or_invalid_decision_is_rejected() {
        let state = ToolApprovalState::default();
        let _waiter = state.register("req", "tap_1_1").await;

        assert!(!state.decide("req", "tap_1_2", "approve").await);
        assert!(!state.decide("req", "tap_1_1", "maybe").await);
    }

    #[tokio::test]
    async fn duplicate_decision_is_rejected_after_first_consume() {
        let state = ToolApprovalState::default();
        let _waiter = state.register("req", "tap_1_1").await;

        assert!(state.decide("req", "tap_1_1", "approve").await);
        assert!(!state.decide("req", "tap_1_1", "approve").await);
    }

    #[tokio::test]
    async fn pending_for_req_lists_only_live_waiters() {
        let state = ToolApprovalState::default();
        let first = state.register("req", "tap_1_1").await;
        let _other = state.register("other", "tap_2_1").await;

        let pending = state.pending_for_req("req").await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].approval_id, "tap_1_1");
        assert_eq!(pending[0].registered_at_ms, first.registered_at_ms());
        assert_eq!(pending[0].expires_at_ms, first.expires_at_ms());
        assert!(pending[0].expires_at_ms > pending[0].registered_at_ms);

        assert!(state.decide("req", "tap_1_1", "deny").await);
        assert!(state.pending_for_req("req").await.is_empty());
    }

    #[tokio::test]
    async fn clear_req_removes_only_matching_waiters() {
        let state = ToolApprovalState::default();
        let mut first = state.register("req", "tap_1_1").await;
        let mut second = state.register("req", "tap_1_2").await;
        let mut other = state.register("other", "tap_2_1").await;

        assert_eq!(state.clear_req("req").await, 2);
        assert!(state.pending_for_req("req").await.is_empty());
        assert_eq!(state.pending_for_req("other").await.len(), 1);
        assert!(!state.decide("req", "tap_1_1", "approve").await);
        assert!(!first.changed().await);
        assert!(!second.changed().await);

        assert!(state.decide("other", "tap_2_1", "approve").await);
        assert!(other.changed().await);
        assert_eq!(other.decision(), Some(ToolApprovalDecision::Approve));
    }
