use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use crate::open_commerce_developer_production_test_support::test_app_state;

use crate::compute_federation::capacity_commitment_service::test_support::Fixture;

use super::routes;

#[tokio::test]
async fn owner_http_is_authenticated_isolated_and_admin_expiry_is_role_gated() {
    let fixture = Fixture::new_http();
    let path = fixture.collection_path();
    let owner_token = fixture.owner_token.clone().unwrap();
    let admin_token = fixture.admin_token.clone().unwrap();
    let outsider_token = fixture.outsider_token.clone().unwrap();
    let outsider_id = fixture.outsider_id.clone().unwrap();
    let provider_id = fixture.provider_id.clone();
    let pool_id = fixture.pool_id.clone();
    let body = create_body(&fixture, "capacity-http-primary");
    let source_path = format!(
        "/api/me/compute/providers/{}/capacity-pools/{}/offers/{}/price-snapshots/{}/capacity-commitment-source",
        fixture.provider_id,
        fixture.pool_id,
        fixture.offer.offer_id,
        fixture.binding.snapshot_id,
    );
    let expected_binding_id = fixture.binding.binding_id.clone();
    let root = fixture.root.clone();
    let state = Arc::new(test_app_state(fixture.store, &root));
    let router = routes().with_state(state.clone());

    assert_eq!(
        call(&router, Method::GET, &source_path, None, &Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &router,
            Method::GET,
            &source_path,
            Some(&outsider_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let (status, source) = call(
        &router,
        Method::GET,
        &source_path,
        Some(&owner_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{source}");
    assert_eq!(
        source["reference_binding"]["binding_id"],
        expected_binding_id
    );
    assert_eq!(
        source["snapshot"]["snapshot_id"],
        fixture.binding.snapshot_id
    );

    assert_eq!(
        call(&router, Method::POST, &path, None, &body).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, outsider) = call(&router, Method::POST, &path, Some(&outsider_token), &body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{outsider}");

    let (status, created) = call(&router, Method::POST, &path, Some(&owner_token), &body).await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(created["commitment"]["commitment_status"], "committed");

    let (status, owner_list) = call(
        &router,
        Method::GET,
        &format!("{path}?status=committed&limit=20"),
        Some(&owner_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{owner_list}");
    assert_eq!(
        owner_list["capacity_commitments"].as_array().unwrap().len(),
        1
    );
    let (_, outsider_list) = call(
        &router,
        Method::GET,
        &path,
        Some(&outsider_token),
        &Value::Null,
    )
    .await;
    assert!(outsider_list["capacity_commitments"]
        .as_array()
        .unwrap()
        .is_empty());

    let admin_path = "/api/admin/compute/capacity-commitments/expire-due";
    let expire = json!({"limit":20,"confirm_expire_due":true});
    assert_eq!(
        call(
            &router,
            Method::POST,
            admin_path,
            Some(&outsider_token),
            &expire,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, report) = call(
        &router,
        Method::POST,
        admin_path,
        Some(&admin_token),
        &expire,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["selected_count"], 0);

    assert_ne!(
        outsider_id,
        state
            .store
            .compute_provider(&provider_id)
            .unwrap()
            .provider
            .owner_account_id
    );
    assert!(!pool_id.is_empty());
    drop(router);
    drop(state);
    let _ = std::fs::remove_dir_all(root);
}

fn create_body(fixture: &Fixture, idempotency_key: &str) -> Value {
    let body = fixture.create_body(idempotency_key, true);
    json!({
        "idempotency_key":body.idempotency_key,
        "provider_policy_revision":body.provider_policy_revision,
        "provider_digest":body.provider_digest,
        "offer_id":body.offer_id,
        "offer_version":body.offer_version,
        "offer_digest":body.offer_digest,
        "capacity_epoch":body.capacity_epoch,
        "pool_revision":body.pool_revision,
        "pool_digest":body.pool_digest,
        "delivery_window_id":body.delivery_window_id,
        "delivery_window_digest":body.delivery_window_digest,
        "price_snapshot_id":body.price_snapshot_id,
        "price_snapshot_digest":body.price_snapshot_digest,
        "reference_binding_id":body.reference_binding_id,
        "reference_binding_digest":body.reference_binding_digest,
        "instrument_id":body.instrument_id,
        "quantities":body.quantities,
        "confirm_commitment":body.confirm_commitment
    })
}

async fn call(
    router: &Router,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    body: &Value,
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
