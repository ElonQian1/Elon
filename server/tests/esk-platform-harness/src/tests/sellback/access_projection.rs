use super::*;
use crate::esk_asset::platform::{access::*, DelegatedAssetPage};
use rusqlite::params;

const CLIENT: &str = "quant.android";
const ORIGIN: &str = "https://main.example.test";
const REDIRECT: &str = "com.elon.quant:/asset-access/callback";

fn grant(fixture: &Fixture, user: &str, progress: bool) -> AccessToken {
    let verifier = "v".repeat(43);
    let mut scopes = vec![AccessScope::EskSummaryRead];
    if progress {
        scopes.push(AccessScope::EskProgressRead);
    }
    let code = fixture
        .store
        .authorize_asset_access(
            user,
            &token(user),
            &AuthorizeBody {
                schema: AUTHORIZE_SCHEMA.into(),
                client_id: CLIENT.into(),
                redirect_uri: REDIRECT.into(),
                state: "s".repeat(32),
                code_challenge: challenge(&verifier).unwrap(),
                code_challenge_method: "S256".into(),
                scopes,
                expires_in: 600,
                explicit_consent: true,
                confirmation: AUTHORIZE_CONFIRMATION.into(),
            },
            ORIGIN,
        )
        .unwrap();
    fixture
        .store
        .exchange_asset_access_code(
            &TokenBody {
                schema: TOKEN_SCHEMA.into(),
                grant_type: "authorization_code".into(),
                client_id: CLIENT.into(),
                redirect_uri: REDIRECT.into(),
                state: code.state,
                code: code.code,
                code_verifier: verifier,
            },
            ORIGIN,
        )
        .unwrap()
}

fn delegated(
    fixture: &Fixture,
    grant: &AccessToken,
    limit: usize,
    cursor: Option<&str>,
    progress: bool,
    config: &SellbackConfiguration,
) -> anyhow::Result<DelegatedAssetPage> {
    fixture
        .store
        .asset_access_esk(&grant.access_token, CLIENT, limit, cursor, progress, config)
}

fn access_error(result: anyhow::Result<DelegatedAssetPage>, expected: AccessError) {
    let error = result.unwrap_err();
    assert_eq!(error.downcast_ref::<AccessError>(), Some(&expected));
}

#[test]
fn delegated_matches_existing_session_scan_and_never_writes_on_reads() {
    let (fixture, _, config) = setup();
    let canceled = submit(&fixture, "alice", "hidden-idempotency-one", 2, &config);
    submit(&fixture, "alice", "hidden-idempotency-two", 3, &config);
    fixture
        .store
        .cancel_esk_platform_sellback(
            "alice",
            &token("alice"),
            &canceled.request.request_id,
            &config,
        )
        .unwrap();
    let authorization = grant(&fixture, "alice", true);
    let before = std::fs::read(&fixture.path).unwrap();
    let existing = page(&fixture, "alice", &config);
    let response = delegated(&fixture, &authorization, 20, None, true, &config).unwrap();
    assert_eq!(response.snapshot_digest, existing.summary.snapshot_digest);
    assert_eq!(response.subject, authorization.subject);
    assert_eq!(response.client_id, CLIENT);
    assert_eq!(response.balance.total_base_units, "10000000");
    assert_eq!(response.balance.reserved_base_units, "3");
    assert_eq!(response.balance.available_base_units, "9999997");
    let progress = response.progress.as_ref().unwrap();
    assert_eq!(progress.request_count, "2");
    assert_eq!(progress.open_count, "1");
    assert!(progress.requests.iter().any(|r| r.status == "canceled"));
    let encoded = serde_json::to_value(&response).unwrap();
    assert_eq!(encoded["asset"]["source"], "platform_recorded");
    assert_eq!(encoded["asset"]["simulated"], false);
    assert_eq!(encoded["asset"]["funds_moved"], false);
    for forbidden in [
        "user_id",
        "grant_id",
        "idempotency",
        "terms",
        "payment",
        "alice",
    ] {
        assert!(!encoded.to_string().contains(forbidden));
    }
    assert_eq!(
        delegated(&fixture, &authorization, 20, None, true, &config).unwrap(),
        response
    );
    assert_eq!(std::fs::read(&fixture.path).unwrap(), before);
    error(
        fixture
            .store
            .esk_platform_sellback_page("alice", &token("bob"), 20, None, &config),
        SellbackError::Unauthorized,
    );
    assert!(fixture
        .store
        .esk_platform_history("alice", &authorization.access_token, 20, None)
        .is_err());
}

