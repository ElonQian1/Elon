use super::*;
use rusqlite::params;

fn mutate_request(fixture: &Fixture, id: &str, field: &str, value: &str) {
    let conn = fixture.store.conn().unwrap();
    conn.pragma_update(None, "foreign_keys", false).unwrap();
    conn.execute_batch("DROP TRIGGER IF EXISTS trg_esk_platform_sellback_requests_no_update")
        .unwrap();
    // The column comes only from fixed test literals below, never external input.
    conn.execute(
        &format!("UPDATE esk_platform_sellback_requests SET {field} = ?1 WHERE request_id = ?2"),
        params![value, id],
    )
    .unwrap();
}

#[test]
fn off_page_request_corruption_cannot_hide_from_detail_lookup_or_new_writes() {
    for (field, value) in [
        ("input_json", "{}".into()),
        ("policy_json", "{}".into()),
        ("request_digest", "0".repeat(64)),
        ("platform_policy_digest", "0".repeat(64)),
        ("source_fingerprint", "0".repeat(64)),
        ("idempotency_key", "other-key".into()),
        ("amount_base_units", "2".into()),
        ("created_at", "not-a-date".into()),
    ] {
        let (fixture, _, config) = setup();
        submit(&fixture, "alice", "one", 1, &config);
        submit(&fixture, "alice", "two", 1, &config);
        let whole = page(&fixture, "alice", &config);
        let first = &whole.requests[0];
        let last = &whole.requests[1];
        let next = input(&fixture, "alice", "next", 1, &config);
        mutate_request(&fixture, &last.request_id, field, &value);
        error(
            fixture
                .store
                .esk_platform_sellback_page("alice", &token("bob"), 1, None, &config),
            SellbackError::Unauthorized,
        );
        error(
            fixture
                .store
                .esk_platform_sellback_page("alice", &token("alice"), 1, None, &config),
            SellbackError::Corrupt,
        );
        error(
            fixture.store.esk_platform_sellback_request(
                "alice",
                &token("alice"),
                &first.request_id,
                &config,
            ),
            SellbackError::Corrupt,
        );
        error(
            fixture.store.lookup_esk_platform_sellback(
                "alice",
                &token("alice"),
                &first.input.idempotency_key,
                &config,
            ),
            SellbackError::Corrupt,
        );
        error(
            fixture
                .store
                .submit_esk_platform_sellback("alice", &token("alice"), &next, &config),
            SellbackError::Corrupt,
        );
        error(
            fixture.store.cancel_esk_platform_sellback(
                "alice",
                &token("alice"),
                &first.request_id,
                &config,
            ),
            SellbackError::Corrupt,
        );
        assert_eq!(fixture.count("esk_platform_sellback_requests"), 2);
        assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 0);
    }
}

#[test]
fn corrupt_other_users_open_or_canceled_record_is_not_summed_into_global_cap() {
    for canceled in [false, true] {
        let (fixture, _, config) = setup();
        let bob = submit(&fixture, "bob", "global-corruption", 1, &config);
        if canceled {
            fixture
                .store
                .cancel_esk_platform_sellback(
                    "bob",
                    &token("bob"),
                    &bob.request.request_id,
                    &config,
                )
                .unwrap();
        }
        let next = input(&fixture, "alice", "next", 1, &config);
        mutate_request(&fixture, &bob.request.request_id, "input_json", "{}");
        // Private reads validate the complete owner ledger, not other users' payloads.
        assert_eq!(
            page(&fixture, "alice", &config).summary.reserved_base_units,
            0
        );
        error(
            fixture
                .store
                .submit_esk_platform_sellback("alice", &token("alice"), &next, &config),
            SellbackError::Corrupt,
        );
        assert_eq!(fixture.count("esk_platform_sellback_requests"), 1);
    }
}

#[test]
fn self_consistent_policy_on_wrong_formal_source_is_rejected_by_own_and_global_scans() {
    let (fixture, _, config) = setup();
    let mut record = submit(&fixture, "bob", "wrong-source", 1, &config).request;
    let next = input(&fixture, "alice", "next", 1, &config);
    record.policy.body.source_fingerprint = "f".repeat(64);
    record.policy = validate_policy(record.policy.body).unwrap();
    record.input.policy_digest = record.policy.policy_digest.clone();
    record.request_digest = request_digest("bob", &record.policy, &record.input).unwrap();
    validate_stored_request(&record).unwrap();
    let conn = fixture.store.conn().unwrap();
    conn.execute_batch("DROP TRIGGER trg_esk_platform_sellback_requests_no_update")
        .unwrap();
    conn.execute("UPDATE esk_platform_sellback_requests SET input_json = ?1, policy_json = ?2, request_digest = ?3, source_fingerprint = ?4 WHERE request_id = ?5",
        params![serde_json::to_string(&record.input).unwrap(), serde_json::to_string(&record.policy).unwrap(), record.request_digest, record.policy.body.source_fingerprint, record.request_id]).unwrap();
    error(
        fixture
            .store
            .esk_platform_sellback_page("bob", &token("bob"), 20, None, &config),
        SellbackError::Corrupt,
    );
    error(
        fixture
            .store
            .submit_esk_platform_sellback("alice", &token("alice"), &next, &config),
        SellbackError::Corrupt,
    );
}

