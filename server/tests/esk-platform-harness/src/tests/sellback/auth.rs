use super::*;

fn operation(
    fixture: &Fixture,
    index: usize,
    user: &str,
    credential: &str,
    input: &SellbackSubmitInput,
    request_id: &str,
    config: &SellbackConfiguration,
) -> anyhow::Result<()> {
    match index {
        0 => fixture
            .store
            .esk_platform_sellback_page(user, credential, 0, Some("invalid"), config)
            .map(|_| ()),
        1 => fixture
            .store
            .esk_platform_sellback_request(user, credential, request_id, config)
            .map(|_| ()),
        2 => fixture
            .store
            .lookup_esk_platform_sellback(user, credential, &input.idempotency_key, config)
            .map(|_| ()),
        3 => fixture
            .store
            .submit_esk_platform_sellback(user, credential, input, config)
            .map(|_| ()),
        _ => fixture
            .store
            .cancel_esk_platform_sellback(user, credential, request_id, config)
            .map(|_| ()),
    }
}

#[test]
fn every_endpoint_rejects_missing_wrong_cross_user_or_virtual_credentials_before_input_errors() {
    let (fixture, _, config) = setup();
    let input = input(&fixture, "alice", "existing", 1, &config);
    let existing = fixture
        .store
        .submit_esk_platform_sellback("alice", &token("alice"), &input, &config)
        .unwrap();
    for (user, credential) in [
        ("alice", "".into()),
        ("alice", " ".into()),
        ("alice", token("bob")),
        ("ghost", token("alice")),
        ("local-owner", token("local-owner")),
        ("inactive-user", token("inactive-user")),
        ("alice", "synthetic-invalid".into()),
    ] {
        for index in 0..5 {
            error(
                operation(
                    &fixture,
                    index,
                    user,
                    &credential,
                    &input,
                    &existing.request.request_id,
                    &config,
                ),
                SellbackError::Unauthorized,
            );
        }
    }
    assert_eq!(fixture.count("esk_platform_sellback_requests"), 1);
    assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 0);
}

#[test]
fn preliminary_auth_does_not_survive_revocation_expiry_rebinding_or_account_disable() {
    for change in [
        "UPDATE sessions SET revoked_at = '2026-09-04T10:00:00Z' WHERE user_id = 'alice'",
        "UPDATE sessions SET expires_at = '2026-09-04T10:00:00Z' WHERE user_id = 'alice'",
        "UPDATE sessions SET expires_at = 'not-a-date' WHERE user_id = 'alice'",
        "UPDATE sessions SET expires_at = '2026-09-04T17:59:59+08:00' WHERE user_id = 'alice'",
        "UPDATE sessions SET user_id = 'bob' WHERE user_id = 'alice'",
        "UPDATE users SET status = 'disabled' WHERE id = 'alice'",
    ] {
        let (fixture, _, config) = setup();
        let input = input(&fixture, "alice", "existing", 1, &config);
        let existing = fixture
            .store
            .submit_esk_platform_sellback("alice", &token("alice"), &input, &config)
            .unwrap();
        fixture
            .store
            .validate_esk_platform_session("alice", &token("alice"))
            .unwrap();
        fixture.store.conn().unwrap().execute_batch(change).unwrap();
        for index in 0..5 {
            error(
                operation(
                    &fixture,
                    index,
                    "alice",
                    &token("alice"),
                    &input,
                    &existing.request.request_id,
                    &config,
                ),
                SellbackError::Unauthorized,
            );
        }
        assert_eq!(fixture.count("esk_platform_sellback_requests"), 1);
        assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 0);
    }
}

