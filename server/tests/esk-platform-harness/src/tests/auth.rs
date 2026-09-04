use super::*;

#[test]
fn ordinary_read_session_gate_checks_identity_expiry_and_is_database_read_only() {
    for case in [
        "allowed",
        "other-session",
        "revoked",
        "disabled",
        "malformed-expiry",
        "expired",
        "exact-now",
        "offset-expired",
        "offset-future",
        "local-owner",
        "empty-token",
    ] {
        let fixture = Fixture::new();
        let conn = fixture.store.conn().unwrap();
        let user = if case == "local-owner" {
            "local-owner"
        } else {
            "alice"
        };
        let session = match case {
            "other-session" => token("bob"),
            "empty-token" => String::new(),
            _ => token(user),
        };
        match case {
            "revoked" => {
                conn.execute(
                    "UPDATE sessions SET revoked_at='fixture' WHERE id='alice'",
                    [],
                )
                .unwrap();
            }
            "disabled" => {
                conn.execute("UPDATE users SET status='disabled' WHERE id='alice'", [])
                    .unwrap();
            }
            "malformed-expiry" | "expired" | "exact-now" | "offset-expired" | "offset-future" => {
                let expiry = match case {
                    "malformed-expiry" => "not-a-date",
                    "expired" => "2000-01-01T00:00:00Z",
                    "exact-now" => "2026-09-04T10:00:00Z",
                    "offset-expired" => "2026-09-04T17:59:59+08:00",
                    _ => "2026-09-04T03:00:00-08:00",
                };
                conn.execute(
                    "UPDATE sessions SET expires_at=?1 WHERE id='alice'",
                    params![expiry],
                )
                .unwrap();
            }
            _ => (),
        }
        let tables = [
            "esk_platform_policy",
            "esk_platform_allocations",
            "esk_platform_approvals",
            "esk_platform_ledger_entries",
            "esk_platform_cancellations",
            "esk_asset_ledger_entries",
        ];
        let counts_before = tables.map(|table| fixture.count(table));
        let version_before: i64 = conn
            .query_row("PRAGMA data_version", [], |row| row.get(0))
            .unwrap();
        let result = fixture.store.validate_esk_platform_session(user, &session);
        if matches!(case, "allowed" | "offset-future") {
            assert!(
                result.is_ok(),
                "ordinary user session rejected: {case}: {result:?}"
            );
        } else {
            assert_error(result, PlatformError::Unauthorized);
        }
        let version_after: i64 = conn
            .query_row("PRAGMA data_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            version_before, version_after,
            "session gate committed a write: {case}"
        );
        assert_eq!(counts_before, tables.map(|table| fixture.count(table)));
        assert_eq!(fixture.paper_total(), 123000000);
    }
}

#[test]
fn expiry_is_validated_as_an_instant_for_prepare_record_and_cancel() {
    // Harness now() is 2026-09-04T10:00:00Z. Offset strings deliberately have
    // lexical ordering opposite to their real expiry, guarding against string comparisons.
    for (expiry, allowed) in [
        ("not-a-date", false),
        ("2026-09-04T10:00:00Z", false),
        ("2026-09-04T17:59:59+08:00", false),
        ("2026-09-04T03:00:00-08:00", true),
    ] {
        for operation in ["prepare", "record", "cancel"] {
            let fixture = Fixture::new();
            let policy = policy(100000000);
            let pending = prepare(&fixture, &policy);
            fixture
                .store
                .conn()
                .unwrap()
                .execute(
                    "UPDATE sessions SET expires_at=?1 WHERE id='admin-1'",
                    params![expiry],
                )
                .unwrap();
            let result = match operation {
                "prepare" => fixture.store.prepare_esk_platform_allocation(
                    &policy,
                    &pending.input,
                    "admin-1",
                    &token("admin-1"),
                ),
                "record" => fixture.store.record_esk_platform_allocation(
                    &policy,
                    &pending.allocation_id,
                    &pending.input.request_digest,
                    "admin-1",
                    &token("admin-1"),
                ),
                _ => fixture.store.cancel_esk_platform_allocation(
                    &policy,
                    &pending.allocation_id,
                    &pending.input.request_digest,
                    "admin-1",
                    &token("admin-1"),
                ),
            };
            if allowed {
                assert!(
                    result.is_ok(),
                    "rejected valid expiry for {operation}: {result:?}"
                );
            } else {
                assert_error(result, PlatformError::Unauthorized);
                fixture.assert_empty_posting();
                assert_eq!(fixture.count("esk_platform_cancellations"), 0);
            }
        }
    }
}

