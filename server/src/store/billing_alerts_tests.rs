    use super::super::codex_vault_emergency::CodexVaultEmergencyLeaseCreate;
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon_billing_alerts_{}.db",
            Uuid::new_v4().simple()
        ));
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn negative_balance_creates_critical_alert() {
        let (store, path) = temp_store();
        let user = store
            .create_user("billing-alert-negative@example.com", "secret1", None, None)
            .unwrap();
        store
            .billing_recharge(&user.id, 1, "test", "test", None)
            .unwrap();
        store
            .conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE user_balance SET balance_fen = -1 WHERE user_id = ?1",
                params![user.id],
            )
            .unwrap();

        let alerts = store.refresh_billing_alerts().unwrap();
        assert!(alerts.iter().any(|alert| {
            alert.fingerprint == "billing:negative-balances" && alert.severity == "critical"
        }));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn codex_sharing_anomalies_create_admin_alerts() {
        let (store, path) = temp_store();
        let provider = store
            .create_user(
                "billing-alert-codex-provider@example.com",
                "secret1",
                Some("provider"),
                None,
            )
            .unwrap();
        let consumer = store
            .create_user(
                "billing-alert-codex-consumer@example.com",
                "secret1",
                Some("consumer"),
                None,
            )
            .unwrap();
        let grant = store
            .upsert_codex_vault_emergency_grant(
                &provider.id,
                &consumer.id,
                Some("provider shares to consumer"),
                Some("robot_codex_vault_shared_access"),
                Some(900),
                None,
                &provider.id,
            )
            .unwrap();
        let lease = store
            .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
                grant_id: &grant.id,
                provider_user_id: &provider.id,
                consumer_user_id: &consumer.id,
                consumer_node_id: "node-consumer",
                provider_slot_id: "slot-provider",
                account_hint_hash: Some("hint-provider"),
                purpose: Some("billing_alert_test"),
                failure_reason: None,
                max_lease_seconds: 900,
            })
            .unwrap();
        store
            .conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE codex_vault_emergency_leases
                    SET expires_at = '2000-01-01T00:00:00+00:00',
                        total_tokens = 42
                  WHERE id = ?1",
                params![lease.id],
            )
            .unwrap();
        store
            .record_codex_vault_event(
                &consumer.id,
                "sharing_restore_failed",
                Some("node-consumer"),
                false,
                Some("unit test failure"),
            )
            .unwrap();

        let alerts = store.refresh_billing_alerts().unwrap();
        assert!(alerts.iter().any(|alert| {
            alert.fingerprint == "billing:codex-sharing-expired-uncleared-leases"
                && alert.severity == "warning"
        }));
        assert!(alerts.iter().any(|alert| {
            alert.fingerprint == "billing:codex-sharing-accounting-anomalies"
                && alert.severity == "critical"
        }));
        assert!(alerts.iter().any(|alert| {
            alert.fingerprint == "billing:codex-sharing-recent-failures"
                && alert.severity == "warning"
        }));

        drop(store);
        let _ = std::fs::remove_file(path);
    }
