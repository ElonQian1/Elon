use anyhow::Result;
use serde_json::{json, Value};

use crate::open_commerce_app_block_model::OpenCommerceAppBlocked;

use super::test_support::{fixture, AUTHORIZED_QUERY, FREE_AUTHORIZED_QUERY, PUBLIC_QUERY};

const REQUEST_AUTHORIZATION: &str = "open_commerce_request_consumer_authorization";

fn call(
    fixture: &super::test_support::Fixture,
    user_id: &str,
    app_id: &str,
    uses_default_mcp_identity: bool,
    capability_key: &str,
    purpose: &str,
    confirmation_phrase: &str,
) -> Result<Value> {
    Ok(super::call_if_handled(
        &fixture.store,
        user_id,
        app_id,
        uses_default_mcp_identity,
        REQUEST_AUTHORIZATION,
        json!({
            "merchant_id":fixture.merchant_id,
            "capability_key":capability_key,
            "purpose":purpose,
            "confirmation_phrase":confirmation_phrase
        }),
    )?
    .unwrap())
}

#[test]
fn definition_and_identity_confirmation_guards_fail_closed_without_writes() {
    let fixture = fixture();
    let definition = super::definitions()
        .into_iter()
        .find(|definition| definition["name"] == REQUEST_AUTHORIZATION)
        .unwrap();
    assert_eq!(definition["annotations"]["readOnlyHint"], false);
    assert_eq!(definition["annotations"]["destructiveHint"], false);
    assert_eq!(definition["annotations"]["idempotentHint"], true);
    assert_eq!(
        definition["inputSchema"]["properties"]["confirmation_phrase"]["const"],
        "REQUEST_AUTHORIZATION"
    );

    let before = authorization_and_audit_counts(&fixture);
    let default_identity = call(
        &fixture,
        &fixture.consumer_id,
        "mcp-client",
        true,
        AUTHORIZED_QUERY,
        "申请菜单查询授权",
        "REQUEST_AUTHORIZATION",
    )
    .unwrap_err();
    assert!(default_identity.to_string().contains("x-elon-app-id"));

    let wrong_phrase = call(
        &fixture,
        &fixture.consumer_id,
        &fixture.app_id,
        false,
        AUTHORIZED_QUERY,
        "申请菜单查询授权",
        "YES",
    )
    .unwrap_err();
    assert!(wrong_phrase.to_string().contains("确认短语无效"));

    let other_app = call(
        &fixture,
        &fixture.consumer_id,
        &fixture.other_app_id,
        false,
        AUTHORIZED_QUERY,
        "申请菜单查询授权",
        "REQUEST_AUTHORIZATION",
    )
    .unwrap_err();
    assert!(other_app
        .to_string()
        .contains("当前用户不能代表该开发者应用"));
    assert_eq!(before, authorization_and_audit_counts(&fixture));
}

#[tokio::test]
async fn first_request_routes_through_mcp_replays_once_and_returns_safe_projection() {
    let fixture = fixture();
    let before = business_counts(&fixture);
    let params = json!({
        "name":REQUEST_AUTHORIZATION,
        "arguments":{
            "merchant_id":fixture.merchant_id,
            "capability_key":AUTHORIZED_QUERY,
            "purpose":"用于比较当前菜单",
            "confirmation_phrase":"REQUEST_AUTHORIZATION"
        }
    });
    let first = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        &fixture.app_id,
        params.clone(),
    )
    .await
    .unwrap();
    let replay = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        &fixture.app_id,
        params,
    )
    .await
    .unwrap();
    let first = &first["structuredContent"];
    let replay = &replay["structuredContent"];
    assert_eq!(first["request_id"], replay["request_id"]);
    assert_eq!(first["status"], "pending");
    assert_eq!(first["merchant_id"], fixture.merchant_id);
    assert_eq!(first["requester_app_id"], fixture.app_id);
    assert_eq!(first["scopes"], json!([AUTHORIZED_QUERY]));
    assert!(first["grant_id"].is_null());

    let serialized = first.to_string();
    for field in [
        "merchant_project_id",
        "requester_user_id",
        "decided_by_user_id",
        "test_token",
        "token_hint",
    ] {
        assert!(!serialized.contains(field), "leaked field {field}");
    }
    for identity in [
        fixture.merchant_project_id.as_str(),
        fixture.merchant_owner_id.as_str(),
        fixture.consumer_id.as_str(),
        fixture.other_user_id.as_str(),
    ] {
        assert!(!serialized.contains(identity), "leaked identity {identity}");
    }

    let requests = fixture
        .store
        .list_project_open_commerce_authorization_requests(&fixture.merchant_project_id, 100)
        .unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].status, "pending");
    assert!(requests[0].grant_id.is_none());
    assert_eq!(
        audit_count(&fixture, "authorization.requested", &requests[0].id),
        1
    );
    let after = business_counts(&fixture);
    assert_eq!(
        after.authorization_requests,
        before.authorization_requests + 1
    );
    assert_eq!(after.audit_events, before.audit_events + 1);
    assert_eq!(after.grants, before.grants);
    assert_eq!(after.action_confirmations, before.action_confirmations);
    assert_eq!(after.invocations, before.invocations);
    assert_eq!(after.budget_reservations, before.budget_reservations);
}

