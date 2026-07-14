use super::*;
use crate::store::{codex_vault_emergency::CodexVaultEmergencyLeaseCreate, Store};

fn temp_store() -> (Store, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon-codex-usage-estimation-{}.sqlite",
        uuid::Uuid::new_v4().simple()
    ));
    let _ = std::fs::remove_file(&path);
    (Store::open(&path).expect("store should open"), path)
}

#[test]
fn estimate_allocates_observed_percent_by_shared_tokens() {
    let (store, path) = temp_store();
    let provider = store
        .create_user(
            "usage-provider@example.com",
            "secret1",
            Some("provider"),
            None,
        )
        .unwrap();
    let a = store
        .create_user("usage-a@example.com", "secret1", Some("A"), None)
        .unwrap();
    let b = store
        .create_user("usage-b@example.com", "secret1", Some("B"), None)
        .unwrap();
    let grant_a = store
        .upsert_codex_vault_emergency_grant(
            &provider.id,
            &a.id,
            Some("provider to A"),
            Some("test"),
            Some(900),
            None,
            &provider.id,
        )
        .unwrap();
    let grant_b = store
        .upsert_codex_vault_emergency_grant(
            &provider.id,
            &b.id,
            Some("provider to B"),
            Some("test"),
            Some(900),
            None,
            &provider.id,
        )
        .unwrap();
    let lease_a = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant_a.id,
            provider_user_id: &provider.id,
            consumer_user_id: &a.id,
            consumer_node_id: "node-a",
            provider_slot_id: "slot",
            account_hint_hash: Some("hint"),
            purpose: Some("patient"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();
    let lease_b = store
        .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
            grant_id: &grant_b.id,
            provider_user_id: &provider.id,
            consumer_user_id: &b.id,
            consumer_node_id: "node-b",
            provider_slot_id: "slot",
            account_hint_hash: Some("hint"),
            purpose: Some("patient"),
            failure_reason: None,
            max_lease_seconds: 900,
        })
        .unwrap();
    store
        .record_codex_vault_usage_snapshot(&CodexVaultUsageSnapshotWrite {
            provider_user_id: provider.id.clone(),
            observed_by_user_id: provider.id.clone(),
            lease_id: None,
            account_hint_hash: Some("hint".to_string()),
            source: None,
            limit_id: "codex".to_string(),
            limit_name: None,
            plan_type: Some("pro".to_string()),
            used_percent: Some(10.0),
            remaining_percent: Some(90.0),
            window_duration_mins: Some(300),
            resets_at: Some("2026-07-06T10:00:00Z".to_string()),
            rate_limit_reached_type: None,
            credits_balance: None,
            lifetime_tokens: Some(1_000_000),
            daily_bucket_date: None,
            daily_tokens: None,
            observed_at: Some("2026-07-06T05:00:00Z".to_string()),
        })
        .unwrap();
    store
        .attach_codex_vault_emergency_usage(
            &lease_a.id,
            Some("tok-a"),
            None,
            None,
            600_000,
            0,
            0,
            0,
            Some("billed"),
        )
        .unwrap();
    store
        .attach_codex_vault_emergency_usage(
            &lease_b.id,
            Some("tok-b"),
            None,
            None,
            800_000,
            0,
            0,
            0,
            Some("billed"),
        )
        .unwrap();
    store
        .conn()
        .unwrap()
        .execute(
            "UPDATE codex_vault_emergency_lease_usage_events
                    SET created_at = CASE lease_id
                        WHEN ?1 THEN '2026-07-06T05:10:00Z'
                        WHEN ?2 THEN '2026-07-06T05:20:00Z'
                        ELSE created_at
                    END
                  WHERE lease_id IN (?1, ?2)",
            rusqlite::params![lease_a.id, lease_b.id],
        )
        .unwrap();
    store
        .record_codex_vault_usage_snapshot(&CodexVaultUsageSnapshotWrite {
            provider_user_id: provider.id.clone(),
            observed_by_user_id: provider.id.clone(),
            lease_id: None,
            account_hint_hash: Some("hint".to_string()),
            source: None,
            limit_id: "codex".to_string(),
            limit_name: None,
            plan_type: Some("pro".to_string()),
            used_percent: Some(30.0),
            remaining_percent: Some(70.0),
            window_duration_mins: Some(300),
            resets_at: Some("2026-07-06T10:00:00Z".to_string()),
            rate_limit_reached_type: None,
            credits_balance: None,
            lifetime_tokens: Some(2_400_000),
            daily_bucket_date: None,
            daily_tokens: None,
            observed_at: Some("2026-07-06T05:30:00Z".to_string()),
        })
        .unwrap();

    let report = store
        .codex_vault_usage_estimate_report(&provider.id, 30, "codex", 20_000)
        .unwrap();
    assert_eq!(report.windows.len(), 1);
    let window = &report.windows[0];
    assert_eq!(window.consumed_percent, 20.0);
    assert_eq!(window.official_token_delta, Some(1_400_000));
    assert_eq!(window.shared_token_total, 1_400_000);
    assert_eq!(window.allocations.len(), 2);
    let total_estimated: f64 = window.allocations.iter().map(|a| a.estimated_percent).sum();
    assert!((total_estimated - 20.0).abs() < 0.001);

    drop(store);
    let _ = std::fs::remove_file(path);
}
