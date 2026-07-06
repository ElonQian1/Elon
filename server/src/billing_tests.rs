mod tests {
    use super::*;
    use crate::store::BillingPriceRuleUpsert;
    use uuid::Uuid;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon_billing_runtime_{}.db",
            Uuid::new_v4().simple()
        ));
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn strict_billing_grants_new_user_trial_credit_before_first_call() {
        let (store, path) = temp_store();
        let expected = new_user_trial_credit_fen(&store);
        if expected <= 0 {
            let _ = std::fs::remove_file(path);
            return;
        }

        let user = store
            .create_user(
                &format!("trial-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();
        assert_eq!(store.billing_get_balance(&user.id).unwrap(), None);

        check_can_call(&store, &user.id).unwrap();

        assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(expected));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn new_user_trial_credit_is_only_granted_once() {
        let (store, path) = temp_store();
        store
            .billing_set_config("new_user_trial_credit_fen", "100")
            .unwrap();
        let expected = new_user_trial_credit_fen(&store);
        if expected <= 0 {
            let _ = std::fs::remove_file(path);
            return;
        }

        let user = store
            .create_user(
                &format!("trial-once-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();

        check_can_call(&store, &user.id).unwrap();
        store
            .billing_deduct(
                &user.id,
                expected,
                None,
                0,
                0,
                0,
                73000,
                1200,
                BillingPriceSnapshot::legacy(),
            )
            .unwrap();
        assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(0));

        let err = check_can_call(&store, &user.id).unwrap_err();
        assert!(err.contains("余额不足"));
        assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(0));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn trial_credit_lookup_sums_topup_records() {
        let (store, path) = temp_store();
        let user = store
            .create_user(
                &format!("trial-topup-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();

        store
            .billing_recharge(
                &user.id,
                100,
                NEW_USER_TRIAL_METHOD,
                "system",
                Some("old trial"),
            )
            .unwrap();
        store
            .billing_recharge(
                &user.id,
                29_900,
                NEW_USER_TRIAL_METHOD,
                "system",
                Some("trial top-up"),
            )
            .unwrap();

        let grant = store
            .billing_find_recharge_by_method(&user.id, NEW_USER_TRIAL_METHOD)
            .unwrap()
            .expect("trial grant should be summarized");
        assert_eq!(grant.amount_fen, 30_000);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reservation_grants_trial_credit_before_holding_balance() {
        let (store, path) = temp_store();
        let expected = new_user_trial_credit_fen(&store);
        if expected <= 0 {
            let _ = std::fs::remove_file(path);
            return;
        }
        let reserve_fen = expected.min(10);
        let user = store
            .create_user(
                &format!("trial-reserve-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();

        reserve_trusted_call(
            &store,
            &user.id,
            "test-reservation",
            "chat",
            "server_api_key",
            Some("test-model"),
            reserve_fen,
        )
        .unwrap();

        assert_eq!(
            store.billing_get_balance(&user.id).unwrap(),
            Some(expected - reserve_fen)
        );
        release_trusted_call(&store, &user.id, "test-reservation", "released_no_usage");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn estimate_cost_uses_configured_price_rule() {
        let (store, path) = temp_store();
        store
            .billing_set_config("usd_to_rmb_rate_x10000", "10000")
            .unwrap();
        store.billing_set_config("markup_x1000", "1000").unwrap();
        let rule = store
            .billing_upsert_price_rule(&BillingPriceRuleUpsert {
                pattern: "custom-expensive".to_string(),
                input_usd_per_m: 1.0,
                cached_usd_per_m: 0.5,
                output_usd_per_m: 1000.0,
                priority: 999,
                enabled: true,
                note: Some("snapshot test".to_string()),
            })
            .unwrap();

        let cost = estimate_cost_for_tokens(&store, "custom-expensive-v1", 0, 0, 1_000_000);
        assert_eq!(cost, 100_000);
        let user = store
            .create_user(
                &format!("billing-price-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();
        store
            .billing_recharge(&user.id, 200_000, "test", "test", None)
            .unwrap();

        let record = TokenUsageRecord {
            user_id: &user.id,
            feature: "price_rule_feature",
            usage_mode: "server_codex_cli",
            model: Some("custom-expensive-v1"),
            input_tokens: 0,
            cached_input_tokens: 0,
            output_tokens: 1_000_000,
            reasoning_tokens: 0,
            total_tokens: 1_000_000,
            billing_source: None,
            resource_owner_user_id: None,
            idempotency_key: Some("price-snapshot-key-1"),
        };
        let result = account_trusted_usage(&store, &record).unwrap();
        assert_eq!(result.accounting_status, "billed");
        assert_eq!(result.cost_rmb_fen, 100_000);

        let (events, total) = store.billing_list_events(&user.id, 1, 10).unwrap();
        assert_eq!(total, 1);
        let event = &events[0];
        assert_eq!(
            event.token_usage_event_id.as_deref(),
            Some(result.token_usage_event_id.as_str())
        );
        assert_eq!(event.price_rule_id.as_deref(), Some(rule.id.as_str()));
        assert_eq!(event.price_rule_version, Some(rule.version));
        assert_eq!(
            event.price_rule_pattern.as_deref(),
            Some("custom-expensive")
        );
        assert_eq!(event.input_usd_per_m, Some(1.0));
        assert_eq!(event.cached_usd_per_m, Some(0.5));
        assert_eq!(event.output_usd_per_m, Some(1000.0));
        assert_eq!(event.price_source.as_str(), "rule");
        let _ = std::fs::remove_file(path);
    }
