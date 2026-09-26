//! Production business queue/routes; only the enclosing runtime is reduced.
#![allow(dead_code)]
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    value.lock().unwrap()
}
fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
#[path = "../src/node_agent_win_codex_control/group_ai.rs"]
mod group_ai;
#[path = "../src/node_agent_win_codex_control/group_ai_api.rs"]
mod group_ai_api;
#[derive(Default)]
struct Hub {
    group_ai: group_ai::GroupAiControl,
}
#[derive(Default)]
struct NodeRuntime {
    win_codex_control: Hub,
}

#[tokio::test]
async fn production_routes_claim_once_then_publish_bounded_receipt() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let runtime = Arc::new(NodeRuntime::default());
    let id = uuid::Uuid::new_v4().to_string();
    runtime
        .win_codex_control
        .group_ai
        .enqueue(
            Path::new("fixture"),
            serde_json::from_value(json!({"command_id":id,"action":"groups"})).unwrap(),
        )
        .unwrap();
    let router = group_ai_api::routes().with_state(runtime.clone());
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/codex-control/group-ai/pending")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let pending: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 32768).await.unwrap()).unwrap();
    assert_eq!(pending["command_ids"][0], id);
    let worker = uuid::Uuid::new_v4().to_string();
    let request = || {
        Request::builder()
            .method("POST")
            .uri(format!("/api/codex-control/group-ai/{id}/claim"))
            .header("content-type", "application/json")
            .body(Body::from(json!({"worker_id":worker}).to_string()))
            .unwrap()
    };
    assert_eq!(
        router.clone().oneshot(request()).await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(
        router.clone().oneshot(request()).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    let receipt = Request::builder().method("POST").uri(format!("/api/codex-control/group-ai/{id}/receipt")).header("content-type", "application/json")
        .body(Body::from(json!({"worker_id":worker,"result":{"schema":"elon.win_group_ai_result.v1","ok":true,"authorization":"secret"}}).to_string())).unwrap();
    assert_eq!(
        router.oneshot(receipt).await.unwrap().status(),
        StatusCode::OK
    );
    assert!(!runtime
        .win_codex_control
        .group_ai
        .status(Path::new("fixture"), &id)
        .unwrap()
        .to_string()
        .contains("secret"));
}