#[test]
fn summary_only_scope_has_no_paging_path_or_record_disclosure() {
    let (fixture, _, config) = setup();
    for key in ["first", "second"] {
        submit(&fixture, "alice", key, 1, &config);
    }
    let authorization = grant(&fixture, "alice", false);
    let summary = delegated(&fixture, &authorization, 20, None, false, &config).unwrap();
    assert!(summary.progress.is_none());
    assert!(serde_json::to_value(summary)
        .unwrap()
        .get("progress")
        .is_none());
    access_error(
        delegated(&fixture, &authorization, 1, None, true, &config),
        AccessError::InsufficientScope,
    );
    access_error(
        delegated(&fixture, &authorization, 1, Some(""), false, &config),
        AccessError::InvalidInput,
    );
    for limit in [0, 21, usize::MAX] {
        access_error(
            delegated(&fixture, &authorization, limit, None, false, &config),
            AccessError::InvalidInput,
        );
    }
}

#[test]
fn zero_formal_balance_never_includes_existing_paper_allocation() {
    let fixture = Fixture::new();
    let authorization = grant(&fixture, "alice", true);
    let before = std::fs::read(&fixture.path).unwrap();
    let value = delegated(
        &fixture,
        &authorization,
        20,
        None,
        true,
        &SellbackConfiguration::Disabled,
    )
    .unwrap();
    assert_eq!(value.balance.total_base_units, "0");
    assert_eq!(value.balance.reserved_base_units, "0");
    assert_eq!(value.balance.available_base_units, "0");
    let progress = value.progress.unwrap();
    assert!(progress.requests.is_empty());
    assert_eq!(progress.range_start, "0");
    assert_eq!(progress.range_end, "0");
    assert!(!progress.has_more);
    assert_eq!(fixture.paper_total(), 123000000);
    assert_eq!(std::fs::read(&fixture.path).unwrap(), before);
}

#[test]
fn balances_above_javascript_safe_integer_remain_exact_strings() {
    let fixture = Fixture::new();
    let formal = policy(i64::MAX);
    let mut body = super::super::body();
    body.amount = "9007199254.740993".into();
    body.payment_amount = "18014398509.481986".into();
    let input = crate::esk_asset::platform::prepare_input(&formal, body).unwrap();
    let prepared = fixture
        .store
        .prepare_esk_platform_allocation(&formal, &input, "admin-1", &token("admin-1"))
        .unwrap();
    super::super::record(&fixture, &formal, &prepared);
    let authorization = grant(&fixture, "alice", false);
    let value = delegated(
        &fixture,
        &authorization,
        20,
        None,
        false,
        &SellbackConfiguration::Disabled,
    )
    .unwrap();
    let encoded = serde_json::to_value(value).unwrap();
    assert_eq!(encoded["balance"]["total_base_units"], "9007199254740993");
    assert_eq!(
        encoded["balance"]["available_base_units"],
        "9007199254740993"
    );
    assert_eq!(encoded["balance"]["reserved_base_units"], "0");
}

#[test]
fn pagination_binds_subject_and_complete_snapshot_across_refreshes() {
    let (fixture, formal, config) = setup();
    for key in ["first", "second", "third"] {
        submit(&fixture, "alice", key, 1, &config);
    }
    submit(&fixture, "bob", "bob-one", 1, &config);
    let alice = grant(&fixture, "alice", true);
    let bob = grant(&fixture, "bob", true);
    let first = delegated(&fixture, &alice, 1, None, true, &config).unwrap();
    let cursor = first.progress.as_ref().unwrap().next_cursor.as_deref();
    let second = delegated(&fixture, &alice, 2, cursor, true, &config).unwrap();
    assert_eq!(first.balance, second.balance);
    assert_eq!(first.snapshot_digest, second.snapshot_digest);
    assert_eq!(second.progress.as_ref().unwrap().range_start, "2");
    assert_eq!(second.progress.as_ref().unwrap().range_end, "3");
    error(
        delegated(&fixture, &bob, 1, cursor, true, &config),
        SellbackError::SnapshotChanged,
    );
    let off_page = &second.progress.as_ref().unwrap().requests[1].request_id;
    fixture
        .store
        .cancel_esk_platform_sellback("alice", &token("alice"), off_page, &config)
        .unwrap();
    error(
        delegated(&fixture, &alice, 1, cursor, true, &config),
        SellbackError::SnapshotChanged,
    );
    let current = delegated(&fixture, &alice, 1, None, true, &config).unwrap();
    history::post(&fixture, &formal, "alice", 9);
    error(
        delegated(
            &fixture,
            &alice,
            1,
            current.progress.as_ref().unwrap().next_cursor.as_deref(),
            true,
            &config,
        ),
        SellbackError::SnapshotChanged,
    );
}

