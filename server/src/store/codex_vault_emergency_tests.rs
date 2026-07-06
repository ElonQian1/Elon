    use super::CodexVaultEmergencyLeaseCreate;
    use crate::store::Store;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-codex-sharing-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn grant_reciprocal_and_active_lease_are_queryable() {
        let (store, path) = temp_store();
        let a = store
            .create_user("vault-a@example.com", "secret1", Some("A"), None)
            .unwrap();
        let b = store
            .create_user("vault-b@example.com", "secret1", Some("B"), None)
            .unwrap();
        let ab = store
            .upsert_codex_vault_emergency_grant(
                &a.id,
                &b.id,
                Some("A to B"),
                Some("test"),
                Some(900),
                None,
                &a.id,
            )
            .unwrap();
        store
            .upsert_codex_vault_emergency_grant(
                &b.id,
                &a.id,
                Some("B to A"),
                Some("test"),
                Some(900),
                None,
                &b.id,
            )
            .unwrap();

        let grants = store.list_codex_vault_emergency_grants(&a.id).unwrap();
        assert_eq!(grants.len(), 2);
        assert!(grants.iter().any(|grant| grant.reciprocal_active));

        let lease = store
            .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
                grant_id: &ab.id,
                provider_user_id: &a.id,
                consumer_user_id: &b.id,
                consumer_node_id: "node-b",
                provider_slot_id: "slot-a",
                account_hint_hash: Some("hint-a"),
                purpose: Some("unit_test"),
                failure_reason: None,
                max_lease_seconds: 900,
            })
            .unwrap();
        assert_eq!(lease.billing_source, "shared_codex");
        let active = store
            .get_active_codex_vault_emergency_lease_for_node(&b.id, "node-b")
            .unwrap()
            .expect("active lease");
        assert_eq!(active.provider_user_id, a.id);

        assert!(store
            .attach_codex_vault_emergency_usage(
                &lease.id,
                Some("tok_1"),
                Some("bill_1"),
                Some("ntx_1"),
                100,
                50,
                7,
                5,
                Some("billed"),
            )
            .unwrap());
        let billed = store
            .get_codex_vault_emergency_lease(&lease.id)
            .unwrap()
            .unwrap();
        assert_eq!(billed.total_tokens, 150);
        assert_eq!(billed.billed_cost_rmb_fen, 7);

        let cleared = store
            .clear_codex_vault_emergency_lease_for_node(&b.id, "node-b", Some(&lease.id))
            .unwrap()
            .expect("cleared lease");
        assert_eq!(cleared.status, "cleared");
        assert!(cleared.cleared_at.is_some());
        assert!(store
            .get_active_codex_vault_emergency_lease_for_node(&b.id, "node-b")
            .unwrap()
            .is_none());

        assert!(!store
            .attach_codex_vault_emergency_usage(
                &lease.id,
                Some("tok_after_clear"),
                Some("bill_after_clear"),
                Some("ntx_after_clear"),
                100,
                50,
                7,
                5,
                Some("billed"),
            )
            .unwrap());
        let updated = store
            .get_codex_vault_emergency_lease(&lease.id)
            .unwrap()
            .unwrap();
        assert_eq!(updated.total_tokens, 150);
        assert_eq!(updated.billed_cost_rmb_fen, 7);

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn revoked_and_expired_grants_are_not_shareable() {
        let (store, path) = temp_store();
        let provider = store
            .create_user(
                "vault-provider@example.com",
                "secret1",
                Some("provider"),
                None,
            )
            .unwrap();
        let consumer = store
            .create_user(
                "vault-consumer@example.com",
                "secret1",
                Some("consumer"),
                None,
            )
            .unwrap();
        let grant = store
            .upsert_codex_vault_emergency_grant(
                &provider.id,
                &consumer.id,
                Some("provider to consumer"),
                Some("robot_codex_vault_shared_access"),
                Some(900),
                None,
                &provider.id,
            )
            .unwrap();
        assert!(store
            .find_active_codex_vault_emergency_grant(&provider.id, &consumer.id)
            .unwrap()
            .is_some());
        assert!(store
            .revoke_codex_vault_emergency_grant(&grant.id, &provider.id)
            .unwrap());
        assert!(store
            .find_active_codex_vault_emergency_grant(&provider.id, &consumer.id)
            .unwrap()
            .is_none());

        let reverse = store
            .upsert_codex_vault_emergency_grant(
                &consumer.id,
                &provider.id,
                Some("expired reverse"),
                Some("robot_codex_vault_shared_access"),
                Some(900),
                Some("2000-01-01T00:00:00+00:00"),
                &consumer.id,
            )
            .unwrap();
        assert_eq!(reverse.status, "active");
        assert!(store
            .find_active_codex_vault_emergency_grant(&consumer.id, &provider.id)
            .unwrap()
            .is_none());

        drop(store);
        let _ = std::fs::remove_file(path);
    }
