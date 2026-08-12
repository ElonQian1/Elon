use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::Value;
use tower::ServiceExt;

use crate::{
    compute_federation::capacity_commitment_service::test_support::Fixture,
    open_commerce_developer_production_test_support::test_app_state,
};

use super::routes;

#[tokio::test]
async fn admin_reads_exact_instrument_and_adoption_while_non_admin_is_forbidden() {
    let fixture = Fixture::new_http();
    let admin_token = fixture.admin_token.clone().unwrap();
    let outsider_token = fixture.outsider_token.clone().unwrap();
    let instrument_path = format!(
        "/api/admin/compute/capacity-instruments/{}",
        fixture.capacity_instrument.instrument_id
    );
    let adoption_path = format!(
        "/api/admin/compute/offers/{}/capacity-instrument-adoption",
        fixture.offer.offer_id
    );
    let expected_instrument_digest = fixture.capacity_instrument.instrument_digest.clone();
    let expected_offer_digest = fixture.offer.offer_digest.clone();
    let expected_publication_digest = fixture.publication.publication_digest.clone();
    let root = fixture.root.clone();
    let state = Arc::new(test_app_state(fixture.store, &root));
    let router = routes().with_state(state.clone());

    assert_eq!(
        call(&router, Method::GET, &instrument_path, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &router,
            Method::GET,
            &instrument_path,
            Some(&outsider_token),
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, instrument) =
        call(&router, Method::GET, &instrument_path, Some(&admin_token)).await;
    assert_eq!(status, StatusCode::OK, "{instrument}");
    assert_eq!(instrument["instrument_digest"], expected_instrument_digest);

    let (status, adoption) = call(&router, Method::GET, &adoption_path, Some(&admin_token)).await;
    assert_eq!(status, StatusCode::OK, "{adoption}");
    assert_eq!(adoption["offer_digest"], expected_offer_digest);
    assert_eq!(adoption["publication_digest"], expected_publication_digest);

    drop(router);
    drop(state);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn admin_read_errors_distinguish_not_found_invalid_and_conflict() {
    let fixture = Fixture::new_http();
    let admin_token = fixture.admin_token.clone().unwrap();
    let missing = "/api/admin/compute/capacity-instruments/missing-capacity-instrument";
    let invalid = "/api/admin/compute/capacity-instruments/%20";
    let activation_path = format!(
        "/api/admin/compute/capacity-instruments/{}/activate",
        fixture.capacity_instrument.instrument_id
    );
    let stale_activation = serde_json::json!({
        "expected_instrument_revision": fixture.capacity_instrument.instrument_revision,
        "expected_instrument_digest": "0".repeat(64),
        "idempotency_key": "stale-activation",
        "confirm_activation": true
    });
    let root = fixture.root.clone();
    let state = Arc::new(test_app_state(fixture.store, &root));
    let router = routes().with_state(state.clone());

    assert_eq!(
        call(&router, Method::GET, missing, Some(&admin_token))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&router, Method::GET, invalid, Some(&admin_token))
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        call_json(
            &router,
            Method::POST,
            &activation_path,
            &stale_activation,
            Some(&admin_token),
        )
        .await
        .0,
        StatusCode::CONFLICT
    );

    drop(router);
    drop(state);
    let _ = std::fs::remove_dir_all(root);
}

async fn call(
    router: &Router,
    method: Method,
    path: &str,
    bearer: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
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

async fn call_json(
    router: &Router,
    method: Method,
    path: &str,
    payload: &Value,
    bearer: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(builder.body(Body::from(payload.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&bytes).unwrap();
    (status, value)
}
