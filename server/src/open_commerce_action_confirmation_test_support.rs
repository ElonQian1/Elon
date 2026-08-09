use serde_json::json;
use uuid::Uuid;

use crate::{
    open_commerce_model::{
        CreateCapabilityRequest, CreateMerchantRequest, InvokeCapabilityRequest, ACCESS_PUBLIC,
        HANDLER_STATIC_JSON,
    },
    open_commerce_service::{self, OpenCommerceActor},
    store::Store,
};

pub(crate) struct Fixture {
    pub(crate) store: Store,
    pub(crate) owner_id: String,
    pub(crate) project_id: String,
    pub(crate) merchant_id: String,
}

pub(crate) fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "elon_action_confirmation_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("action-confirmation test store should open");
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

pub(crate) fn action_request(fixture: &Fixture, idempotency_key: &str) -> InvokeCapabilityRequest {
    InvokeCapabilityRequest {
        merchant_id: fixture.merchant_id.clone(),
        capability_key: "order.commit".to_string(),
        requester_app_id: "pc-web".to_string(),
        grant_id: None,
        idempotency_key: idempotency_key.to_string(),
        input: json!({"private_note":"do-not-store-this-value"}),
    }
}