#[test]
fn private_read_rechecks_client_user_and_parent_session_state() {
    for change in [
        "UPDATE sessions SET revoked_at='2026-01-01T00:00:00Z' WHERE id='alice'",
        "UPDATE sessions SET expires_at='2000-01-01T00:00:00Z' WHERE id='alice'",
        "UPDATE sessions SET user_id='bob' WHERE id='alice'",
        "UPDATE users SET status='disabled' WHERE id='alice'",
    ] {
        let (fixture, _, config) = setup();
        let authorization = grant(&fixture, "alice", true);
        assert!(fixture
            .store
            .asset_access_esk(
                &authorization.access_token,
                "quant.web",
                20,
                None,
                true,
                &config
            )
            .is_err());
        assert!(fixture
            .store
            .asset_access_esk(&token("alice"), CLIENT, 20, None, true, &config)
            .is_err());
        fixture.store.conn().unwrap().execute_batch(change).unwrap();
        assert!(delegated(&fixture, &authorization, 20, None, true, &config).is_err());
        assert!(fixture
            .store
            .esk_platform_history("alice", &token("alice"), 20, None)
            .is_err());
    }
}

#[test]
fn grant_revocation_is_owner_bound_persistent_and_does_not_revoke_other_grants() {
    let (fixture, _, config) = setup();
    let first = grant(&fixture, "alice", true);
    let second = grant(&fixture, "alice", true);
    assert!(fixture
        .store
        .revoke_asset_access_grant("bob", &token("bob"), &first.grant_id)
        .is_err());
    assert!(delegated(&fixture, &first, 20, None, true, &config).is_ok());
    fixture
        .store
        .revoke_asset_access_grant("alice", &token("alice"), &first.grant_id)
        .unwrap();
    let before = std::fs::read(&fixture.path).unwrap();
    access_error(
        delegated(&fixture, &first, 20, None, true, &config),
        AccessError::Unauthorized,
    );
    // A fresh Store connection must observe persisted revocation after reopen.
    assert!(fixture
        .store
        .clone()
        .asset_access_esk(&first.access_token, CLIENT, 20, None, true, &config)
        .is_err());
    assert!(delegated(&fixture, &second, 20, None, true, &config).is_ok());
    assert_eq!(std::fs::read(&fixture.path).unwrap(), before);
    fixture
        .store
        .revoke_asset_access_token(&second.access_token, CLIENT)
        .unwrap();
    access_error(
        delegated(&fixture, &second, 20, None, true, &config),
        AccessError::Unauthorized,
    );
    assert!(fixture
        .store
        .esk_platform_history("alice", &token("alice"), 20, None)
        .is_ok());
}

#[test]
fn corruption_outside_requested_page_fails_summary_and_progress_reads() {
    let (fixture, _, config) = setup();
    for key in ["first", "second"] {
        submit(&fixture, "alice", key, 1, &config);
    }
    let authorization = grant(&fixture, "alice", true);
    let last = page(&fixture, "alice", &config).requests.pop().unwrap();
    let conn = fixture.store.conn().unwrap();
    conn.execute_batch("DROP TRIGGER IF EXISTS trg_esk_platform_sellback_requests_no_update")
        .unwrap();
    conn.execute(
        "UPDATE esk_platform_sellback_requests SET input_json='{}' WHERE request_id=?1",
        params![last.request_id],
    )
    .unwrap();
    for progress in [false, true] {
        error(
            delegated(&fixture, &authorization, 1, None, progress, &config),
            SellbackError::Corrupt,
        );
    }
}