#[test]
fn valid_offset_expiry_is_accepted_by_all_current_session_gates() {
    let (fixture, _, config) = setup();
    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch(
            "UPDATE sessions SET expires_at = '2026-09-04T03:00:00-08:00' WHERE user_id = 'alice'",
        )
        .unwrap();
    let first = submit(&fixture, "alice", "future-offset", 1, &config);
    fixture
        .store
        .lookup_esk_platform_sellback("alice", &token("alice"), "future-offset", &config)
        .unwrap();
    fixture
        .store
        .cancel_esk_platform_sellback("alice", &token("alice"), &first.request.request_id, &config)
        .unwrap();
    assert_eq!(
        page(&fixture, "alice", &config).summary.reserved_base_units,
        0
    );
}

#[test]
fn in_transaction_session_revocation_rolls_back_request_or_cancel_and_the_revocation() {
    for cancel in [false, true] {
        let (fixture, _, config) = setup();
        let input = input(&fixture, "alice", "transaction-auth", 1, &config);
        let existing = if cancel {
            Some(
                fixture
                    .store
                    .submit_esk_platform_sellback("alice", &token("alice"), &input, &config)
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
        fixture
            .store
            .conn()
            .unwrap()
            .execute_batch(&format!(
                "CREATE TRIGGER synthetic_revoke AFTER INSERT ON {table} BEGIN
             UPDATE sessions SET revoked_at = '2026-09-04T10:00:00Z' WHERE user_id = 'alice'; END;"
            ))
            .unwrap();
        let result = match existing {
            Some(record) => fixture.store.cancel_esk_platform_sellback(
                "alice",
                &token("alice"),
                &record.request.request_id,
                &config,
            ),
            None => fixture.store.submit_esk_platform_sellback(
                "alice",
                &token("alice"),
                &input,
                &config,
            ),
        };
        error(result, SellbackError::Unauthorized);
        assert_eq!(
            fixture.count("esk_platform_sellback_requests"),
            i64::from(cancel)
        );
        assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 0);
        fixture
            .store
            .validate_esk_platform_session("alice", &token("alice"))
            .unwrap();
    }
}

#[test]
fn page_detail_lookup_and_exact_replays_do_not_commit_persistent_writes() {
    let (fixture, _, config) = setup();
    let input = input(&fixture, "alice", "read-only", 1, &config);
    let first = fixture
        .store
        .submit_esk_platform_sellback("alice", &token("alice"), &input, &config)
        .unwrap();
    fixture
        .store
        .cancel_esk_platform_sellback("alice", &token("alice"), &first.request.request_id, &config)
        .unwrap();
    let observer = fixture.store.conn().unwrap();
    let before: i64 = observer
        .query_row("PRAGMA data_version", [], |row| row.get(0))
        .unwrap();
    for _ in 0..3 {
        page(&fixture, "alice", &SellbackConfiguration::Disabled);
        fixture
            .store
            .esk_platform_sellback_request(
                "alice",
                &token("alice"),
                &first.request.request_id,
                &config,
            )
            .unwrap();
        fixture
            .store
            .lookup_esk_platform_sellback("alice", &token("alice"), "read-only", &config)
            .unwrap();
        assert!(
            fixture
                .store
                .submit_esk_platform_sellback(
                    "alice",
                    &token("alice"),
                    &input,
                    &SellbackConfiguration::Invalid
                )
                .unwrap()
                .replayed
        );
        assert!(
            fixture
                .store
                .cancel_esk_platform_sellback(
                    "alice",
                    &token("alice"),
                    &first.request.request_id,
                    &config
                )
                .unwrap()
                .replayed
        );
        error(
            fixture
                .store
                .lookup_esk_platform_sellback("alice", &token("alice"), "absent", &config),
            SellbackError::NotFound,
        );
    }
    let after: i64 = observer
        .query_row("PRAGMA data_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        before, after,
        "a read or replay committed a persistent write"
    );
    assert_eq!(fixture.count("esk_platform_sellback_requests"), 1);
    assert_eq!(fixture.count("esk_platform_sellback_cancellations"), 1);
    assert_eq!(fixture.paper_total(), 123_000_000);
}