#[test]
fn public_missing_unpublished_disabled_and_blocked_targets_fail_closed() {
    let public = fixture();
    let public_error = call(
        &public,
        &public.consumer_id,
        &public.app_id,
        false,
        PUBLIC_QUERY,
        "不应申请公开能力",
        "REQUEST_AUTHORIZATION",
    )
    .unwrap_err();
    assert!(public_error
        .to_string()
        .contains("只有 authorized 能力可以申请授权"));

    let missing = fixture();
    let missing_error = call(
        &missing,
        &missing.consumer_id,
        &missing.app_id,
        false,
        "missing.capability",
        "申请不存在能力",
        "REQUEST_AUTHORIZATION",
    )
    .unwrap_err();
    assert!(missing_error.to_string().contains("商户未发布该商业能力"));

    let unpublished = fixture();
    unpublished.set_published(false);
    let unpublished_error = call(
        &unpublished,
        &unpublished.consumer_id,
        &unpublished.app_id,
        false,
        AUTHORIZED_QUERY,
        "申请未发布商户能力",
        "REQUEST_AUTHORIZATION",
    )
    .unwrap_err();
    assert!(unpublished_error
        .to_string()
        .contains("商户节点未发布到开放目录"));

    let disabled = fixture();
    disabled
        .store
        .disable_open_commerce_developer_app(&disabled.consumer_project_id, &disabled.app_record_id)
        .unwrap();
    let disabled_error = call(
        &disabled,
        &disabled.consumer_id,
        &disabled.app_id,
        false,
        AUTHORIZED_QUERY,
        "停用应用不应申请",
        "REQUEST_AUTHORIZATION",
    )
    .unwrap_err();
    assert!(disabled_error.to_string().contains("开发者应用已停用"));

    let blocked = fixture();
    blocked.block_app();
    let blocked_error = call(
        &blocked,
        &blocked.consumer_id,
        &blocked.app_id,
        false,
        AUTHORIZED_QUERY,
        "封禁应用不应申请",
        "REQUEST_AUTHORIZATION",
    )
    .unwrap_err();
    assert!(blocked_error.is::<OpenCommerceAppBlocked>());
    assert_eq!(
        blocked
            .store
            .list_project_open_commerce_authorization_requests(&blocked.merchant_project_id, 100,)
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn any_available_grant_blocks_duplicate_requests_including_free_calls() {
    let available = fixture();
    available.grant(AUTHORIZED_QUERY, Some(5), Some(5_000), "CNY");
    assert_active_grant_rejects(&available, AUTHORIZED_QUERY);

    let multi = fixture();
    let old_available = multi.grant(AUTHORIZED_QUERY, Some(5), Some(5_000), "CNY");
    multi.set_grant_created_at(&old_available.id, "2025-01-01T00:00:00Z");
    let latest_exhausted = multi.grant(AUTHORIZED_QUERY, Some(1), Some(5_000), "CNY");
    multi.set_grant_usage(&latest_exhausted.id, 1, 1_000);
    multi.set_grant_created_at(&latest_exhausted.id, "2026-01-01T00:00:00Z");
    assert_active_grant_rejects(&multi, AUTHORIZED_QUERY);

    let free = fixture();
    let free_grant = free.grant(FREE_AUTHORIZED_QUERY, None, Some(1), "CNY");
    free.set_grant_usage(&free_grant.id, 0, 1);
    assert_active_grant_rejects(&free, FREE_AUTHORIZED_QUERY);
}

#[test]
fn exhausted_or_currency_mismatched_grants_allow_pending_refresh_without_mutation() {
    for (kind, max_calls, max_amount, currency, used_calls, used_amount) in [
        ("count", Some(1), Some(5_000), "CNY", 1, 0),
        ("amount", None, Some(1_000), "CNY", 0, 1),
        ("currency", None, Some(5_000), "USD", 0, 0),
    ] {
        let fixture = fixture();
        let grant = fixture.grant(AUTHORIZED_QUERY, max_calls, max_amount, currency);
        fixture.set_grant_usage(&grant.id, used_calls, used_amount);
        let before = fixture.store.open_commerce_grant(&grant.id).unwrap();
        let authorization = call(
            &fixture,
            &fixture.consumer_id,
            &fixture.app_id,
            false,
            AUTHORIZED_QUERY,
            &format!("{kind} 预算续额申请"),
            "REQUEST_AUTHORIZATION",
        )
        .unwrap();
        assert_eq!(authorization["status"], "pending");
        let after = fixture.store.open_commerce_grant(&grant.id).unwrap();
        assert_eq!(after.used_invocations, before.used_invocations);
        assert_eq!(after.used_amount_micros, before.used_amount_micros);
        assert_eq!(after.max_invocations, before.max_invocations);
        assert_eq!(after.max_amount_micros, before.max_amount_micros);
        assert_eq!(after.budget_currency, before.budget_currency);
        assert!(after.revoked_at.is_none());
    }
}

#[test]
fn a_different_pending_request_cannot_be_silently_rewritten() {
    let fixture = fixture();
    let first = call(
        &fixture,
        &fixture.consumer_id,
        &fixture.app_id,
        false,
        AUTHORIZED_QUERY,
        "用于比较当前菜单",
        "REQUEST_AUTHORIZATION",
    )
    .unwrap();
    let changed_purpose = call(
        &fixture,
        &fixture.consumer_id,
        &fixture.app_id,
        false,
        AUTHORIZED_QUERY,
        "改成自动分析全部菜单",
        "REQUEST_AUTHORIZATION",
    )
    .unwrap_err();
    assert!(changed_purpose.to_string().contains("已有待处理授权请求"));
    let requests = fixture
        .store
        .list_project_open_commerce_authorization_requests(&fixture.merchant_project_id, 100)
        .unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].id, first["request_id"].as_str().unwrap());
    assert_eq!(requests[0].purpose, "用于比较当前菜单");
}

