//! Only synthetic accounts in temporary production Store + Router instances; never network writes.
use super::{enable_fixture_policy, request, Fixture};
use crate::esk_asset::platform::sellback::{
    api::{override_configuration, ConfigurationGuard},
    *,
};
use axum::http::StatusCode;
use serde_json::{json, Value};

const BASE: &str = "/api/me/assets/esk/platform/sellback-requests";
#[path = "http_test_support.rs"]
mod support;
use support::*;
#[path = "http_boundary_tests.rs"]
mod boundary;
#[path = "http_paging_tests.rs"]
mod paging;

#[tokio::test]
async fn empty_disabled_read_is_formal_and_never_uses_paper() {
    let fixture = Fixture::new();
    let _disabled = override_configuration(SellbackConfiguration::Disabled);
    let response = page(&fixture).await;
    assert_base(&response, true);
    assert_summary(&response["summary"], "0", "0", "0", "0");
    assert_eq!(response["summary"]["new_requests_enabled"], false);
    assert_eq!(response["summary"]["unavailable_reason"], "disabled");
    assert_eq!(response["summary"]["policy"], Value::Null);
    assert_eq!(response["requests"], json!([]));
    assert_eq!(response["range_start"], "0");
    assert_eq!(response["range_end"], "0");
    assert_eq!(response["has_more"], false);
    assert_eq!(response["next_cursor"], Value::Null);
    assert_eq!(
        fixture
            .state
            .store
            .esk_account_ledger(&fixture.user_id)
            .unwrap()
            .total_base_units,
        9_000_000
    );
    fixture.cleanup();
}

