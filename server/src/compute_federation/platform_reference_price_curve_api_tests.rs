use std::{path::PathBuf, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    compute_federation_offer_publication_model::PublishComputeOfferDraftRequest,
    compute_federation_offer_publication_service, compute_federation_offer_service,
    compute_federation_offer_service::test_support::Fixture as OfferFixture,
    open_commerce_developer_production_test_support::test_app_state,
    store::{PublicUser, Store},
};

use super::routes;

struct Fixture {
    state: Arc<crate::types::AppState>,
    router: Router,
    submitter_token: String,
    reviewer_token: String,
    applier_token: String,
    member_token: String,
    offer: crate::compute_federation::offer::ComputeOffer,
    window_id: String,
    root: PathBuf,
}

#[tokio::test]
async fn administrator_api_governs_reference_batch_and_registers_v171_snapshot() {
    let fixture = fixture();
    let (body, valid_from) = submit_body(&fixture, "api-reference-submit", 1);

    assert_eq!(
        call(&fixture.router, Method::POST, curve_path(), None, &body)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            curve_path(),
            Some(&fixture.member_token),
            &body,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut injected = body.clone();
    injected["submitted_by_admin_user_id"] = json!("forged-admin");
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            curve_path(),
            Some(&fixture.submitter_token),
            &injected,
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let (status, submitted) = call(
        &fixture.router,
        Method::POST,
        curve_path(),
        Some(&fixture.submitter_token),
        &body,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{submitted}");
    assert_eq!(submitted["status"], "submitted");
    assert_eq!(submitted["entries"].as_array().unwrap().len(), 1);
    assert!(submitted["entries"][0].get("components").is_none());
    assert!(submitted["entries"][0]
        .get("consumer_max_amount_micros")
        .is_none());
    let batch_id = submitted["batch_id"].as_str().unwrap();
    let detail_path = format!("{}/{batch_id}", curve_path());
    let preflight_path = format!("{detail_path}/preflight");
    let empty = json!({});

    let (_, listed) = call(
        &fixture.router,
        Method::GET,
        &format!("{}?status=submitted&limit=1", curve_path()),
        Some(&fixture.reviewer_token),
        &empty,
    )
    .await;
    assert_eq!(
        listed["reference_curve_batches"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        listed["reference_curve_batches"][0]["batch"]["batch_id"],
        submitted["batch_id"]
    );
    let (_, submitter_preflight) = call(
        &fixture.router,
        Method::GET,
        &preflight_path,
        Some(&fixture.submitter_token),
        &empty,
    )
    .await;
    assert_eq!(submitter_preflight["admin_review_allowed"], false);
    assert_eq!(
        submitter_preflight["blockers"][0],
        "current_admin_cannot_review_own_submission"
    );
    let (_, reviewer_preflight) = call(
        &fixture.router,
        Method::GET,
        &preflight_path,
        Some(&fixture.reviewer_token),
        &empty,
    )
    .await;
    assert_eq!(reviewer_preflight["admin_review_allowed"], true);

    let review = json!({
        "idempotency_key":"api-reference-review",
        "expected_batch_digest":submitted["batch_digest"],
        "expected_batch_material_digest":submitted["batch_material_digest"],
        "decision":"approved",
        "review_note":Value::Null,
        "confirm_review":true
    });
    let (status, reviewed) = call(
        &fixture.router,
        Method::POST,
        &format!("{detail_path}/review"),
        Some(&fixture.reviewer_token),
        &review,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reviewed}");
    let (_, approved_preflight) = call(
        &fixture.router,
        Method::GET,
        &preflight_path,
        Some(&fixture.applier_token),
        &empty,
    )
    .await;
    assert_eq!(approved_preflight["admin_apply_allowed"], true);

    wait_until(&valid_from).await;
    let application = json!({
        "idempotency_key":"api-reference-apply",
        "expected_batch_digest":submitted["batch_digest"],
        "expected_batch_material_digest":submitted["batch_material_digest"],
        "expected_review_id":reviewed["review_id"],
        "expected_review_digest":reviewed["review_digest"],
        "apply_note":"register fallback snapshot only",
        "confirm_application":true
    });
    let (status, applied) = call(
        &fixture.router,
        Method::POST,
        &format!("{detail_path}/application"),
        Some(&fixture.applier_token),
        &application,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["status"], "applied");
    assert_eq!(applied["market_effect"], "quote_candidate_enabled");
    assert_eq!(applied["funds_effect"], "none");
    let snapshot_id = applied["bindings"][0]["snapshot_id"].as_str().unwrap();
    let snapshot = fixture
        .state
        .store
        .compute_price_snapshot(snapshot_id)
        .unwrap();
    assert_eq!(snapshot.snapshot.price_source.source_kind, "fallback_curve");
    assert_eq!(snapshot.snapshot.price_source.sample_count, 0);

    let (_, detail) = call(
        &fixture.router,
        Method::GET,
        &detail_path,
        Some(&fixture.applier_token),
        &empty,
    )
    .await;
    assert_eq!(detail["batch"]["status"], "applied");
    assert_eq!(detail["review"]["review_id"], reviewed["review_id"]);
    assert_eq!(
        detail["application"]["application_id"],
        applied["application_id"]
    );
    let (_, final_preflight) = call(
        &fixture.router,
        Method::GET,
        &preflight_path,
        Some(&fixture.applier_token),
        &empty,
    )
    .await;
    assert_eq!(final_preflight["application_present"], true);
    assert_eq!(final_preflight["admin_apply_allowed"], false);
    assert_eq!(
        final_preflight["blockers"][0],
        "reference_curve_batch_already_applied"
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &format!("{}?status=canceled", curve_path()),
            Some(&fixture.applier_token),
            &empty,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    fixture.cleanup();
}

fn fixture() -> Fixture {
    let (offer_fixture, offer) = active_offer();
    let submitter = user(&offer_fixture.store, "curve-submitter", Some("admin"));
    let reviewer = user(&offer_fixture.store, "curve-reviewer", Some("admin"));
    let applier = user(&offer_fixture.store, "curve-applier", Some("admin"));
    let member = user(&offer_fixture.store, "curve-member", None);
    let submitter_token = session(&offer_fixture.store, &submitter.id);
    let reviewer_token = session(&offer_fixture.store, &reviewer.id);
    let applier_token = session(&offer_fixture.store, &applier.id);
    let member_token = session(&offer_fixture.store, &member.id);
    let root = offer_fixture.root;
    let window_id = offer_fixture.window_id;
    let state = Arc::new(test_app_state(offer_fixture.store, &root));
    let router = routes().with_state(state.clone());
    Fixture {
        state,
        router,
        submitter_token,
        reviewer_token,
        applier_token,
        member_token,
        offer: offer.offer,
        window_id,
        root,
    }
}

fn active_offer() -> (
    OfferFixture,
    crate::compute_federation_offer_service::MyComputeOfferView,
) {
    let mut fixture = OfferFixture::new();
    let now = Utc::now();
    fixture.starts_at = (now + Duration::milliseconds(300)).to_rfc3339();
    fixture.ends_at = (now + Duration::hours(2)).to_rfc3339();
    fixture.valid_until = (now + Duration::hours(3)).to_rfc3339();
    fixture.seed_active_supply();
    let draft = compute_federation_offer_service::create_draft_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        fixture.create_request("reference-curve-api-offer", 100, 4),
    )
    .unwrap();
    compute_federation_offer_publication_service::publish_for_review(
        &fixture.store,
        &fixture.admin_id,
        &draft.offer.offer_id,
        PublishComputeOfferDraftRequest {
            expected_offer_version: draft.offer.offer_version,
            expected_offer_digest: draft.offer.offer_digest,
            idempotency_key: "publish-reference-curve-api-offer".into(),
            confirm_publish: true,
        },
    )
    .unwrap();
    let active = compute_federation_offer_service::get_for_user(
        &fixture.store,
        &fixture.owner_id,
        &fixture.provider_id,
        &fixture.pool_id,
        &draft.offer.offer_id,
    )
    .unwrap();
    let target = DateTime::parse_from_rfc3339(&active.offer.valid_from)
        .unwrap()
        .with_timezone(&Utc);
    if let Ok(wait) = (target - Utc::now()).to_std() {
        std::thread::sleep(wait + std::time::Duration::from_millis(20));
    }
    (fixture, active)
}

fn submit_body(fixture: &Fixture, idempotency_key: &str, curve_version: i64) -> (Value, String) {
    let valid_from = canonical(Utc::now() + Duration::milliseconds(250));
    let valid_until = canonical(Utc::now() + Duration::minutes(30));
    let offer = &fixture.offer;
    let window = offer
        .delivery_windows
        .iter()
        .find(|window| window.binding.window_id == fixture.window_id)
        .unwrap();
    let components = offer
        .price_terms
        .components
        .iter()
        .map(|component| {
            json!({
                "meter":component.meter,
                "unit_size":component.unit_size,
                "consumer_unit_price_micros":component.consumer_unit_price_micros,
                "provider_unit_price_micros":component.provider_unit_price_micros,
                "max_units":component.max_units
            })
        })
        .collect::<Vec<_>>();
    let body = json!({
        "idempotency_key":idempotency_key,
        "curve_id":"platform-reference-cny",
        "curve_version":curve_version,
        "valid_from":valid_from,
        "valid_until":valid_until,
        "quote_ttl_seconds":300,
        "entries":[{
            "entry_key":format!("{}:{}", offer.offer_id, fixture.window_id),
            "provider_id":offer.provider_id,
            "offer_id":offer.offer_id,
            "offer_version":offer.offer_version,
            "offer_digest":offer.offer_digest,
            "sku_id":offer.sku.sku_id,
            "sku_digest":offer.sku.sku_digest,
            "delivery_window_id":window.binding.window_id,
            "delivery_window_digest":window.binding.window_digest,
            "pricing_mode":offer.price_terms.pricing_mode,
            "currency":offer.price_terms.currency,
            "offer_curve_id":offer.price_terms.curve_id,
            "offer_curve_version":offer.price_terms.curve_version,
            "instrument_id":offer.price_terms.instrument_id,
            "components":components,
            "fee_rules":[],
            "consumer_max_amount_micros":100000,
            "provider_max_amount_micros":80000
        }],
        "submission_note":"governed fallback only; no market samples",
        "confirm_submission":true
    });
    (body, valid_from)
}

fn user(store: &Store, prefix: &str, role: Option<&str>) -> PublicUser {
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
        .create_session(user_id, Some("platform-reference-curve-api"), None)
        .unwrap()
        .0
}

fn curve_path() -> &'static str {
    "/api/admin/compute/platform-reference-price-curves"
}

async fn call(
    router: &Router,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    body: &Value,
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
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn canonical(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

async fn wait_until(value: &str) {
    let target = DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc);
    if let Ok(wait) = (target - Utc::now()).to_std() {
        tokio::time::sleep(wait + std::time::Duration::from_millis(20)).await;
    }
}

impl Fixture {
    fn cleanup(self) {
        drop(self.router);
        drop(self.state);
        let _ = std::fs::remove_dir_all(self.root);
    }
}
