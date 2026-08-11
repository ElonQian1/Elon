use std::{
    convert::Infallible,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::Poll,
};

use futures::stream;
use sha2::{Digest, Sha256};

use axum::http::{HeaderMap, HeaderName, HeaderValue};

use super::*;

pub(super) const LOCAL_OWNER_TOKEN: &str = "external-pool-release-local-owner-token";

pub(super) async fn stage_release(
    fixture: &Fixture,
    suffix: &str,
    version: &str,
    artifact: &[u8],
) -> Value {
    let mut submission = submit_body(&format!("{suffix}-submit"), version, true);
    submission["candidate_artifact_ref"] =
        json!(format!("artifact-ref:sensitive-{suffix}-{version}"));
    submission["declared_implementation_sha256"] = json!(sha256(artifact));
    let (status, submitted) = call(
        &fixture.router,
        Method::POST,
        release_path(),
        Some(&fixture.submitter_token),
        &submission,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{submitted}");

    let request_id = submitted["request_id"].as_str().unwrap();
    let review = review_body(&submitted, &format!("{suffix}-review"), "approved", true);
    let (status, reviewed) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{request_id}/review", release_path()),
        Some(&fixture.reviewer_token),
        &review,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reviewed}");

    let stage = stage_body(&submitted, &reviewed, &format!("{suffix}-stage"), true);
    let (status, staged) = call(
        &fixture.router,
        Method::POST,
        &format!("{}/{request_id}/stage", release_path()),
        Some(&fixture.applier_token),
        &stage,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{staged}");
    assert_eq!(staged["status"], "staged");
    staged
}

pub(super) fn terminal_path(staged: &Value) -> String {
    format!(
        "/api/admin/compute/external-pool-adapter-release-admissions/{}/terminal",
        staged["admission_id"].as_str().unwrap()
    )
}

pub(super) fn currentness_path(staged: &Value) -> String {
    format!(
        "/api/admin/compute/external-pool-adapter-release-admissions/{}/currentness",
        staged["admission_id"].as_str().unwrap()
    )
}

pub(super) fn artifact_source_path(staged: &Value) -> String {
    artifact_source_path_for_admission(staged["admission_id"].as_str().unwrap())
}

pub(super) fn artifact_source_path_for_admission(admission_id: &str) -> String {
    format!(
        "/api/admin/compute/external-pool-adapter-release-admissions/{admission_id}/artifact-source"
    )
}

pub(super) fn terminal_body(
    staged: &Value,
    idempotency_key: &str,
    terminal_status: &str,
    successor: Option<&Value>,
    confirmed: bool,
) -> Value {
    json!({
        "idempotency_key": idempotency_key,
        "expected_admission_digest": staged["admission_digest"],
        "terminal_status": terminal_status,
        "successor_admission_id": successor.map(|value| value["admission_id"].clone()),
        "expected_successor_admission_digest": successor
            .map(|value| value["admission_digest"].clone()),
        "reason": format!("{terminal_status} by authenticated lifecycle HTTP test"),
        "confirm_terminal": confirmed
    })
}

pub(super) async fn raw_artifact_call(
    router: &Router,
    staged: &Value,
    bearer: Option<&str>,
    idempotency_key: &str,
    body: Body,
) -> (StatusCode, Value) {
    raw_artifact_call_with_headers(
        router,
        staged["admission_id"].as_str().unwrap(),
        bearer,
        artifact_source_headers(staged, idempotency_key),
        body,
    )
    .await
}

pub(super) fn artifact_source_headers(staged: &Value, idempotency_key: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.append(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.append(
        HeaderName::from_static("idempotency-key"),
        header_value(idempotency_key),
    );
    headers.append(
        HeaderName::from_static("x-elon-expected-admission-digest"),
        header_value(staged["admission_digest"].as_str().unwrap()),
    );
    headers.append(
        HeaderName::from_static("x-elon-artifact-source-confirmation"),
        HeaderValue::from_static("confirm_external_pool_adapter_artifact_source_intake"),
    );
    headers
}

pub(super) async fn raw_artifact_call_with_headers(
    router: &Router,
    admission_id: &str,
    bearer: Option<&str>,
    mut headers: HeaderMap,
    body: Body,
) -> (StatusCode, Value) {
    if let Some(token) = bearer {
        headers.append(
            header::AUTHORIZATION,
            header_value(&format!("Bearer {token}")),
        );
    }
    let mut request = Request::builder()
        .method(Method::PUT)
        .uri(artifact_source_path_for_admission(admission_id))
        .body(body)
        .unwrap();
    *request.headers_mut() = headers;
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn header_value(value: &str) -> HeaderValue {
    HeaderValue::from_bytes(value.as_bytes()).unwrap()
}

pub(super) fn tracked_body(bytes: &[u8]) -> (Body, Arc<AtomicBool>) {
    let polled = Arc::new(AtomicBool::new(false));
    let poll_marker = polled.clone();
    let mut chunk = Some(bytes.to_vec());
    let body = Body::from_stream(stream::poll_fn(move |_| {
        poll_marker.store(true, Ordering::SeqCst);
        Poll::Ready(chunk.take().map(Ok::<Vec<u8>, Infallible>))
    }));
    (body, polled)
}

pub(super) fn was_polled(marker: &AtomicBool) -> bool {
    marker.load(Ordering::SeqCst)
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub(super) fn artifact_blob_path(fixture: &Fixture, digest: &str) -> PathBuf {
    fixture
        .data_dir
        .join("compute-federation")
        .join("external-pool-adapter-artifacts")
        .join("v1")
        .join("quarantine")
        .join("blobs")
        .join("sha256")
        .join(&digest[..2])
        .join(format!("{digest}.blob"))
}

pub(super) fn assert_release_material_redacted(body: &Value, bearer: &str) {
    let encoded = body.to_string();
    for forbidden in [
        "candidate_artifact_ref",
        "expected_credential_verifier",
        "community-pool-verifier",
        "artifact-ref:sensitive-",
        bearer,
        "secret1",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "response exposed forbidden material {forbidden}: {encoded}"
        );
    }
}

pub(super) fn assert_artifact_http_response_redacted(
    fixture: &Fixture,
    body: &Value,
    raw_artifact: &[u8],
    bearer: &str,
) {
    assert_release_material_redacted(body, bearer);
    assert_forbidden_keys_absent(
        body,
        &[
            "artifact_size_bytes",
            "absolute_path",
            "blob_path",
            "candidate_artifact_ref",
            "content_address_digest",
            "download_url",
            "expected_credential_verifier",
            "file_path",
            "idempotency_key",
            "idempotency_scope",
            "intake_confirmation",
            "source_receipt_json",
        ],
    );
    let encoded = body.to_string();
    if !raw_artifact.is_empty() {
        if let Ok(raw) = std::str::from_utf8(raw_artifact) {
            assert!(
                !encoded.contains(raw),
                "response exposed raw artifact bytes: {encoded}"
            );
        }
    }
    for path in [&fixture.data_dir, &fixture.database_path] {
        let native = path.display().to_string();
        let json_escaped = native.replace('\\', "\\\\");
        assert!(
            !encoded.contains(&native) && !encoded.contains(&json_escaped),
            "response exposed a server-local path: {encoded}"
        );
    }
    assert!(!encoded.contains(".blob"), "response exposed a blob path");
}

fn assert_forbidden_keys_absent(value: &Value, forbidden: &[&str]) {
    match value {
        Value::Object(object) => {
            for key in forbidden {
                assert!(
                    !object.contains_key(*key),
                    "response exposed forbidden key {key}: {value}"
                );
            }
            for nested in object.values() {
                assert_forbidden_keys_absent(nested, forbidden);
            }
        }
        Value::Array(values) => {
            for nested in values {
                assert_forbidden_keys_absent(nested, forbidden);
            }
        }
        _ => {}
    }
}

pub(super) fn artifact_source_receipt_count(fixture: &Fixture) -> i64 {
    fixture
        .state
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM compute_external_pool_adapter_artifact_source_receipts",
            [],
            |row| row.get(0),
        )
        .unwrap()
}