#[tokio::test]
async fn formal_request_lookup_cancel_and_replay_preserve_total_and_old_contract() {
    let fixture = Fixture::new();
    let _allocation_policy = enable_fixture_policy();
    credit(&fixture).await;
    let (_configuration, policy) = configure(&fixture);
    let initial = page(&fixture).await;
    let public = &initial["summary"]["policy"];
    keys(
        public,
        &[
            "policy_digest",
            "revision",
            "terms_digest",
            "terms_text",
            "min_request_base_units",
            "max_request_base_units",
            "max_open_requests_per_user",
            "max_reserved_base_units_per_user",
            "hold_mode",
            "cancel_mode",
            "expiry_mode",
            "participation_effect",
            "disabled_account_recovery_text",
        ],
    );
    assert_eq!(
        public["terms_digest"],
        text_digest(public["terms_text"].as_str().unwrap())
    );
    let body = submit_body(&initial, "synthetic-key-one", "10000000");
    let (status, accepted) = send(
        &fixture,
        "POST",
        BASE,
        Some(&fixture.user_token),
        body.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_base(&accepted, false);
    assert_record(&accepted["request"]);
    assert_summary(
        &accepted["summary"],
        "25000000",
        "10000000",
        "15000000",
        "1",
    );
    assert_eq!(accepted["summary"]["open_request_count"], "1");
    assert_eq!(accepted["replayed"], false);
    let id = accepted["request"]["request_id"].as_str().unwrap();
    let (status, detail) = send(
        &fixture,
        "GET",
        &format!("{BASE}/{id}"),
        Some(&fixture.user_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["request"], accepted["request"]);
    let lookup = json!({"schema":LOOKUP_SCHEMA,"idempotency_key":"synthetic-key-one"});
    let (status, recovered) = send(
        &fixture,
        "POST",
        &format!("{BASE}/lookup"),
        Some(&fixture.user_token),
        lookup,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(recovered["request"], accepted["request"]);
    assert_eq!(recovered["replayed"], true);
    assert_eq!(recovered["summary"], accepted["summary"]);
    let (status, canceled) = send(
        &fixture,
        "POST",
        &format!("{BASE}/{id}/cancel"),
        Some(&fixture.user_token),
        cancel_body(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(canceled["request"]["status"], "canceled");
    assert_summary(&canceled["summary"], "25000000", "0", "25000000", "1");
    assert_eq!(canceled["summary"]["open_request_count"], "0");
    let (status, replay) = send(&fixture, "POST", BASE, Some(&fixture.user_token), body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["request"], canceled["request"]);
    assert_eq!(replay["summary"], canceled["summary"]);
    let (_, again) = send(
        &fixture,
        "POST",
        &format!("{BASE}/{id}/cancel"),
        Some(&fixture.user_token),
        cancel_body(),
    )
    .await;
    assert_eq!(again["replayed"], true);
    assert_eq!(again["request"], canceled["request"]);
    let (_, account) = request(
        &fixture.router,
        "GET",
        "/api/me/assets/esk/platform",
        Some(&fixture.user_token),
        Value::Null,
    )
    .await;
    assert_eq!(account.as_object().unwrap().len(), 18);
    assert_eq!(account["total"], "25.000000");
    assert_eq!(account["capabilities"]["sellback_settlement"], false);
    let response_text = accepted.to_string();
    for private in [
        &fixture.user_id,
        &fixture.user_token,
        &policy.body.approval_digest,
    ] {
        assert!(!response_text.contains(private));
    }
    assert_eq!(
        fixture
            .state
            .store
            .esk_account_ledger(&fixture.user_id)
            .unwrap()
            .total_base_units,
        9_000_000
    );
    fixture.cleanup();
}

#[tokio::test]
async fn same_key_conflict_and_stale_snapshot_have_distinct_errors() {
    let fixture = Fixture::new();
    let _allocation_policy = enable_fixture_policy();
    credit(&fixture).await;
    let (_configuration, _) = configure(&fixture);
    let initial = page(&fixture).await;
    let body = submit_body(&initial, "one", "10000000");
    assert_eq!(
        send(
            &fixture,
            "POST",
            BASE,
            Some(&fixture.user_token),
            body.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    let mut changed = body.clone();
    changed["amount_base_units"] = "1".into();
    let (status, error) = send(&fixture, "POST", BASE, Some(&fixture.user_token), changed).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"], "ESK_PLATFORM_SELLBACK_CONFLICT");
    let stale = submit_body(&initial, "two", "1");
    let (status, error) = send(&fixture, "POST", BASE, Some(&fixture.user_token), stale).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"], "ESK_PLATFORM_SELLBACK_SNAPSHOT_CHANGED");
    assert_summary(
        &page(&fixture).await["summary"],
        "25000000",
        "10000000",
        "15000000",
        "1",
    );
    fixture.cleanup();
}

#[tokio::test]
async fn disabled_or_invalid_current_config_never_traps_old_requests_or_replays() {
    for configuration in [
        SellbackConfiguration::Disabled,
        SellbackConfiguration::Invalid,
    ] {
        let fixture = Fixture::new();
        let _allocation_policy = enable_fixture_policy();
        credit(&fixture).await;
        let (_enabled, _) = configure(&fixture);
        let initial = page(&fixture).await;
        let original = submit_body(&initial, "recover-me", "10000000");
        let (status, accepted) = send(
            &fixture,
            "POST",
            BASE,
            Some(&fixture.user_token),
            original.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let _changed = override_configuration(configuration);
        let closed = page(&fixture).await;
        assert_eq!(closed["summary"]["policy"], Value::Null);
        assert_eq!(closed["summary"]["new_requests_enabled"], false);
        let (status, replay) = send(
            &fixture,
            "POST",
            BASE,
            Some(&fixture.user_token),
            original.clone(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(replay["request"], accepted["request"]);
        let mut fresh = original;
        fresh["idempotency_key"] = "new-key".into();
        fresh["expected_snapshot_digest"] = closed["summary"]["snapshot_digest"].clone();
        let (status, error) = send(&fixture, "POST", BASE, Some(&fixture.user_token), fresh).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error["error"], "ESK_PLATFORM_SELLBACK_DISABLED");
        let id = accepted["request"]["request_id"].as_str().unwrap();
        let (status, canceled) = send(
            &fixture,
            "POST",
            &format!("{BASE}/{id}/cancel"),
            Some(&fixture.user_token),
            cancel_body(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_summary(&canceled["summary"], "25000000", "0", "25000000", "1");
        fixture.cleanup();
    }
}

#[tokio::test]
async fn changed_policy_requires_new_terms_but_preserves_old_recovery_and_cancel() {
    let fixture = Fixture::new();
    let _allocation_policy = enable_fixture_policy();
    credit(&fixture).await;
    let (_enabled, policy) = configure(&fixture);
    let initial = page(&fixture).await;
    let original = submit_body(&initial, "old-terms", "10");
    let (_, accepted) = send(
        &fixture,
        "POST",
        BASE,
        Some(&fixture.user_token),
        original.clone(),
    )
    .await;
    let mut replacement = policy.body;
    replacement.revision = "synthetic-v2".into();
    replacement.terms_text = "Updated synthetic request terms, still no settlement.".into();
    replacement.terms_digest = text_digest(&replacement.terms_text);
    let _changed = override_configuration(SellbackConfiguration::Enabled(
        validate_policy(replacement).unwrap(),
    ));
    let current = page(&fixture).await;
    let mut old_terms = original.clone();
    old_terms["idempotency_key"] = "new-with-old-terms".into();
    old_terms["expected_snapshot_digest"] = current["summary"]["snapshot_digest"].clone();
    let (status, error) = send(&fixture, "POST", BASE, Some(&fixture.user_token), old_terms).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"], "ESK_PLATFORM_SELLBACK_POLICY_CHANGED");
    let (status, replay) = send(&fixture, "POST", BASE, Some(&fixture.user_token), original).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["request"], accepted["request"]);
    let id = accepted["request"]["request_id"].as_str().unwrap();
    assert_eq!(
        send(
            &fixture,
            "POST",
            &format!("{BASE}/{id}/cancel"),
            Some(&fixture.user_token),
            cancel_body()
        )
        .await
        .0,
        StatusCode::OK
    );
    fixture.cleanup();
}
