use serde_json::{json, Value};

use crate::{
    open_commerce_action_confirmation_model::ACTION_CONFIRMATION_PHRASE,
    open_commerce_action_confirmation_service,
    open_commerce_service::{self, OpenCommerceActor},
};

use super::test_support::{action_request, fixture, Fixture};

async fn read_confirmation(fixture: &Fixture, confirmation_id: &str) -> Value {
    crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_get_my_action_confirmation",
            "arguments":{"confirmation_id":confirmation_id}
        }),
    )
    .await
    .unwrap()["structuredContent"]
        .clone()
}

#[tokio::test]
async fn mcp_read_projects_pending_confirmed_and_consumed_states() {
    let fixture = fixture();
    let actor = OpenCommerceActor {
        user_id: &fixture.owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };

    let pending = open_commerce_action_confirmation_service::prepare(
        &fixture.store,
        &actor,
        action_request(&fixture, "read-pending-action"),
    )
    .unwrap();
    let pending_projection = read_confirmation(&fixture, &pending.id).await;
    assert_eq!(pending_projection["status"], "pending");
    assert_eq!(
        pending_projection["next_step"],
        "obtain_explicit_user_confirmation"
    );

    let confirmed = open_commerce_action_confirmation_service::prepare(
        &fixture.store,
        &actor,
        action_request(&fixture, "read-confirmed-action"),
    )
    .unwrap();
    open_commerce_action_confirmation_service::confirm(
        &fixture.store,
        &actor,
        &confirmed.id,
        ACTION_CONFIRMATION_PHRASE,
    )
    .unwrap();
    let confirmed_projection = read_confirmation(&fixture, &confirmed.id).await;
    assert_eq!(confirmed_projection["status"], "confirmed");
    assert_eq!(
        confirmed_projection["next_step"],
        "invoke_with_confirmation"
    );

    let consumed_request = action_request(&fixture, "read-consumed-action");
    let consumed = open_commerce_action_confirmation_service::prepare(
        &fixture.store,
        &actor,
        consumed_request.clone(),
    )
    .unwrap();
    open_commerce_action_confirmation_service::confirm(
        &fixture.store,
        &actor,
        &consumed.id,
        ACTION_CONFIRMATION_PHRASE,
    )
    .unwrap();
    open_commerce_service::invoke_with_action_confirmation(
        &fixture.store,
        &actor,
        consumed_request,
        Some(&consumed.id),
    )
    .await
    .unwrap();
    let consumed_projection = read_confirmation(&fixture, &consumed.id).await;
    assert_eq!(consumed_projection["status"], "consumed");
    assert_eq!(consumed_projection["next_step"], "read_invocation_receipt");
    assert!(consumed_projection["invocation_id"].is_string());
}

#[tokio::test]
async fn mcp_read_derives_expiry_without_mutating_confirmation_or_creating_invocation() {
    let fixture = fixture();
    let actor = OpenCommerceActor {
        user_id: &fixture.owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let prepared = open_commerce_action_confirmation_service::prepare(
        &fixture.store,
        &actor,
        action_request(&fixture, "read-expired-action"),
    )
    .unwrap();
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_action_confirmations
             SET expires_at='2000-01-01T00:00:00Z' WHERE id=?1",
            [&prepared.id],
        )
        .unwrap();
    let audits_before = fixture
        .store
        .list_project_open_commerce_audit(&fixture.project_id, 200)
        .unwrap()
        .len();

    let projection = read_confirmation(&fixture, &prepared.id).await;
    assert_eq!(projection["status"], "expired");
    assert_eq!(projection["stored_status"], "pending");
    assert_eq!(projection["next_step"], "prepare_new_confirmation");
    let persisted = fixture
        .store
        .open_commerce_action_confirmation(&prepared.id)
        .unwrap();
    assert_eq!(persisted.status, "pending");
    assert!(persisted.canceled_at.is_none());
    assert!(fixture
        .store
        .list_project_open_commerce_invocations(&fixture.project_id, 20)
        .unwrap()
        .is_empty());
    assert_eq!(
        fixture
            .store
            .list_project_open_commerce_audit(&fixture.project_id, 200)
            .unwrap()
            .len(),
        audits_before
    );
}

#[tokio::test]
async fn mcp_read_hides_input_and_internal_identity_and_does_not_enumerate() {
    let fixture = fixture();
    let actor = OpenCommerceActor {
        user_id: &fixture.owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let prepared = open_commerce_action_confirmation_service::prepare(
        &fixture.store,
        &actor,
        action_request(&fixture, "read-private-action"),
    )
    .unwrap();

    let projection = read_confirmation(&fixture, &prepared.id).await;
    assert_eq!(projection["contains_raw_input_values"], false);
    let serialized = projection.to_string();
    assert!(!serialized.contains("do-not-store-this-value"));
    for forbidden in [
        "request_hash",
        "requester_user_id",
        "project_id",
        "capability_id",
        "input",
    ] {
        assert!(projection.get(forbidden).is_none(), "leaked {forbidden}");
    }

    let other = fixture
        .store
        .create_user("read-other@example.com", "secret1", None, None)
        .unwrap();
    for (user_id, app_id, confirmation_id) in [
        (other.id.as_str(), "pc-web", prepared.id.as_str()),
        (fixture.owner_id.as_str(), "other-app", prepared.id.as_str()),
        (fixture.owner_id.as_str(), "pc-web", "missing-confirmation"),
    ] {
        let error = crate::open_commerce_action_confirmation_mcp::call_if_handled(
            &fixture.store,
            user_id,
            "owner",
            app_id,
            "open_commerce_get_my_action_confirmation",
            json!({"confirmation_id":confirmation_id}),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("不存在"));
    }
}
