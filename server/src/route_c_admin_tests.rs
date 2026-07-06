    use super::{route_c_operation_action_items, RouteCBudgetEventRow, RouteCBudgetOutcomeSummary};
    use crate::server_agent_runtime_budget::ServerRuntimeBudgetStatus;

    fn budget_status(status: &'static str) -> ServerRuntimeBudgetStatus {
        ServerRuntimeBudgetStatus {
            enabled: true,
            status,
            source: "test",
            used_calls_today: 100,
            daily_call_limit: Some(100),
            remaining_calls_today: Some(0),
            per_user_enabled: true,
            per_user_source: "test",
            used_calls_today_for_user: Some(3),
            per_user_daily_call_limit: Some(3),
            remaining_calls_today_for_user: Some(0),
            reset_after_secs: 3600,
        }
    }

    fn outcome(outcome: &str, completed_calls: i64) -> RouteCBudgetOutcomeSummary {
        RouteCBudgetOutcomeSummary {
            outcome: outcome.to_string(),
            total_calls: completed_calls,
            completed_calls,
            pending_calls: 0,
            stale_pending_calls: 0,
            unique_users: 1,
            total_tokens: None,
            first_created_at: None,
            last_created_at: None,
        }
    }

    fn stale_event() -> RouteCBudgetEventRow {
        RouteCBudgetEventRow {
            id: "evt_1".to_string(),
            user_id: "user_1".to_string(),
            request_fingerprint: "fp_1".to_string(),
            route_day: "2026-06-24".to_string(),
            created_at: "2026-06-24T00:00:00+00:00".to_string(),
            outcome: "admitted".to_string(),
            completed_at: None,
            model: None,
            total_tokens: None,
            error_summary: Some("secret prompt text should not leave event rows".to_string()),
        }
    }

    #[test]
    fn route_c_operation_action_items_surface_budget_and_stale_pending() {
        let actions = route_c_operation_action_items(
            &budget_status("exhausted"),
            &[outcome("provider_error", 2)],
            &[stale_event()],
        );
        let kinds = actions.iter().map(|item| item.kind).collect::<Vec<_>>();

        assert_eq!(
            kinds,
            vec!["budget_exhausted", "stale_pending_calls", "provider_errors"]
        );
        assert_eq!(actions[0].severity, "blocker");
        assert_eq!(actions[1].count, 1);
        assert_eq!(actions[2].count, 2);

        let serialized = serde_json::to_string(&actions).unwrap();
        assert!(!serialized.contains("secret prompt text"));
        assert!(!serialized.contains("fp_1"));
        assert!(!serialized.contains("user_1"));
    }

    #[test]
    fn route_c_operation_action_items_warn_when_budget_is_low() {
        let mut status = budget_status("available");
        status.used_calls_today = 95;
        status.remaining_calls_today = Some(5);

        let actions = route_c_operation_action_items(&status, &[], &[]);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].kind, "budget_low");
        assert_eq!(actions[0].severity, "warning");
        assert_eq!(actions[0].count, 5);
    }
