use anyhow::Result;
use serde_json::{json, Value};

use super::test_support::{fixture, Fixture};

const LIST_REQUESTS: &str = "open_commerce_list_my_consumer_authorization_requests";
const LIST_GRANTS: &str = "open_commerce_list_my_active_grants";
const CANCEL_REQUEST: &str = "open_commerce_cancel_my_consumer_authorization_request";

fn call(
    fixture: &Fixture,
    project_id: &str,
    user_id: &str,
    role: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    super::call_if_handled(
        &fixture.store,
        project_id,
        user_id,
        role,
        "pc-web",
        name,
        arguments,
    )
}

fn grant_by_id<'a>(projection: &'a Value, grant_id: &str) -> &'a Value {
    projection["grants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|grant| grant["grant_id"] == grant_id)
        .unwrap()
}

#[tokio::test]
async fn request_list_is_routed_filtered_and_identity_minimized() {
    let fixture = fixture();
    let definitions = super::definitions();
    assert_eq!(definitions.len(), 3);
    for definition in &definitions {
        let name = definition["name"].as_str().unwrap();
        if name == CANCEL_REQUEST {
            assert_eq!(definition["annotations"]["destructiveHint"], true);
            assert_eq!(definition["annotations"]["readOnlyHint"], false);
        } else {
            assert_eq!(definition["annotations"]["readOnlyHint"], true);
            assert_eq!(definition["annotations"]["destructiveHint"], false);
        }
    }

    let routed = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        "pc-web",
        json!({"name":LIST_REQUESTS,"arguments":{}}),
    )
    .await
    .unwrap();
    let projection = &routed["structuredContent"];
    assert_eq!(projection["count"], 4);
    let requests = projection["requests"].as_array().unwrap();
    for (request_id, status) in [
        (&fixture.pending_request_id, "pending"),
        (&fixture.approved_request_id, "approved"),
        (&fixture.rejected_request_id, "rejected"),
        (&fixture.canceled_request_id, "canceled"),
    ] {
        assert!(requests
            .iter()
            .any(|request| request["request_id"] == *request_id && request["status"] == status));
    }
    let approved = requests
        .iter()
        .find(|request| request["request_id"] == fixture.approved_request_id)
        .unwrap();
    assert_eq!(approved["grant_id"], fixture.approved_grant_id);
    assert_eq!(approved["grant_max_invocations"], 5);
    assert_eq!(approved["grant_max_amount_micros"], 5_000);
    assert_eq!(approved["grant_budget_currency"], "CNY");
    let rejected = requests
        .iter()
        .find(|request| request["request_id"] == fixture.rejected_request_id)
        .unwrap();
    assert_eq!(rejected["decision_reason"], "用途不清晰");

    let serialized = projection.to_string();
    for secret in [
        fixture.merchant_project_id.as_str(),
        fixture.merchant_owner_id.as_str(),
        fixture.consumer_id.as_str(),
        fixture.teammate_id.as_str(),
        fixture.teammate_app_id.as_str(),
        fixture.other_project_id.as_str(),
        fixture.test_token.as_str(),
    ] {
        assert!(!serialized.contains(secret), "leaked {secret}");
    }
    for field in [
        "merchant_project_id",
        "requester_user_id",
        "decided_by_user_id",
        "test_token",
        "token_hint",
    ] {
        assert!(!serialized.contains(field), "leaked field {field}");
    }

    for status in ["pending", "approved", "rejected", "canceled"] {
        let filtered = call(
            &fixture,
            &fixture.consumer_project_id,
            &fixture.consumer_id,
            "owner",
            LIST_REQUESTS,
            json!({"status":status,"limit":100}),
        )
        .unwrap()
        .unwrap();
        assert_eq!(filtered["count"], 1);
        assert_eq!(filtered["requests"][0]["status"], status);
    }
    let bounded = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        LIST_REQUESTS,
        json!({"limit":1}),
    )
    .unwrap()
    .unwrap();
    assert_eq!(bounded["count"], 1);
    let invalid = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        LIST_REQUESTS,
        json!({"status":"unknown"}),
    )
    .unwrap_err();
    assert!(invalid.to_string().contains("状态筛选无效"));
}