#[test]
fn administrator_or_owner_requires_matching_active_database_session() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    for actor in ["alice", "inactive-admin", "local-owner", "missing-admin"] {
        assert_error(
            fixture.store.prepare_esk_platform_allocation(
                &policy,
                &input(&policy),
                actor,
                &token(actor),
            ),
            PlatformError::Unauthorized,
        );
    }
    for session in [
        String::new(),
        "unknown-session".to_owned(),
        token("owner-1"),
        token("alice"),
    ] {
        assert_error(
            fixture.store.prepare_esk_platform_allocation(
                &policy,
                &input(&policy),
                "admin-1",
                &session,
            ),
            PlatformError::Unauthorized,
        );
    }
    assert_eq!(fixture.count("esk_platform_policy"), 0);
    let prepared = fixture
        .store
        .prepare_esk_platform_allocation(&policy, &input(&policy), "owner-1", &token("owner-1"))
        .unwrap();
    let recorded = fixture
        .store
        .record_esk_platform_allocation(
            &policy,
            &prepared.allocation_id,
            &prepared.input.request_digest,
            "owner-1",
            &token("owner-1"),
        )
        .unwrap();
    assert!(recorded.recorded_at.is_some());
}

#[test]
fn missing_disabled_or_virtual_recipient_is_never_created_or_credited() {
    let fixture = Fixture::new();
    let policy = policy(100000000);
    let before = fixture.count("users");
    for user in ["missing-user", "inactive-user", "local-owner"] {
        let mut value = body();
        value.user_id = user.into();
        let value = prepare_input(&policy, value).unwrap();
        assert_error(
            fixture.store.prepare_esk_platform_allocation(
                &policy,
                &value,
                "admin-1",
                &token("admin-1"),
            ),
            PlatformError::UserUnavailable,
        );
        assert_error(
            fixture.store.esk_platform_account(user, &token(user), 20),
            PlatformError::Unauthorized,
        );
    }
    assert_eq!(fixture.count("users"), before);
    assert_eq!(fixture.count("esk_platform_policy"), 0);
    fixture.assert_empty_posting();
}

#[test]
fn role_or_recipient_status_is_rechecked_between_prepare_and_confirm() {
    for recipient_change in [false, true] {
        let fixture = Fixture::new();
        let policy = policy(100000000);
        let prepared = prepare(&fixture, &policy);
        let conn = fixture.store.conn().unwrap();
        if recipient_change {
            conn.execute("UPDATE users SET status='disabled' WHERE id='alice'", [])
                .unwrap();
        } else {
            conn.execute("UPDATE users SET role='user' WHERE id='admin-1'", [])
                .unwrap();
        }
        assert_error(
            fixture.store.record_esk_platform_allocation(
                &policy,
                &prepared.allocation_id,
                &prepared.input.request_digest,
                "admin-1",
                &token("admin-1"),
            ),
            if recipient_change {
                PlatformError::UserUnavailable
            } else {
                PlatformError::Unauthorized
            },
        );
        assert_eq!(fixture.count("esk_platform_approvals"), 0);
        assert_eq!(fixture.count("esk_platform_ledger_entries"), 0);
        assert_eq!(fixture.paper_total(), 123000000);
    }
}

#[test]
fn revoked_expired_or_wrong_user_session_cannot_confirm_existing_application() {
    for change in ["revoke", "expire", "other-user"] {
        let fixture = Fixture::new();
        let policy = policy(100000000);
        let prepared = prepare(&fixture, &policy);
        let conn = fixture.store.conn().unwrap();
        match change {
            "revoke" => conn.execute(
                "UPDATE sessions SET revoked_at='fixture' WHERE id='admin-1'",
                [],
            ),
            "expire" => conn.execute(
                "UPDATE sessions SET expires_at='2000-01-01T00:00:00Z' WHERE id='admin-1'",
                [],
            ),
            _ => conn.execute(
                "UPDATE sessions SET user_id='owner-1' WHERE id='admin-1'",
                [],
            ),
        }
        .unwrap();
        assert_error(
            fixture.store.record_esk_platform_allocation(
                &policy,
                &prepared.allocation_id,
                &prepared.input.request_digest,
                "admin-1",
                &token("admin-1"),
            ),
            PlatformError::Unauthorized,
        );
        fixture.assert_empty_posting();
    }
}