#[test]
fn canceled_event_binding_or_orphan_damage_never_silently_releases_balance() {
    for (field, value) in [
        ("canceled_by", "bob".into()),
        ("request_digest", "0".repeat(64)),
        ("cancel_event_id", format!("badbad_{}", "0".repeat(32))),
        ("created_at", "2026-09-04T09:59:59.999Z".into()),
    ] {
        let (fixture, _, config) = setup();
        let first = submit(&fixture, "alice", "cancel-corruption", 1, &config);
        fixture
            .store
            .cancel_esk_platform_sellback(
                "alice",
                &token("alice"),
                &first.request.request_id,
                &config,
            )
            .unwrap();
        let conn = fixture.store.conn().unwrap();
        conn.execute_batch("DROP TRIGGER trg_esk_platform_sellback_cancellations_no_update")
            .unwrap();
        conn.execute(
            &format!("UPDATE esk_platform_sellback_cancellations SET {field} = ?1"),
            params![value],
        )
        .unwrap();
        error(
            fixture
                .store
                .esk_platform_sellback_page("alice", &token("alice"), 20, None, &config),
            SellbackError::Corrupt,
        );
        error(
            fixture.store.cancel_esk_platform_sellback(
                "alice",
                &token("alice"),
                &first.request.request_id,
                &config,
            ),
            SellbackError::Corrupt,
        );
        assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 1);
    }
    let (fixture, _, config) = setup();
    let first = submit(&fixture, "alice", "orphan", 1, &config);
    fixture
        .store
        .cancel_esk_platform_sellback("alice", &token("alice"), &first.request.request_id, &config)
        .unwrap();
    let conn = fixture.store.conn().unwrap();
    conn.pragma_update(None, "foreign_keys", false).unwrap();
    conn.execute_batch("DROP TRIGGER trg_esk_platform_sellback_requests_no_delete; DELETE FROM esk_platform_sellback_requests").unwrap();
    error(
        fixture
            .store
            .esk_platform_sellback_page("alice", &token("alice"), 20, None, &config),
        SellbackError::Corrupt,
    );
}

#[test]
fn self_consistent_request_above_formal_total_fails_without_clamping() {
    let (fixture, _, config) = setup();
    let mut record = submit(&fixture, "alice", "too-much", 1, &config).request;
    record.input.amount_base_units = 11_000_000;
    record.request_digest = request_digest("alice", &record.policy, &record.input).unwrap();
    validate_stored_request(&record).unwrap();
    let conn = fixture.store.conn().unwrap();
    conn.execute_batch("DROP TRIGGER trg_esk_platform_sellback_requests_no_update")
        .unwrap();
    conn.execute("UPDATE esk_platform_sellback_requests SET amount_base_units = ?1, input_json = ?2, request_digest = ?3",
        params![record.input.amount_base_units, serde_json::to_string(&record.input).unwrap(), record.request_digest]).unwrap();
    error(
        fixture
            .store
            .esk_platform_sellback_page("alice", &token("alice"), 20, None, &config),
        SellbackError::Corrupt,
    );
    error(
        fixture.store.cancel_esk_platform_sellback(
            "alice",
            &token("alice"),
            &record.request_id,
            &config,
        ),
        SellbackError::Corrupt,
    );
}

#[test]
fn off_page_formal_allocation_corruption_blocks_sellback_even_with_valid_requests() {
    let (fixture, formal, config) = setup();
    history::post(&fixture, &formal, "alice", 8);
    let first = submit(&fixture, "alice", "formal-corrupt", 1, &config);
    let conn = fixture.store.conn().unwrap();
    conn.execute_batch("DROP TRIGGER trg_esk_platform_allocations_no_update;
        UPDATE esk_platform_allocations SET input_json = '{}' WHERE allocation_id = (
          SELECT allocation_id FROM esk_platform_ledger_entries WHERE user_id = 'alice' ORDER BY created_at,entry_id LIMIT 1)").unwrap();
    error(
        fixture.store.lookup_esk_platform_sellback(
            "alice",
            &token("alice"),
            "formal-corrupt",
            &config,
        ),
        SellbackError::Corrupt,
    );
    error(
        fixture.store.cancel_esk_platform_sellback(
            "alice",
            &token("alice"),
            &first.request.request_id,
            &config,
        ),
        SellbackError::Corrupt,
    );
}

#[test]
fn insert_failures_roll_back_without_new_requests_or_cancellation_events() {
    for cancel in [false, true] {
        let (fixture, _, config) = setup();
        let new = input(&fixture, "alice", "failed-insert", 1, &config);
        let existing = if cancel {
            Some(
                fixture
                    .store
                    .submit_esk_platform_sellback("alice", &token("alice"), &new, &config)
                    .unwrap(),
            )
        } else {
            None
        };
        let table = if cancel {
            "esk_platform_sellback_cancellations"
        } else {
            "esk_platform_sellback_requests"
        };
        fixture.store.conn().unwrap().execute_batch(&format!(
            "CREATE TRIGGER synthetic_abort BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'synthetic-only'); END"
        )).unwrap();
        let result = match existing {
            Some(record) => fixture.store.cancel_esk_platform_sellback(
                "alice",
                &token("alice"),
                &record.request.request_id,
                &config,
            ),
            None => {
                fixture
                    .store
                    .submit_esk_platform_sellback("alice", &token("alice"), &new, &config)
            }
        };
        assert!(result.is_err());
        assert_eq!(
            fixture.count("esk_platform_sellback_requests"),
            i64::from(cancel)
        );
        assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 0);
    }
}