#[test]
fn active_grants_report_finite_exhausted_and_unlimited_budgets_without_writes() -> Result<()> {
    let fixture = fixture();
    let tracked_ids = [
        &fixture.approved_grant_id,
        &fixture.count_exhausted_grant_id,
        &fixture.amount_exhausted_grant_id,
        &fixture.unlimited_grant_id,
        &fixture.second_app_grant_id,
        &fixture.expired_grant_id,
        &fixture.revoked_grant_id,
    ];
    let before = tracked_ids
        .iter()
        .map(|id| fixture.store.open_commerce_grant(id).unwrap())
        .collect::<Vec<_>>();
    let projection = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        LIST_GRANTS,
        json!({"limit":100}),
    )?
    .unwrap();

    assert_eq!(projection["count"], 5);
    assert_eq!(projection["requires_execution_plan_before_invoke"], true);
    let approved = grant_by_id(&projection, &fixture.approved_grant_id);
    assert_eq!(approved["budget_status"], "budget_available");
    assert_eq!(approved["remaining_invocations"], 3);
    assert_eq!(approved["remaining_amount_micros"], 3_000);
    assert_eq!(
        grant_by_id(&projection, &fixture.count_exhausted_grant_id)["budget_status"],
        "budget_exhausted"
    );
    assert_eq!(
        grant_by_id(&projection, &fixture.amount_exhausted_grant_id)["budget_status"],
        "budget_exhausted"
    );
    let unlimited = grant_by_id(&projection, &fixture.unlimited_grant_id);
    assert_eq!(unlimited["budget_status"], "budget_available");
    assert!(unlimited["max_invocations"].is_null());
    assert!(unlimited["remaining_invocations"].is_null());
    assert!(unlimited["max_amount_micros"].is_null());
    assert!(unlimited["remaining_amount_micros"].is_null());
    assert!(projection["grants"]
        .as_array()
        .unwrap()
        .iter()
        .all(|grant| grant["grant_id"] != fixture.expired_grant_id
            && grant["grant_id"] != fixture.revoked_grant_id));

    let app_filtered = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        LIST_GRANTS,
        json!({"app_id":fixture.second_app_id,"limit":100}),
    )?
    .unwrap();
    assert_eq!(app_filtered["count"], 1);
    assert_eq!(
        app_filtered["grants"][0]["grant_id"],
        fixture.second_app_grant_id
    );

    let serialized = projection.to_string();
    for hidden in [
        fixture.consumer_project_id.as_str(),
        fixture.merchant_project_id.as_str(),
        fixture.consumer_id.as_str(),
        fixture.merchant_owner_id.as_str(),
        fixture.test_token.as_str(),
        "project_id",
        "grantor_user_id",
        "owner_user_id",
    ] {
        assert!(!serialized.contains(hidden), "leaked {hidden}");
    }
    let after = tracked_ids
        .iter()
        .map(|id| fixture.store.open_commerce_grant(id).unwrap())
        .collect::<Vec<_>>();
    for (before, after) in before.iter().zip(after.iter()) {
        assert_eq!(after.used_invocations, before.used_invocations);
        assert_eq!(after.used_amount_micros, before.used_amount_micros);
        assert_eq!(after.updated_at, before.updated_at);
        assert_eq!(after.revoked_at, before.revoked_at);
    }
    Ok(())
}

#[test]
fn cancellation_requires_owner_editor_confirmation_and_audits_once_per_side() -> Result<()> {
    let fixture = fixture();
    let wrong_phrase = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        CANCEL_REQUEST,
        json!({"request_id":fixture.pending_request_id,"confirmation_phrase":"cancel"}),
    )
    .unwrap_err();
    assert!(wrong_phrase.to_string().contains("确认短语无效"));

    let wrong_project = call(
        &fixture,
        &fixture.other_project_id,
        &fixture.consumer_id,
        "owner",
        CANCEL_REQUEST,
        json!({
            "request_id":fixture.pending_request_id,
            "confirmation_phrase":"CANCEL_AUTHORIZATION_REQUEST"
        }),
    )
    .unwrap_err();
    assert!(wrong_project.to_string().contains("不属于当前开发者项目"));
    let read_only = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "viewer",
        CANCEL_REQUEST,
        json!({
            "request_id":fixture.pending_request_id,
            "confirmation_phrase":"CANCEL_AUTHORIZATION_REQUEST"
        }),
    )
    .unwrap_err();
    assert!(read_only.to_string().contains("编辑权限"));
    let other_user = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.teammate_id,
        "editor",
        CANCEL_REQUEST,
        json!({
            "request_id":fixture.pending_request_id,
            "confirmation_phrase":"CANCEL_AUTHORIZATION_REQUEST"
        }),
    )
    .unwrap_err();
    assert!(other_user.to_string().contains("无权撤回"));

    let canceled = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        CANCEL_REQUEST,
        json!({
            "request_id":fixture.pending_request_id,
            "confirmation_phrase":"CANCEL_AUTHORIZATION_REQUEST"
        }),
    )?
    .unwrap();
    assert_eq!(canceled["status"], "canceled");
    assert_eq!(canceled["request_id"], fixture.pending_request_id);
    for hidden in [
        "merchant_project_id",
        "requester_user_id",
        "decided_by_user_id",
    ] {
        assert!(canceled.get(hidden).is_none(), "leaked {hidden}");
    }
    let replayed = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        CANCEL_REQUEST,
        json!({
            "request_id":fixture.pending_request_id,
            "confirmation_phrase":"CANCEL_AUTHORIZATION_REQUEST"
        }),
    )?
    .unwrap();
    assert_eq!(replayed["updated_at"], canceled["updated_at"]);

    let consumer_audits = fixture
        .store
        .list_project_open_commerce_audit(&fixture.consumer_project_id, 200)?
        .into_iter()
        .filter(|event| {
            event.action == "authorization.request_canceled"
                && event.subject_id == fixture.pending_request_id
        })
        .count();
    let merchant_audits = fixture
        .store
        .list_project_open_commerce_audit(&fixture.merchant_project_id, 200)?
        .into_iter()
        .filter(|event| {
            event.action == "authorization.canceled"
                && event.subject_id == fixture.pending_request_id
        })
        .count();
    assert_eq!(consumer_audits, 1);
    assert_eq!(merchant_audits, 1);

    let approved = call(
        &fixture,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        CANCEL_REQUEST,
        json!({
            "request_id":fixture.approved_request_id,
            "confirmation_phrase":"CANCEL_AUTHORIZATION_REQUEST"
        }),
    )?
    .unwrap();
    assert_eq!(approved["status"], "approved");
    assert!(fixture
        .store
        .open_commerce_grant(&fixture.approved_grant_id)?
        .revoked_at
        .is_none());
    Ok(())
}
