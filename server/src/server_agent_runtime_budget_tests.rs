    use super::{
        route_day_from_epoch_secs, status_for_used_calls, unavailable_status,
        ServerRuntimeBudgetConfig, DAILY_CALL_LIMIT_ENV, PER_USER_DAILY_CALL_LIMIT_ENV,
        SECONDS_PER_DAY,
    };

    #[test]
    fn budget_status_defaults_to_unlimited_but_preserves_usage_visibility() {
        let config = ServerRuntimeBudgetConfig::from_lookup(|_| None);
        let status = status_for_used_calls(config, 2, None, 10);

        assert!(!status.enabled);
        assert_eq!(status.status, "unlimited");
        assert_eq!(status.used_calls_today, 2);
        assert_eq!(status.remaining_calls_today, None);
        assert!(!status.per_user_enabled);
        assert_eq!(status.used_calls_today_for_user, None);
    }

    #[test]
    fn budget_status_reports_exhausted_operator_daily_call_limit() {
        let config = ServerRuntimeBudgetConfig::from_lookup(|name| {
            (name == DAILY_CALL_LIMIT_ENV).then(|| "2".to_string())
        });
        let available = status_for_used_calls(config, 1, Some(1), 10);
        let exhausted = status_for_used_calls(config, 2, Some(2), 11);

        assert!(available.ready());
        assert_eq!(available.remaining_calls_today, Some(1));
        assert_eq!(exhausted.status, "exhausted");
        assert_eq!(exhausted.remaining_calls_today, Some(0));
        assert!(!exhausted.ready());
    }

    #[test]
    fn budget_status_reports_exhausted_per_user_daily_call_limit() {
        let config = ServerRuntimeBudgetConfig::from_lookup(|name| {
            (name == PER_USER_DAILY_CALL_LIMIT_ENV).then(|| "1".to_string())
        });
        let available = status_for_used_calls(config, 3, Some(0), 10);
        let exhausted = status_for_used_calls(config, 3, Some(1), 11);

        assert!(available.enabled);
        assert!(available.per_user_enabled);
        assert_eq!(available.status, "available");
        assert_eq!(available.remaining_calls_today, None);
        assert_eq!(available.remaining_calls_today_for_user, Some(1));
        assert_eq!(exhausted.status, "user_exhausted");
        assert_eq!(exhausted.used_calls_today_for_user, Some(1));
        assert_eq!(exhausted.remaining_calls_today_for_user, Some(0));
        assert!(!exhausted.ready());
    }

    #[test]
    fn route_day_uses_utc_day_boundary() {
        assert_eq!(route_day_from_epoch_secs(SECONDS_PER_DAY - 1), "1970-01-01");
        assert_eq!(route_day_from_epoch_secs(SECONDS_PER_DAY), "1970-01-02");
    }

    #[test]
    fn unavailable_budget_status_blocks_when_operator_limit_is_configured() {
        let config = ServerRuntimeBudgetConfig::from_lookup(|name| {
            (name == DAILY_CALL_LIMIT_ENV).then(|| "1".to_string())
        });
        let status = unavailable_status(config, 10);

        assert_eq!(status.status, "unavailable");
        assert!(!status.ready());
        assert!(status.enabled);
        assert_eq!(status.daily_call_limit, Some(1));
    }

    #[test]
    fn budget_ignores_invalid_operator_limit_values() {
        for raw in ["", "0", "not-a-number"] {
            let config = ServerRuntimeBudgetConfig::from_lookup(|name| {
                (name == DAILY_CALL_LIMIT_ENV).then(|| raw.to_string())
            });
            let status = status_for_used_calls(config, 0, None, 10);

            assert!(!status.enabled);
            assert_eq!(status.status, "unlimited");
            assert_eq!(status.remaining_calls_today, None);
        }
    }

    #[test]
    fn budget_ignores_invalid_per_user_limit_values() {
        for raw in ["", "0", "not-a-number"] {
            let config = ServerRuntimeBudgetConfig::from_lookup(|name| {
                (name == PER_USER_DAILY_CALL_LIMIT_ENV).then(|| raw.to_string())
            });
            let status = status_for_used_calls(config, 0, Some(0), 10);

            assert!(!status.enabled);
            assert_eq!(status.status, "unlimited");
            assert!(!status.per_user_enabled);
            assert_eq!(status.remaining_calls_today_for_user, None);
        }
    }
