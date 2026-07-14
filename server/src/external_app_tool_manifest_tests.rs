use super::*;

#[test]
fn normalizes_prefixed_fb2_tool_ids() {
    assert_eq!(
        normalize_fb2_tool_id("fb2.search_matches").as_deref(),
        Some("search_matches")
    );
    assert_eq!(
        normalize_fb2_tool_id(" opinion_result_reviews ").as_deref(),
        Some("opinion_result_reviews")
    );
    assert!(normalize_fb2_tool_id("  ").is_none());
}

#[test]
fn manifest_status_never_exposes_secrets() {
    let status = manifest_status("not_configured", "missing_token");

    assert_eq!(status["secret_values_exposed"], false);
    assert!(status.get("FB2_MAIN_PROJECT_SHARED_SECRET").is_none());
}

#[test]
fn live_manifest_marks_manifest_only_tools_as_non_executable() {
    let policy = fb2_tool_execution_policy(&[
        "search_matches".to_string(),
        "record_context_feedback".to_string(),
        "context_quality_summary".to_string(),
    ]);

    assert_eq!(
        policy["schema"],
        "external_app.live_tool_execution_policy.v1"
    );
    assert_eq!(
        policy["chat_auto_executable_tool_ids"]
            .as_array()
            .unwrap()
            .first()
            .and_then(Value::as_str),
        Some("search_matches")
    );
    assert!(policy["manifest_only_tool_ids"]
        .as_array()
        .unwrap()
        .contains(&json!("record_context_feedback")));
    assert!(policy["manifest_only_tool_ids"]
        .as_array()
        .unwrap()
        .contains(&json!("context_quality_summary")));
    assert!(policy["main_project_allowed_missing_tool_ids"]
        .as_array()
        .unwrap()
        .contains(&json!("get_match_detail")));
    assert_eq!(policy["coverage_status"], "degraded");
}

#[test]
fn bb64a_static_manifest_exposes_local_mcp_tools() {
    let manifest = bb64a_static_tool_manifest();
    assert_eq!(manifest["status"], "ready");
    assert_eq!(manifest["source"], "bb64a:local-windows-mcp");
    assert!(manifest["tool_ids"]
        .as_array()
        .unwrap()
        .contains(&json!("bb64a_doctor")));
    assert!(
        manifest["main_project_tool_execution_policy"]["chat_auto_executable_tool_ids"]
            .as_array()
            .unwrap()
            .contains(&json!("close_all_proxies"))
    );
}
