use axum::{
    body::{to_bytes, Body},
    http::{header, HeaderMap, Request, StatusCode},
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signer as _, SigningKey};
use serde_json::{json, Value};
use tower::ServiceExt;

use super::super::http_tests::Fixture;
use crate::esk_asset::platform::sui_address_binding::*;

const BASE: &str = "/api/me/assets/esk/platform/sui-address-binding";

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[1_u8; 32])
}

fn address() -> String {
    derive_sui_address(
        SignatureScheme::Ed25519,
        &signing_key().verifying_key().to_bytes(),
    )
}

fn synthetic_address(index: u32) -> String {
    format!("0x{index:064x}")
}

async fn send(
    fixture: &Fixture,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = match body {
        Some(value) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    let response = fixture
        .router
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert_eq!(headers[header::PRAGMA], "no-cache");
    assert_eq!(headers["referrer-policy"], "no-referrer");
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    let value = serde_json::from_slice(&bytes).unwrap();
    (status, headers, value)
}

fn assert_error(status: StatusCode, body: &Value, expected: StatusCode, code: &str) {
    assert_eq!(status, expected);
    assert_eq!(body, &json!({"error": code}));
}

fn assert_invalid(status: StatusCode, body: &Value) {
    assert_error(
        status,
        body,
        StatusCode::BAD_REQUEST,
        "ESK_PLATFORM_SUI_BINDING_INVALID_INPUT",
    );
}

fn assert_unauthorized(status: StatusCode, body: &Value) {
    assert_error(
        status,
        body,
        StatusCode::UNAUTHORIZED,
        "ESK_PLATFORM_SUI_BINDING_NOT_AUTHORIZED",
    );
}

async fn create(
    fixture: &Fixture,
    token: &str,
    address: String,
    ttl_seconds: u32,
) -> (StatusCode, Value) {
    let (status, _, body) = send(
        fixture,
        "POST",
        &format!("{BASE}/challenges"),
        Some(token),
        Some(json!({
            "schema": PLATFORM_REQUEST_SCHEMA,
            "address": address,
            "ttl_seconds": ttl_seconds,
        })),
    )
    .await;
    (status, body)
}

async fn complete(
    fixture: &Fixture,
    token: &str,
    challenge_id: &str,
    body: Value,
) -> (StatusCode, Value) {
    let (status, _, body) = send(
        fixture,
        "POST",
        &format!("{BASE}/challenges/{challenge_id}/complete"),
        Some(token),
        Some(body),
    )
    .await;
    (status, body)
}

fn response_for(challenge: &AddressBindingChallenge) -> Value {
    let message = BASE64.decode(&challenge.message_base64).unwrap();
    let digest = personal_message_digest(&message);
    let key = signing_key();
    let mut signature = vec![SignatureScheme::Ed25519.flag()];
    signature.extend_from_slice(&key.sign(&digest).to_bytes());
    signature.extend_from_slice(&key.verifying_key().to_bytes());
    json!({
        "schema": WALLET_RESPONSE_SCHEMA,
        "challenge_id": challenge.challenge_id,
        "message_base64": challenge.message_base64,
        "signature": BASE64.encode(signature),
    })
}

#[tokio::test]
async fn authenticated_create_complete_and_read_expose_only_the_public_contract() {
    let fixture = Fixture::new();
    let (status, _, account_before) = send(
        &fixture,
        "GET",
        "/api/me/assets/esk/platform",
        Some(&fixture.user_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, unbound) = send(&fixture, "GET", BASE, Some(&fixture.user_token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        unbound,
        json!({
            "schema": "yilong.esk.sui.platform_address_binding.v2",
            "status": "unbound",
        })
    );

    let (status, challenge_value) = create(&fixture, &fixture.user_token, address(), 600).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(challenge_value.as_object().unwrap().len(), 12);
    let challenge: AddressBindingChallenge = serde_json::from_value(challenge_value).unwrap();

    let wallet_response = response_for(&challenge);
    let wallet_signature = wallet_response["signature"].as_str().unwrap().to_owned();
    let (status, bound) = complete(
        &fixture,
        &fixture.user_token,
        &challenge.challenge_id,
        wallet_response,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_bound_contract(&bound);
    assert_eq!(bound["address"], address());

    let (status, _, read) = send(&fixture, "GET", BASE, Some(&fixture.user_token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read, bound);
    let serialized = read.to_string();
    for private in [
        fixture.user_id.as_str(),
        fixture.user_token.as_str(),
        challenge.subject_commitment.as_str(),
        challenge.nonce_base64.as_str(),
        challenge.message_base64.as_str(),
        wallet_signature.as_str(),
    ] {
        assert!(!serialized.contains(private));
    }
    assert_eq!(
        fixture
            .state
            .store
            .esk_account_ledger(&fixture.user_id)
            .unwrap()
            .total_base_units,
        9_000_000
    );
    let (status, _, account_after) = send(
        &fixture,
        "GET",
        "/api/me/assets/esk/platform",
        Some(&fixture.user_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(account_after, account_before);
    assert_eq!(account_after["source"], "platform_recorded");
    assert_eq!(account_after["chain_status"], "not_deployed");
    assert_eq!(
        account_after["capabilities"],
        json!({
            "service_spending": false,
            "quant_subscription": false,
            "sellback_settlement": false,
            "onchain_transfer": false,
            "chain_migration": false,
        })
    );

    let (status, conflict) =
        create(&fixture, &fixture.user_token, synthetic_address(77), 600).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        conflict,
        json!({"error": "ESK_PLATFORM_SUI_BINDING_CONFLICT"})
    );
    fixture.cleanup();
}

#[tokio::test]
async fn authentication_precedes_input_handling_and_boundaries_fail_closed() {
    let fixture = Fixture::new();
    for token in [
        None,
        Some("synthetic-static-owner-not-a-session"),
        Some(fixture.state.admin_token.as_str()),
        Some("synthetic-invalid-session"),
    ] {
        let (status, _, body) = send(
            &fixture,
            "POST",
            &format!("{BASE}/challenges"),
            token,
            Some(json!({"malformed": true})),
        )
        .await;
        assert_unauthorized(status, &body);
    }

    let (status, _, body) = send(
        &fixture,
        "POST",
        &format!("{BASE}/challenges/not-a-challenge/complete?attacker=true"),
        None,
        Some(json!({"malformed": true})),
    )
    .await;
    assert_unauthorized(status, &body);

    let (status, _, body) = send(
        &fixture,
        "GET",
        &format!("{BASE}?attacker=true"),
        None,
        Some(json!({"malformed": true})),
    )
    .await;
    assert_unauthorized(status, &body);

    let (status, _, body) = send(
        &fixture,
        "POST",
        &format!("{BASE}/challenges"),
        Some(&fixture.user_token),
        Some(json!({
            "schema": PLATFORM_REQUEST_SCHEMA,
            "address": address(),
            "ttl_seconds": 600,
            "user_id": "attacker-selected",
        })),
    )
    .await;
    assert_invalid(status, &body);

    let oversized = "x".repeat(17 * 1024);
    let (status, _, body) = send(
        &fixture,
        "POST",
        &format!("{BASE}/challenges"),
        Some(&fixture.user_token),
        Some(json!({"oversized": oversized})),
    )
    .await;
    assert_invalid(status, &body);

    fixture
        .state
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE users SET status='disabled' WHERE id=?1",
            [&fixture.user_id],
        )
        .unwrap();
    let (status, _, body) = send(&fixture, "GET", BASE, Some(&fixture.user_token), None).await;
    assert_unauthorized(status, &body);
    fixture.cleanup();
}

#[tokio::test]
async fn authentication_storage_failure_is_fixed_and_does_not_leak() {
    let fixture = Fixture::new();
    fixture
        .state
        .store
        .conn()
        .unwrap()
        .execute_batch("ALTER TABLE sessions RENAME TO synthetic_unavailable_sessions")
        .unwrap();

    let (status, _, body) = send(&fixture, "GET", BASE, Some(&fixture.user_token), None).await;
    assert_error(
        status,
        &body,
        StatusCode::INTERNAL_SERVER_ERROR,
        "ESK_PLATFORM_SUI_BINDING_STORAGE_ERROR",
    );
    assert!(!body.to_string().contains("synthetic_unavailable_sessions"));
    fixture.cleanup();
}

#[tokio::test]
async fn request_contract_ttl_and_challenge_ownership_fail_closed() {
    let fixture = Fixture::new();
    for (index, ttl) in [(1, 120), (2, 900)] {
        let (status, challenge) =
            create(&fixture, &fixture.user_token, synthetic_address(index), ttl).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(challenge["ttl_seconds"], ttl);
    }
    for (index, ttl) in [(3, 119), (4, 901)] {
        let (status, body) =
            create(&fixture, &fixture.user_token, synthetic_address(index), ttl).await;
        assert_invalid(status, &body);
    }
    let (status, challenge) = create(&fixture, &fixture.user_token, address(), 600).await;
    assert_eq!(status, StatusCode::OK);
    let challenge: AddressBindingChallenge = serde_json::from_value(challenge).unwrap();
    let valid_response = response_for(&challenge);

    let mut unknown_field = valid_response.clone();
    unknown_field
        .as_object_mut()
        .unwrap()
        .insert("user_id".into(), json!("attacker-selected"));
    let (status, body) = complete(
        &fixture,
        &fixture.user_token,
        &challenge.challenge_id,
        unknown_field,
    )
    .await;
    assert_invalid(status, &body);

    let mut mismatched = valid_response.clone();
    mismatched["challenge_id"] = json!(format!("eab1_{}", "a".repeat(32)));
    let (status, body) = complete(
        &fixture,
        &fixture.user_token,
        &challenge.challenge_id,
        mismatched,
    )
    .await;
    assert_invalid(status, &body);

    for (token, challenge_id, response) in [
        (
            fixture.other_token.as_str(),
            challenge.challenge_id.as_str(),
            valid_response,
        ),
        (
            fixture.user_token.as_str(),
            "eab1_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            json!({
                "schema": WALLET_RESPONSE_SCHEMA,
                "challenge_id": "eab1_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "message_base64": "YQ==",
                "signature": "AAA=",
            }),
        ),
    ] {
        let (status, body) = complete(&fixture, token, challenge_id, response).await;
        assert_error(
            status,
            &body,
            StatusCode::NOT_FOUND,
            "ESK_PLATFORM_SUI_BINDING_NOT_FOUND",
        );
    }

    for (path, body) in [
        (format!("{BASE}?attacker=true"), None),
        (BASE.to_owned(), Some(json!({"unexpected": true}))),
    ] {
        let (status, _, response) =
            send(&fixture, "GET", &path, Some(&fixture.user_token), body).await;
        assert_error(
            status,
            &response,
            StatusCode::BAD_REQUEST,
            "ESK_PLATFORM_SUI_BINDING_INVALID_INPUT",
        );
    }
    fixture.cleanup();
}

#[tokio::test]
async fn fourth_live_challenge_returns_fixed_rate_limit() {
    let fixture = Fixture::new();
    for index in 10..=13 {
        let (status, body) =
            create(&fixture, &fixture.user_token, synthetic_address(index), 600).await;
        if index < 13 {
            assert_eq!(status, StatusCode::OK);
        } else {
            assert_error(
                status,
                &body,
                StatusCode::TOO_MANY_REQUESTS,
                "ESK_PLATFORM_SUI_BINDING_RATE_LIMITED",
            );
        }
    }
    fixture.cleanup();
}

fn assert_bound_contract(value: &Value) {
    let object = value.as_object().unwrap();
    assert_eq!(object.len(), 14);
    for key in [
        "schema",
        "status",
        "network",
        "address",
        "signature_scheme",
        "bound_at",
        "binding_receipt_sha256",
        "address_control_verified",
        "platform_subject_authenticated",
        "challenge_single_use_recorded",
        "chain_finality_verified",
        "asset_identity_verified",
        "balance_eligible",
        "manifest_transition_allowed",
    ] {
        assert!(object.contains_key(key), "missing {key}");
    }
    assert_eq!(
        value["schema"],
        "yilong.esk.sui.platform_address_binding.v2"
    );
    assert_eq!(value["status"], "bound");
    assert_eq!(value["network"], "testnet");
    assert_eq!(value["signature_scheme"], "ed25519");
    parse_timestamp(value["bound_at"].as_str().unwrap()).unwrap();
    let receipt = value["binding_receipt_sha256"].as_str().unwrap();
    let receipt_hex = receipt.strip_prefix("sha256:").unwrap();
    assert_eq!(receipt_hex.len(), 64);
    assert!(receipt_hex
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
    assert_eq!(value["address_control_verified"], true);
    assert_eq!(value["platform_subject_authenticated"], true);
    assert_eq!(value["challenge_single_use_recorded"], true);
    assert_eq!(value["chain_finality_verified"], false);
    assert_eq!(value["asset_identity_verified"], false);
    assert_eq!(value["balance_eligible"], false);
    assert_eq!(value["manifest_transition_allowed"], false);
}
