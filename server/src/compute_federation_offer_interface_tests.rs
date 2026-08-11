use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    compute_federation_offer_lifecycle_model::{
        DrainComputeOfferRequest, TerminateComputeOfferRequest,
    },
    compute_federation_offer_lifecycle_service,
    compute_federation_offer_publication_model::PublishComputeOfferDraftRequest,
    compute_federation_offer_publication_service,
    compute_federation_offer_service::{self, test_support::Fixture},
    open_commerce_developer_production_test_support::test_app_state,
    store::Store,
    types::AppState,
};

const ADMIN_LIST: &str = "compute_admin_list_offer_drafts";
const ADMIN_PUBLISH: &str = "compute_admin_publish_offer";
const ADMIN_REVOKE: &str = "compute_admin_revoke_offer";

#[tokio::test]
async fn offer_http_and_mcp_share_governed_owner_admin_lifecycle() {
    let fixture = InterfaceFixture::new();
    let user_tools = super::definitions_for_platform_role("user");
    assert!(has_tool(&user_tools, "compute_create_my_offer_draft"));
    assert!(!has_tool(&user_tools, ADMIN_PUBLISH));
    assert!(has_tool(
        &super::definitions_for_platform_role("admin"),
        ADMIN_PUBLISH
    ));

    let denied = super::call_admin_if_handled(
        &fixture.state.store,
        &fixture.outsider_id,
        "user",
        ADMIN_LIST,
        json!({}),
    )
    .unwrap_err();
    assert!(denied.to_string().contains("只有平台管理员"), "{denied:#}");

    assert_eq!(
        call_http(
            &fixture.router,
            Method::GET,
            "/api/admin/compute/offers",
            None,
            Value::Null,
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call_http(
            &fixture.router,
            Method::GET,
            "/api/admin/compute/offers",
            Some(&fixture.outsider_token),
            Value::Null,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, drafts) = call_http(
        &fixture.router,
        Method::GET,
        "/api/admin/compute/offers?limit=20",
        Some(&fixture.admin_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{drafts}");
    assert_eq!(drafts["offers"].as_array().unwrap().len(), 1);

    let published = admin_call(
        &fixture.state.store,
        &fixture.admin_id,
        ADMIN_PUBLISH,
        json!({
            "offer_id":fixture.offer_id,
            "request":{
                "expected_offer_version":fixture.draft_version,
                "expected_offer_digest":fixture.draft_digest,
                "idempotency_key":"offer-interface-publish",
                "confirm_publish":true
            }
        }),
    );
    assert_eq!(published["offer_effect"], "active");
    assert_eq!(published["price_snapshot_effect"], "none");
    let replayed = admin_call(
        &fixture.state.store,
        &fixture.admin_id,
        ADMIN_PUBLISH,
        json!({
            "offer_id":fixture.offer_id,
            "request":{
                "expected_offer_version":fixture.draft_version,
                "expected_offer_digest":fixture.draft_digest,
                "idempotency_key":"offer-interface-publish",
                "confirm_publish":true
            }
        }),
    );
    assert_eq!(replayed["replayed"], true);

    let (status, publication) = call_http(
        &fixture.router,
        Method::GET,
        &fixture.owner_receipt_path("publication"),
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{publication}");
    assert_eq!(publication["publication_id"], published["publication_id"]);

    let active = compute_federation_offer_service::get_for_user(
        &fixture.state.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &fixture.offer_id,
    )
    .unwrap();
    let drain_body = json!({
        "expected_offer_version":active.offer.offer_version,
        "expected_offer_digest":active.offer.offer_digest,
        "reason":"interface drain",
        "idempotency_key":"offer-interface-drain",
        "confirm_drain":true
    });
    let (status, drained) = call_http(
        &fixture.router,
        Method::POST,
        &format!("/api/admin/compute/offers/{}/drain", fixture.offer_id),
        Some(&fixture.admin_token),
        drain_body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{drained}");
    assert_eq!(drained["target_status"], "draining");

    let revoked = admin_call(
        &fixture.state.store,
        &fixture.admin_id,
        ADMIN_REVOKE,
        json!({
            "offer_id":fixture.offer_id,
            "request":{
                "expected_offer_version":drained["target_offer_version"],
                "expected_offer_digest":drained["target_offer_digest"],
                "reason":"interface revoke",
                "idempotency_key":"offer-interface-revoke",
                "confirm_terminal":true
            }
        }),
    );
    assert_eq!(revoked["target_status"], "revoked");
    assert_eq!(revoked["reservation_effect"], "preserved");
    let (status, owner_revocation) = call_http(
        &fixture.router,
        Method::GET,
        &fixture.owner_receipt_path("revoke"),
        Some(&fixture.owner_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{owner_revocation}");
    assert_eq!(owner_revocation["event_id"], revoked["event_id"]);

    fixture.cleanup();
}

#[test]
fn offer_lifecycle_and_receipts_survive_file_store_reopen() {
    let fixture = Fixture::new();
    fixture.seed_active_supply();
    let draft = compute_federation_offer_service::create_draft_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_request("offer-reopen", 100, 4),
    )
    .unwrap();
    let publication = compute_federation_offer_publication_service::publish_for_review(
        &fixture.store,
        &fixture.admin_id,
        &draft.offer.offer_id,
        PublishComputeOfferDraftRequest {
            expected_offer_version: draft.offer.offer_version,
            expected_offer_digest: draft.offer.offer_digest,
            idempotency_key: "offer-reopen-publish".into(),
            confirm_publish: true,
        },
    )
    .unwrap();
    let active =
        compute_federation_offer_service::get_for_review(&fixture.store, &draft.offer.offer_id)
            .unwrap();
    let drain = compute_federation_offer_lifecycle_service::drain_for_review(
        &fixture.store,
        &fixture.admin_id,
        &draft.offer.offer_id,
        DrainComputeOfferRequest {
            expected_offer_version: active.offer.offer_version,
            expected_offer_digest: active.offer.offer_digest,
            reason: "reopen drain".into(),
            idempotency_key: "offer-reopen-drain".into(),
            confirm_drain: true,
        },
    )
    .unwrap();
    let terminal = compute_federation_offer_lifecycle_service::revoke_for_review(
        &fixture.store,
        &fixture.admin_id,
        &draft.offer.offer_id,
        TerminateComputeOfferRequest {
            expected_offer_version: drain.target_offer_version,
            expected_offer_digest: drain.target_offer_digest,
            reason: "reopen revoke".into(),
            idempotency_key: "offer-reopen-revoke".into(),
            confirm_terminal: true,
        },
    )
    .unwrap();
    let root = fixture.root.clone();
    let owner_id = fixture.owner_id.clone();
    let provider_id = fixture.provider_id.clone();
    let pool_id = fixture.pool_id.clone();
    let offer_id = draft.offer.offer_id;
    drop(fixture);

    let reopened = Store::open(&root.join("state.sqlite")).unwrap();
    assert_eq!(
        compute_federation_offer_service::get_for_user(
            &reopened,
            &owner_id,
            &provider_id,
            &pool_id,
            &offer_id,
        )
        .unwrap()
        .offer
        .status,
        "revoked"
    );
    assert_eq!(
        compute_federation_offer_publication_service::get_for_review(&reopened, &offer_id)
            .unwrap()
            .publication_id,
        publication.publication_id
    );
    assert_eq!(
        compute_federation_offer_lifecycle_service::get_terminal_for_review(
            &reopened, &offer_id, "revoked",
        )
        .unwrap()
        .event_id,
        terminal.event_id
    );
    drop(reopened);
    let _ = std::fs::remove_dir_all(root);
}

struct InterfaceFixture {
    state: Arc<AppState>,
    router: Router,
    root: std::path::PathBuf,
    owner_id: String,
    admin_id: String,
    outsider_id: String,
    owner_token: String,
    admin_token: String,
    outsider_token: String,
    provider_id: String,
    pool_id: String,
    offer_id: String,
    draft_version: i64,
    draft_digest: String,
}

impl InterfaceFixture {
    fn new() -> Self {
        let mut source = Fixture::new();
        let owner = create_user(&source.store, "offer-interface-owner", None);
        let admin = create_user(&source.store, "offer-interface-admin", Some("admin"));
        let outsider = create_user(&source.store, "offer-interface-outsider", None);
        source.owner_id = owner.id.clone();
        source.admin_id = admin.id.clone();
        source.seed_active_supply();
        let owner_token = session(&source.store, &owner.id);
        let admin_token = session(&source.store, &admin.id);
        let outsider_token = session(&source.store, &outsider.id);
        let create_request = source.create_request("offer-interface", 100, 4);
        let created = super::call_if_handled(
            &source.store,
            "project-unused",
            &owner.id,
            "compute_create_my_offer_draft",
            json!({
                "provider_id":source.provider_id,
                "pool_id":source.pool_id,
                "request":create_request
            }),
        )
        .unwrap()
        .unwrap();
        let root = source.root.clone();
        let provider_id = source.provider_id.clone();
        let pool_id = source.pool_id.clone();
        let offer_id = created["offer"]["offer_id"].as_str().unwrap().to_string();
        let draft_version = created["offer"]["offer_version"].as_i64().unwrap();
        let draft_digest = created["offer"]["offer_digest"]
            .as_str()
            .unwrap()
            .to_string();
        let state = Arc::new(test_app_state(source.store, &root));
        let router = crate::compute_federation_offer_api::routes().with_state(state.clone());
        Self {
            state,
            router,
            root,
            owner_id: owner.id,
            admin_id: admin.id,
            outsider_id: outsider.id,
            owner_token,
            admin_token,
            outsider_token,
            provider_id,
            pool_id,
            offer_id,
            draft_version,
            draft_digest,
        }
    }

    fn owner_receipt_path(&self, receipt: &str) -> String {
        format!(
            "/api/me/compute/providers/{}/capacity-pools/{}/offers/{}/{}",
            self.provider_id, self.pool_id, self.offer_id, receipt
        )
    }

    fn cleanup(self) {
        let root = self.root.clone();
        drop(self);
        let _ = std::fs::remove_dir_all(root);
    }
}

fn admin_call(store: &Store, user_id: &str, name: &str, arguments: Value) -> Value {
    super::call_admin_if_handled(store, user_id, "admin", name, arguments)
        .unwrap()
        .unwrap_or_else(|| panic!("admin MCP tool not handled: {name}"))
}

async fn call_http(
    router: &Router,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    let request_body = if body.is_null() {
        Body::empty()
    } else {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    };
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(builder.body(request_body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}

fn create_user(store: &Store, prefix: &str, role: Option<&str>) -> crate::store::PublicUser {
    store
        .create_user(
            &format!("{prefix}-{}@example.com", Uuid::new_v4().simple()),
            "secret1",
            None,
            role,
        )
        .unwrap()
}

fn session(store: &Store, user_id: &str) -> String {
    store
        .create_session(user_id, Some("compute-offer-interface"), None)
        .unwrap()
        .0
}

fn has_tool(tools: &[Value], name: &str) -> bool {
    tools.iter().any(|tool| tool["name"] == name)
}