fn assert_active_grant_rejects(fixture: &super::test_support::Fixture, capability_key: &str) {
    let before = authorization_and_audit_counts(fixture);
    let error = call(
        fixture,
        &fixture.consumer_id,
        &fixture.app_id,
        false,
        capability_key,
        "不应重复申请有效授权",
        "REQUEST_AUTHORIZATION",
    )
    .unwrap_err();
    assert!(error.to_string().contains("有效授权，无需重复申请"));
    assert_eq!(before, authorization_and_audit_counts(fixture));
}

fn authorization_and_audit_counts(fixture: &super::test_support::Fixture) -> (i64, i64) {
    let counts = business_counts(fixture);
    (counts.authorization_requests, counts.audit_events)
}

#[derive(Debug, PartialEq, Eq)]
struct BusinessCounts {
    authorization_requests: i64,
    grants: i64,
    action_confirmations: i64,
    invocations: i64,
    budget_reservations: i64,
    audit_events: i64,
}

fn business_counts(fixture: &super::test_support::Fixture) -> BusinessCounts {
    let conn = fixture.store.conn().unwrap();
    BusinessCounts {
        authorization_requests: table_count(&conn, "open_commerce_authorization_requests"),
        grants: table_count(&conn, "open_commerce_grants"),
        action_confirmations: table_count(&conn, "open_commerce_action_confirmations"),
        invocations: table_count(&conn, "open_commerce_invocations"),
        budget_reservations: table_count(&conn, "open_commerce_grant_budget_reservations"),
        audit_events: table_count(&conn, "open_commerce_audit_events"),
    }
}

fn table_count(conn: &rusqlite::Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .unwrap()
}

fn audit_count(fixture: &super::test_support::Fixture, action: &str, subject_id: &str) -> i64 {
    fixture
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM open_commerce_audit_events
              WHERE action=?1 AND subject_id=?2",
            rusqlite::params![action, subject_id],
            |row| row.get(0),
        )
        .unwrap()
}
