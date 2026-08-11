use std::{path::PathBuf, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    open_commerce_developer_production_test_support::test_app_state,
    store::{PublicUser, Store},
};

use super::routes;

const PATH: &str = "/api/admin/compute/delivery-allocation-reservations/expire-due";

#[tokio::test]
async fn reservation_expiry_http_is_admin_only_and_has_a_closed_request_contract() {
    let fixture = Fixture::new();
    let confirmed = json!({"limit":20,"confirm_expire_due":true});

    assert_eq!(
        call(&fixture.router, None, &confirmed).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&fixture.router, Some(&fixture.member_token), &confirmed)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &fixture.router,
            Some(&fixture.admin_token),
            &json!({
                "limit":20,
                "confirm_expire_due":true,
                "cutoff":"2099-01-01T00:00:00Z"
            }),
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let (status, denied) = call(
        &fixture.router,
        Some(&fixture.admin_token),
        &json!({"limit":20,"confirm_expire_due":false}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{denied}");
    assert!(denied["error"].as_str().unwrap().contains("显式确认"));

    for invalid_limit in [0, 101] {
        let (status, denied) = call(
            &fixture.router,
            Some(&fixture.admin_token),
            &json!({"limit":invalid_limit,"confirm_expire_due":true}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{denied}");
        assert!(denied["error"].as_str().unwrap().contains("limit"));
    }

    for token in [&fixture.admin_token, &fixture.owner_token] {
        let (status, report) = call(&fixture.router, Some(token), &confirmed).await;
        assert_eq!(status, StatusCode::OK, "{report}");
        assert!(report["recovery_started_at"].is_string(), "{report}");
        assert_eq!(report["selected_count"], 0, "{report}");
        assert_eq!(report["expired_count"], 0, "{report}");
        assert_eq!(report["replayed_count"], 0, "{report}");
        assert_eq!(report["blocked_count"], 0, "{report}");
        assert_eq!(report["failed_count"], 0, "{report}");
        assert_eq!(report["items"], json!([]), "{report}");
        assert_eq!(report["money_effect"], "preauthorization_refund_only");
        assert_eq!(report["provider_balance_effect"], "none");
        assert_eq!(report["settlement_effect"], "none");
    }

    fixture.cleanup();
}

struct Fixture {
    router: Router,
    root: PathBuf,
    admin_token: String,
    owner_token: String,
    member_token: String,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "elon_delivery_allocation_reservation_expiry_http_{}",
            Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let store = Store::open(&root.join("state.db")).unwrap();
        let admin = create_user(&store, "admin", Some("admin"));
        let owner = create_user(&store, "owner", None);
        store
            .conn()
            .unwrap()
            .execute("UPDATE users SET role='owner' WHERE id=?1", [&owner.id])
            .unwrap();
        let member = create_user(&store, "member", None);
        let admin_token = session(&store, &admin.id);
        let owner_token = session(&store, &owner.id);
        let member_token = session(&store, &member.id);
        let state = Arc::new(test_app_state(store, &root));
        Self {
            router: routes().with_state(state),
            root,
            admin_token,
            owner_token,
            member_token,
        }
    }

    fn cleanup(self) {
        let root = self.root.clone();
        drop(self);
        let _ = std::fs::remove_dir_all(root);
    }
}

fn create_user(store: &Store, suffix: &str, role: Option<&str>) -> PublicUser {
    store
        .create_user(
            &format!(
                "delivery-expiry-{suffix}-{}@example.com",
                Uuid::new_v4().simple()
            ),
            "secret1",
            None,
            role,
        )
        .unwrap()
}

fn session(store: &Store, user_id: &str) -> String {
    store
        .create_session(user_id, Some("delivery-expiry-http-test"), None)
        .unwrap()
        .0
}

async fn call(router: &Router, bearer: Option<&str>, body: &Value) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(PATH)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(bytes.as_ref()).into_owned()));
    (status, value)
}
