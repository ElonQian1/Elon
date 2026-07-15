use serde_json::json;
use std::{fs, path::PathBuf};

use super::{
    get_status, mark_applied, mark_dispatched, mark_failure, mark_session_ready,
    record_tool_failure, record_tool_success, start_operation,
};

fn workspace() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon_project_docs_observability_test_{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(root.join(".git")).unwrap();
    root
}

#[test]
fn trace_exposes_each_successful_stage_and_token_budget() {
    let root = workspace();
    let operation = "docs_observable_success";
    start_operation(&root, Some(operation)).unwrap();
    mark_dispatched(&root, Some(operation), Some("task-7")).unwrap();
    mark_session_ready(&root, "docs_session_7").unwrap();
    record_tool_success(
        &root,
        "project_docs_analyze",
        &json!({
            "catalog_revision":"catalog-1",
            "pagination":{"matching_documents":119},
            "budget":{"ambiguous_documents":57}
        }),
    );
    record_tool_success(
        &root,
        "project_docs_read",
        &json!({"documents_read":3,"estimated_tokens_returned":240}),
    );
    record_tool_success(
        &root,
        "project_docs_save_suggestions",
        &json!({
            "catalog_revision":"catalog-1",
            "suggestions_revision":"suggestions-1",
            "authorization_mode":"review_all",
            "suggestions":{"documents_read":3,"estimated_tokens_used":240}
        }),
    );
    let awaiting = get_status(&root, Some(operation)).unwrap();
    assert_eq!(awaiting["status"], "awaiting_review");
    assert_eq!(awaiting["current_stage"], "awaiting_review");
    assert_eq!(awaiting["documents_cataloged"], 119);
    assert_eq!(awaiting["ambiguous_documents"], 57);
    assert_eq!(awaiting["documents_read"], 3);
    assert_eq!(awaiting["estimated_tokens_used"], 240);

    record_tool_success(
        &root,
        "project_docs_apply_suggestions",
        &json!({
            "manifest_revision":"manifest-1",
            "suggestions_revision":"suggestions-2"
        }),
    );
    let applied = get_status(&root, Some(operation)).unwrap();
    assert_eq!(applied["status"], "succeeded");
    assert_eq!(applied["current_stage"], "applied");
    assert_eq!(applied["task_id"], "task-7");
    assert!(applied["events"].as_array().unwrap().len() >= 7);

    mark_session_ready(&root, "docs_session_refresh").unwrap();
    mark_dispatched(&root, Some(operation), Some("task-refresh")).unwrap();
    record_tool_success(
        &root,
        "project_docs_analyze",
        &json!({
            "catalog_revision":"catalog-2",
            "pagination":{"matching_documents":119},
            "budget":{"ambiguous_documents":57}
        }),
    );
    let refreshed = get_status(&root, Some(operation)).unwrap();
    assert_eq!(refreshed["status"], "succeeded");
    assert_eq!(refreshed["current_stage"], "applied");
    assert_eq!(refreshed["session_id"], "docs_session_refresh");
    assert_eq!(refreshed["task_id"], "task-refresh");
    assert_eq!(
        refreshed["events"].as_array().unwrap().len(),
        applied["events"].as_array().unwrap().len()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn trusted_mode_stays_observable_through_real_file_apply_stage() {
    let root = workspace();
    let operation = "docs_trusted_apply";
    start_operation(&root, Some(operation)).unwrap();
    record_tool_success(
        &root,
        "project_docs_save_suggestions",
        &json!({
            "catalog_revision":"catalog-1",
            "suggestions_revision":"suggestions-1",
            "authorization_mode":"trusted_reversible",
            "suggestions":{"documents_read":1,"estimated_tokens_used":40}
        }),
    );
    assert_eq!(
        get_status(&root, Some(operation)).unwrap()["current_stage"],
        "suggestions_ready"
    );
    record_tool_success(
        &root,
        "project_docs_apply_suggestions",
        &json!({
            "manifest_revision":"manifest-1",
            "suggestions_revision":"suggestions-2",
            "suggestions":{"file_operations":[{"status":"proposed"}]}
        }),
    );
    assert_eq!(
        get_status(&root, Some(operation)).unwrap()["current_stage"],
        "virtual_applied"
    );
    record_tool_success(
        &root,
        "project_docs_apply_file_operations",
        &json!({
            "catalog_revision":"catalog-2",
            "manifest_revision":"manifest-2",
            "suggestions_revision":"suggestions-3"
        }),
    );
    let applied = get_status(&root, Some(operation)).unwrap();
    assert_eq!(applied["status"], "succeeded");
    assert_eq!(applied["current_stage"], "files_applied");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn trace_identifies_failure_step_and_recovery_then_allows_retry() {
    let root = workspace();
    let operation = "docs_observable_failure";
    start_operation(&root, Some(operation)).unwrap();
    mark_failure(
        &root,
        Some(operation),
        "dispatch_failed",
        "节点离线",
        "启动节点后重试",
    )
    .unwrap();
    let failed = get_status(&root, Some(operation)).unwrap();
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["error"]["code"], "dispatch_failed");
    assert_eq!(failed["error"]["recovery"], "启动节点后重试");

    record_tool_failure(
        &root,
        "project_docs_save_suggestions",
        &anyhow::anyhow!("目录 revision 过期"),
    );
    let save_failed = get_status(&root, Some(operation)).unwrap();
    assert_eq!(save_failed["error"]["code"], "save_suggestions_failed");
    assert!(save_failed["error"]["recovery"]
        .as_str()
        .unwrap()
        .contains("重新 analyze"));

    record_tool_success(
        &root,
        "project_docs_analyze",
        &json!({
            "catalog_revision":"fresh",
            "pagination":{"matching_documents":1},
            "budget":{"ambiguous_documents":0}
        }),
    );
    let recovered = get_status(&root, Some(operation)).unwrap();
    assert_eq!(recovered["status"], "running");
    assert!(recovered.get("error").is_none());
    assert!(get_status(&root, Some("another-operation")).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reviewed_pc_apply_closes_the_same_observable_operation() {
    let root = workspace();
    start_operation(&root, Some("docs_pc_apply")).unwrap();
    mark_applied(
        &root,
        Some("docs_pc_apply"),
        Some("manifest-ui"),
        Some("suggestions-ui"),
    )
    .unwrap();
    let status = get_status(&root, Some("docs_pc_apply")).unwrap();
    assert_eq!(status["status"], "succeeded");
    assert_eq!(status["manifest_revision"], "manifest-ui");
    assert_eq!(status["suggestions_revision"], "suggestions-ui");
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn loopback_api_exposes_the_same_trace_contract() {
    let root = workspace();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            crate::project_document_observability_api::test_routes(),
        )
        .await
        .unwrap();
    });
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let base = format!("http://127.0.0.1:{port}/api/project-docs/organization");
    let root_string = root.display().to_string();
    let started: serde_json::Value = client
        .post(format!("{base}/start"))
        .json(&json!({"project_root":root_string.clone(),"operation_id":"docs_http_test"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(started["trace"]["current_stage"], "requested");
    let status: serde_json::Value = client
        .post(format!("{base}/status"))
        .json(&json!({"project_root":root_string,"operation_id":"docs_http_test"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["trace"]["operation_id"], "docs_http_test");
    server.abort();
    fs::remove_dir_all(root).unwrap();
}
