    use super::*;
    use uuid::Uuid;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon_route_c_budget_{}.db",
            Uuid::new_v4().simple()
        ));
        (Store::open(&path).expect("store should open"), path)
    }

    fn recorded_event_id(
        result: RouteCBudgetRecordResult,
        total_used: usize,
        user_used: usize,
    ) -> String {
        match result {
            RouteCBudgetRecordResult::Recorded {
                event_id,
                total_used: actual_total,
                user_used: actual_user,
            } => {
                assert_eq!(actual_total, total_used);
                assert_eq!(actual_user, user_used);
                assert!(!event_id.trim().is_empty());
                event_id
            }
            other => panic!("expected recorded budget event, got {other:?}"),
        }
    }

    fn set_budget_created_at(store: &Store, event_id: &str, created_at: &str) {
        store
            .conn()
            .unwrap()
            .execute(
                "UPDATE route_c_runtime_budget_events SET created_at = ?1 WHERE id = ?2",
                rusqlite::params![created_at, event_id],
            )
            .unwrap();
    }

    #[test]
    fn route_c_budget_records_and_blocks_platform_daily_limit() {
        let (store, path) = temp_store();
        let user = store
            .create_user(
                &format!("route-c-budget-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();

        recorded_event_id(
            store
                .route_c_budget_try_record_call(&user.id, "fp-1", "2026-06-23", Some(2), None)
                .unwrap(),
            1,
            1,
        );
        recorded_event_id(
            store
                .route_c_budget_try_record_call(&user.id, "fp-2", "2026-06-23", Some(2), None)
                .unwrap(),
            2,
            2,
        );
        assert_eq!(
            store
                .route_c_budget_try_record_call(&user.id, "fp-3", "2026-06-23", Some(2), None)
                .unwrap(),
            RouteCBudgetRecordResult::PlatformLimitReached {
                total_used: 2,
                user_used: 2
            }
        );
        assert_eq!(store.route_c_budget_count_for_day("2026-06-23").unwrap(), 2);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn route_c_budget_blocks_per_user_daily_limit_without_blocking_other_users() {
        let (store, path) = temp_store();
        let user_a = store
            .create_user(
                &format!(
                    "route-c-budget-user-a-{}@example.com",
                    Uuid::new_v4().simple()
                ),
                "secret1",
                None,
                None,
            )
            .unwrap();
        let user_b = store
            .create_user(
                &format!(
                    "route-c-budget-user-b-{}@example.com",
                    Uuid::new_v4().simple()
                ),
                "secret1",
                None,
                None,
            )
            .unwrap();

        recorded_event_id(
            store
                .route_c_budget_try_record_call(
                    &user_a.id,
                    "fp-a1",
                    "2026-06-23",
                    Some(10),
                    Some(1),
                )
                .unwrap(),
            1,
            1,
        );
        assert_eq!(
            store
                .route_c_budget_try_record_call(
                    &user_a.id,
                    "fp-a2",
                    "2026-06-23",
                    Some(10),
                    Some(1)
                )
                .unwrap(),
            RouteCBudgetRecordResult::UserLimitReached {
                total_used: 1,
                user_used: 1
            }
        );
        recorded_event_id(
            store
                .route_c_budget_try_record_call(
                    &user_b.id,
                    "fp-b1",
                    "2026-06-23",
                    Some(10),
                    Some(1),
                )
                .unwrap(),
            2,
            1,
        );
        assert_eq!(
            store
                .route_c_budget_count_for_day_and_user("2026-06-23", &user_a.id)
                .unwrap(),
            1
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn route_c_budget_completion_updates_outcome_without_prompt_text() {
        let (store, path) = temp_store();
        let user = store
            .create_user(
                &format!(
                    "route-c-budget-completion-{}@example.com",
                    Uuid::new_v4().simple()
                ),
                "secret1",
                None,
                None,
            )
            .unwrap();

        let event_id = recorded_event_id(
            store
                .route_c_budget_try_record_call(&user.id, "fp-1", "2026-06-23", Some(10), None)
                .unwrap(),
            1,
            1,
        );
        store
            .route_c_budget_mark_completed(
                &event_id,
                RouteCBudgetCompletion {
                    outcome: "provider_error".to_string(),
                    model: Some("route-c-model".to_string()),
                    total_tokens: Some(42),
                    error_summary: Some(
                        "rate_limit fingerprint=abc secret prompt text".to_string(),
                    ),
                },
            )
            .unwrap();

        let events = store
            .route_c_budget_recent_events(Some("2026-06-23"), Some(&user.id), 10)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].outcome, "provider_error");
        assert_eq!(events[0].model.as_deref(), Some("route-c-model"));
        assert_eq!(events[0].total_tokens, Some(42));
        assert_eq!(
            events[0].error_summary.as_deref(),
            Some("category=rate_limit, chars=45, fingerprint=d385d708e9876549")
        );
        assert!(!events[0]
            .error_summary
            .as_deref()
            .unwrap_or_default()
            .contains("secret prompt text"));
        assert!(events[0].completed_at.is_some());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn route_c_budget_count_is_day_scoped() {
        let (store, path) = temp_store();
        let user = store
            .create_user(
                &format!("route-c-budget-day-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();

        recorded_event_id(
            store
                .route_c_budget_try_record_call(&user.id, "fp-1", "2026-06-23", Some(10), None)
                .unwrap(),
            1,
            1,
        );

        assert_eq!(store.route_c_budget_count_for_day("2026-06-23").unwrap(), 1);
        assert_eq!(store.route_c_budget_count_for_day("2026-06-24").unwrap(), 0);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn route_c_budget_reports_stale_pending_admissions() {
        let (store, path) = temp_store();
        let user = store
            .create_user(
                &format!(
                    "route-c-budget-stale-{}@example.com",
                    Uuid::new_v4().simple()
                ),
                "secret1",
                None,
                None,
            )
            .unwrap();

        let stale_event = recorded_event_id(
            store
                .route_c_budget_try_record_call(&user.id, "fp-stale", "2026-06-23", None, None)
                .unwrap(),
            1,
            1,
        );
        let fresh_event = recorded_event_id(
            store
                .route_c_budget_try_record_call(&user.id, "fp-fresh", "2026-06-23", None, None)
                .unwrap(),
            2,
            2,
        );
        let done_event = recorded_event_id(
            store
                .route_c_budget_try_record_call(&user.id, "fp-done", "2026-06-23", None, None)
                .unwrap(),
            3,
            3,
        );
        set_budget_created_at(&store, &stale_event, "2026-06-23T00:00:00+00:00");
        set_budget_created_at(&store, &fresh_event, "2026-06-23T00:20:00+00:00");
        set_budget_created_at(&store, &done_event, "2026-06-23T00:00:00+00:00");
        store
            .route_c_budget_mark_completed(
                &done_event,
                RouteCBudgetCompletion {
                    outcome: "success".to_string(),
                    model: Some("route-c-fast".to_string()),
                    total_tokens: Some(88),
                    error_summary: None,
                },
            )
            .unwrap();

        let cutoff = "2026-06-23T00:15:00+00:00";
        let summaries = store
            .route_c_budget_day_summaries_with_stale(10, Some(cutoff))
            .unwrap();
        assert_eq!(summaries[0].route_day, "2026-06-23");
        assert_eq!(summaries[0].total_calls, 3);
        assert_eq!(summaries[0].completed_calls, 1);
        assert_eq!(summaries[0].pending_calls, 2);
        assert_eq!(summaries[0].stale_pending_calls, 1);
        assert_eq!(summaries[0].success_calls, 1);

        let outcomes = store
            .route_c_budget_outcome_summaries_with_stale(Some("2026-06-23"), None, Some(cutoff))
            .unwrap();
        let admitted = outcomes
            .iter()
            .find(|row| row.outcome == "admitted")
            .expect("pending admitted outcome summary");
        assert_eq!(admitted.total_calls, 2);
        assert_eq!(admitted.completed_calls, 0);
        assert_eq!(admitted.pending_calls, 2);
        assert_eq!(admitted.stale_pending_calls, 1);
        let success = outcomes
            .iter()
            .find(|row| row.outcome == "success")
            .expect("success outcome summary");
        assert_eq!(success.pending_calls, 0);
        assert_eq!(success.stale_pending_calls, 0);

        let stale_events = store
            .route_c_budget_stale_pending_events(Some("2026-06-23"), Some(&user.id), cutoff, 10)
            .unwrap();
        assert_eq!(stale_events.len(), 1);
        assert_eq!(stale_events[0].id, stale_event);
        assert_eq!(stale_events[0].completed_at, None);
        assert_eq!(stale_events[0].request_fingerprint, "fp-stale");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn route_c_budget_admin_report_summarizes_and_filters_events() {
        let (store, path) = temp_store();
        let user_a = store
            .create_user(
                &format!(
                    "route-c-budget-admin-a-{}@example.com",
                    Uuid::new_v4().simple()
                ),
                "secret1",
                None,
                None,
            )
            .unwrap();
        let user_b = store
            .create_user(
                &format!(
                    "route-c-budget-admin-b-{}@example.com",
                    Uuid::new_v4().simple()
                ),
                "secret1",
                None,
                None,
            )
            .unwrap();

        recorded_event_id(
            store
                .route_c_budget_try_record_call(&user_a.id, "fp-a1", "2026-06-22", None, None)
                .unwrap(),
            1,
            1,
        );
        let event_a2 = recorded_event_id(
            store
                .route_c_budget_try_record_call(&user_a.id, "fp-a2", "2026-06-23", None, None)
                .unwrap(),
            1,
            1,
        );
        let event_b1 = recorded_event_id(
            store
                .route_c_budget_try_record_call(&user_b.id, "fp-b1", "2026-06-23", None, None)
                .unwrap(),
            2,
            1,
        );
        store
            .route_c_budget_mark_completed(
                &event_a2,
                RouteCBudgetCompletion {
                    outcome: "success".to_string(),
                    model: Some("route-c-fast".to_string()),
                    total_tokens: Some(120),
                    error_summary: None,
                },
            )
            .unwrap();
        store
            .route_c_budget_mark_completed(
                &event_b1,
                RouteCBudgetCompletion {
                    outcome: "provider_error".to_string(),
                    model: Some("route-c-fast".to_string()),
                    total_tokens: None,
                    error_summary: Some("rate_limit fingerprint=def".to_string()),
                },
            )
            .unwrap();

        let summaries = store
            .route_c_budget_day_summaries_with_stale(10, None)
            .unwrap();
        assert_eq!(summaries[0].route_day, "2026-06-23");
        assert_eq!(summaries[0].total_calls, 2);
        assert_eq!(summaries[0].completed_calls, 2);
        assert_eq!(summaries[0].pending_calls, 0);
        assert_eq!(summaries[0].stale_pending_calls, 0);
        assert_eq!(summaries[0].success_calls, 1);
        assert_eq!(summaries[0].failed_calls, 1);
        assert_eq!(summaries[0].unique_users, 2);
        assert_eq!(summaries[0].total_tokens, Some(120));
        assert_eq!(summaries[1].route_day, "2026-06-22");
        assert_eq!(summaries[1].total_calls, 1);
        assert_eq!(summaries[1].completed_calls, 0);
        assert_eq!(summaries[1].pending_calls, 1);
        assert_eq!(summaries[1].stale_pending_calls, 0);

        let outcomes = store
            .route_c_budget_outcome_summaries_with_stale(Some("2026-06-23"), None, None)
            .unwrap();
        assert_eq!(outcomes.len(), 2);
        let success = outcomes
            .iter()
            .find(|row| row.outcome == "success")
            .expect("success outcome summary");
        assert_eq!(success.total_calls, 1);
        assert_eq!(success.completed_calls, 1);
        assert_eq!(success.pending_calls, 0);
        assert_eq!(success.stale_pending_calls, 0);
        assert_eq!(success.unique_users, 1);
        assert_eq!(success.total_tokens, Some(120));
        let provider_error = outcomes
            .iter()
            .find(|row| row.outcome == "provider_error")
            .expect("provider_error outcome summary");
        assert_eq!(provider_error.total_calls, 1);
        assert_eq!(provider_error.completed_calls, 1);
        assert_eq!(provider_error.pending_calls, 0);
        assert_eq!(provider_error.stale_pending_calls, 0);
        assert_eq!(provider_error.unique_users, 1);
        assert_eq!(provider_error.total_tokens, None);

        let user_outcomes = store
            .route_c_budget_outcome_summaries_with_stale(None, Some(&user_a.id), None)
            .unwrap();
        assert!(user_outcomes.iter().any(|row| row.outcome == "admitted"));
        assert!(user_outcomes.iter().any(|row| row.outcome == "success"));
        assert!(!user_outcomes
            .iter()
            .any(|row| row.outcome == "provider_error"));

        let day_events = store
            .route_c_budget_recent_events(Some("2026-06-23"), None, 10)
            .unwrap();
        assert_eq!(day_events.len(), 2);
        assert!(day_events
            .iter()
            .all(|event| event.route_day == "2026-06-23"));
        assert!(day_events.iter().any(|event| event.outcome == "success"));
        assert!(day_events
            .iter()
            .any(|event| event.outcome == "provider_error"));

        let user_events = store
            .route_c_budget_recent_events(None, Some(&user_a.id), 10)
            .unwrap();
        assert_eq!(user_events.len(), 2);
        assert!(user_events.iter().all(|event| event.user_id == user_a.id));

        let _ = std::fs::remove_file(path);
    }
