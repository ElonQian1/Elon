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
    format!(
        "/api/admin/compute/external-pool-adapter-release-admissions/{}/artifact-source",
        staged["admission_id"].as_str().unwrap()
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
    let mut builder = Request::builder()
        .method(Method::PUT)
        .uri(artifact_source_path(staged))
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header("idempotency-key", idempotency_key)
        .header(
            "x-elon-expected-admission-digest",
            staged["admission_digest"].as_str().unwrap(),
        )
        .header(
            "x-elon-artifact-source-confirmation",
            "confirm_external_pool_adapter_artifact_source_intake",
        );
    if let Some(token) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = router
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
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
