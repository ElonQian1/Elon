use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_action_confirmation_model::{
        ACTION_CONFIRMATION_PHRASE, MAX_ACTIVE_ACTION_CONFIRMATIONS_PER_APP,
    },
    open_commerce_action_confirmation_service,
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, InvokeCapabilityRequest, ACCESS_PUBLIC,
        HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_action_confirmation_{}.db",
        Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("action-confirmation test store should open")
}

struct Fixture {
    store: Store,
    owner_id: String,
    project_id: String,
    merchant_id: String,
}

fn fixture() -> Fixture {
    let store = temp_store();
    let owner = store
        .create_user(
            "action-confirmation-owner@example.com",
            "secret1",
            Some("Action Owner"),
            None,
        )
        .unwrap();
    let project = store
        .create_project(&owner.id, "Action Confirmation", None, None)
        .unwrap()
        .project;
    let actor = OpenCommerceActor {
        user_id: &owner.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let merchant = open_commerce_service::create_merchant(
        &store,
        &project.id,
        &actor,
        CreateMerchantRequest {
            display_name: "动作确认咖啡店".to_string(),
            slug: Some("action-confirmation-cafe".to_string()),
            description: String::new(),
            node_mode: "platform_hosted".to_string(),
            public_profile: json!({}),
        },
    )
    .unwrap();
    for (key, kind, response) in [
        ("order.commit", "action", json!({"order_id":"order-1"})),
        ("menu.lookup", "query", json!({"items":["latte"]})),
    ] {
        open_commerce_service::publish_capability(
            &store,
            &project.id,
            &merchant.id,
            &actor,
            CreateCapabilityRequest {
                capability_key: key.to_string(),
                display_name: key.to_string(),
                description: String::new(),
                kind: kind.to_string(),
                access_level: ACCESS_PUBLIC.to_string(),
                input_schema: json!({
                    "type":"object",
                    "properties":{"private_note":{"type":"string","maxLength":120}},
                    "additionalProperties":false
                }),
                output_schema: if kind == "action" {
                    json!({
                        "type":"object",
                        "required":["order_id"],
                        "properties":{"order_id":{"type":"string"}},
                        "additionalProperties":false
                    })
                } else {
                    json!({
                        "type":"object",
                        "required":["items"],
                        "properties":{"items":{"type":"array"}},
                        "additionalProperties":false
                    })
                },
                handler_type: HANDLER_STATIC_JSON.to_string(),
                handler_config: Some(json!({"response":response})),
                unit_price_micros: 1_000,
                currency: "CNY".to_string(),
                freshness_seconds: 30,
            },
        )
        .unwrap();
    }
    Fixture {
        store,
        owner_id: owner.id,
        project_id: project.id,
        merchant_id: merchant.id,
    }
}

fn action_request(fixture: &Fixture, idempotency_key: &str) -> InvokeCapabilityRequest {
    InvokeCapabilityRequest {
        merchant_id: fixture.merchant_id.clone(),
        capability_key: "order.commit".to_string(),
        requester_app_id: "pc-web".to_string(),
        grant_id: None,
        idempotency_key: idempotency_key.to_string(),
        input: json!({"private_note":"do-not-store-this-value"}),
    }
}

#[tokio::test]
async fn action_requires_short_lived_exact_input_confirmation_and_replays_once() {
    let fixture = fixture();
    let actor = OpenCommerceActor {
        user_id: &fixture.owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let request = action_request(&fixture, "action-confirmation-call-1");
    let missing = open_commerce_service::invoke(&fixture.store, &actor, request.clone())
        .await
        .unwrap_err();
    assert!(missing.to_string().contains("服务端一次性确认"));
    assert!(fixture
        .store
        .list_project_open_commerce_invocations(&fixture.project_id, 20)
        .unwrap()
        .is_empty());

    let prepared =
        open_commerce_action_confirmation_service::prepare(&fixture.store, &actor, request.clone())
            .unwrap();
    assert_eq!(prepared.status, "pending");
    let serialized = serde_json::to_string(&prepared).unwrap();
    assert!(!serialized.contains("do-not-store-this-value"));
    assert_eq!(prepared.request_shape["contains_raw_values"], false);
    let prepared_retry =
        open_commerce_action_confirmation_service::prepare(&fixture.store, &actor, request.clone())
            .unwrap();
    assert_eq!(prepared_retry.id, prepared.id);

    let mut changed_prepare = request.clone();
    changed_prepare.input = json!({"private_note":"changed-before-confirm"});
    let conflicting_prepare =
        open_commerce_action_confirmation_service::prepare(&fixture.store, &actor, changed_prepare)
            .unwrap_err();
    assert!(conflicting_prepare
        .to_string()
        .contains("相同幂等键不能用于不同输入或授权"));

    let wrong_phrase = open_commerce_action_confirmation_service::confirm(
        &fixture.store,
        &actor,
        &prepared.id,
        "yes",
    )
    .unwrap_err();
    assert!(wrong_phrase.to_string().contains("短语无效"));
    let confirmed = open_commerce_action_confirmation_service::confirm(
        &fixture.store,
        &actor,
        &prepared.id,
        ACTION_CONFIRMATION_PHRASE,
    )
    .unwrap();
    assert_eq!(confirmed.status, "confirmed");

    let mut changed = request.clone();
    changed.input = json!({"private_note":"changed"});
    let mismatch = open_commerce_service::invoke_with_action_confirmation(
        &fixture.store,
        &actor,
        changed,
        Some(&prepared.id),
    )
    .await
    .unwrap_err();
    assert!(mismatch.to_string().contains("输入不一致"));

    let first = open_commerce_service::invoke_with_action_confirmation(
        &fixture.store,
        &actor,
        request.clone(),
        Some(&prepared.id),
    )
    .await
    .unwrap();
    assert_eq!(first["result"]["order_id"], "order-1");
    assert_eq!(first["replayed"], false);
    let consumed = fixture
        .store
        .open_commerce_action_confirmation(&prepared.id)
        .unwrap();
    assert_eq!(consumed.status, "consumed");
    assert_eq!(
        consumed.invocation_id.as_deref(),
        first["invocation_id"].as_str()
    );

    let replay = open_commerce_service::invoke_with_action_confirmation(
        &fixture.store,
        &actor,
        request.clone(),
        Some(&prepared.id),
    )
    .await
    .unwrap();
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["invocation_id"], first["invocation_id"]);
    let recovered_confirmation =
        open_commerce_action_confirmation_service::prepare(&fixture.store, &actor, request.clone())
            .unwrap();
    assert_eq!(recovered_confirmation.id, prepared.id);
    assert_eq!(recovered_confirmation.status, "consumed");
    assert_eq!(
        recovered_confirmation.invocation_id.as_deref(),
        first["invocation_id"].as_str()
    );

    let second_key = action_request(&fixture, "action-confirmation-call-2");
    let reused = open_commerce_service::invoke_with_action_confirmation(
        &fixture.store,
        &actor,
        second_key,
        Some(&prepared.id),
    )
    .await
    .unwrap_err();
    assert!(reused.to_string().contains("幂等键或输入不一致"));

    let query = open_commerce_service::invoke(
        &fixture.store,
        &actor,
        InvokeCapabilityRequest {
            merchant_id: fixture.merchant_id,
            capability_key: "menu.lookup".to_string(),
            requester_app_id: "pc-web".to_string(),
            grant_id: None,
            idempotency_key: "query-without-confirmation".to_string(),
            input: json!({}),
        },
    )
    .await
    .unwrap();
    assert_eq!(query["result"]["items"][0], "latte");
}

#[test]
fn active_action_confirmations_are_bounded_per_user_and_app() {
    let fixture = fixture();
    let actor = OpenCommerceActor {
        user_id: &fixture.owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    for index in 0..MAX_ACTIVE_ACTION_CONFIRMATIONS_PER_APP {
        open_commerce_action_confirmation_service::prepare(
            &fixture.store,
            &actor,
            action_request(&fixture, &format!("bounded-confirmation-{index:02}")),
        )
        .unwrap();
    }
    let overflow = open_commerce_action_confirmation_service::prepare(
        &fixture.store,
        &actor,
        action_request(&fixture, "bounded-confirmation-overflow"),
    )
    .unwrap_err();
    assert!(overflow.to_string().contains("活动动作确认过多"));
}

#[test]
fn confirmation_rejects_other_actor_and_expired_challenge() {
    let fixture = fixture();
    let actor = OpenCommerceActor {
        user_id: &fixture.owner_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let prepared = open_commerce_action_confirmation_service::prepare(
        &fixture.store,
        &actor,
        action_request(&fixture, "action-confirmation-expired"),
    )
    .unwrap();
    let other = fixture
        .store
        .create_user("action-other@example.com", "secret1", None, None)
        .unwrap();
    let other_actor = OpenCommerceActor {
        user_id: &other.id,
        app_id: "pc-web",
        project_role: None,
    };
    let ownership_error = open_commerce_action_confirmation_service::confirm(
        &fixture.store,
        &other_actor,
        &prepared.id,
        ACTION_CONFIRMATION_PHRASE,
    )
    .unwrap_err();
    assert!(ownership_error.to_string().contains("不属于"));

    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_action_confirmations
                SET expires_at = '2000-01-01T00:00:00Z' WHERE id = ?1",
            [&prepared.id],
        )
        .unwrap();
    let expired = open_commerce_action_confirmation_service::confirm(
        &fixture.store,
        &actor,
        &prepared.id,
        ACTION_CONFIRMATION_PHRASE,
    )
    .unwrap_err();
    assert!(expired.to_string().contains("已过期"));
    assert_eq!(
        fixture
            .store
            .open_commerce_action_confirmation(&prepared.id)
            .unwrap()
            .status,
        "expired"
    );
}

#[tokio::test]
async fn mcp_action_flow_uses_the_same_confirmation_and_invocation_service() {
    let fixture = fixture();
    let prepared = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_prepare_action_confirmation",
            "arguments":{
                "merchant_id":fixture.merchant_id,
                "capability_key":"order.commit",
                "idempotency_key":"mcp-confirmed-action",
                "input":{"private_note":"mcp-secret"}
            }
        }),
    )
    .await
    .unwrap();
    let confirmation_id = prepared["structuredContent"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_confirm_action_confirmation",
            "arguments":{
                "confirmation_id":confirmation_id,
                "confirmation_phrase":"CONFIRM_ACTION"
            }
        }),
    )
    .await
    .unwrap();
    let invoked = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_id,
        "owner",
        "pc-web",
        json!({
            "name":"open_commerce_invoke",
            "arguments":{
                "merchant_id":fixture.merchant_id,
                "capability_key":"order.commit",
                "idempotency_key":"mcp-confirmed-action",
                "action_confirmation_id":confirmation_id,
                "input":{"private_note":"mcp-secret"}
            }
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        invoked["structuredContent"]["result"]["order_id"],
        "order-1"
    );
}
