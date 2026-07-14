use super::*;

#[test]
fn exposes_public_context_readiness_guidance() {
    let guidance = public_context_readiness_guidance("fb2").unwrap();
    assert_eq!(guidance["schema"], "fb2.context_readiness.v1");
    let metadata = guidance["main_project_prompt_metadata"].as_array().unwrap();
    assert!(metadata.contains(&json!("answer_policy")));
    assert!(metadata.contains(&json!("answer_rules")));
    assert!(metadata.contains(&json!("citation_sources")));
    assert!(metadata.contains(&json!("executed_external_app_tools")));
    assert!(guidance["required_response_fields"]
        .as_array()
        .unwrap()
        .contains(&json!("citation_sources")));
    assert!(guidance["automated_checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["name"] == "has_source_ids"));
    assert!(public_context_readiness_guidance("unknown").is_none());
}

#[test]
fn normalizes_nested_fb2_readiness_status() {
    let readiness = normalized_live_readiness(&json!({
        "context_pack_version": "fb2-chat-pack-v1",
        "context_readiness": {"status": "blocked"},
        "checks": [{"name": "has_orders", "status": "failed"}]
    }));

    assert_eq!(
        readiness["schema"],
        "external_app.live_context_readiness.v1"
    );
    assert_eq!(readiness["status"], "blocked");
    assert_eq!(readiness["secret_values_exposed"], false);
    assert!(readiness["warnings"]
        .as_array()
        .unwrap()
        .contains(&json!("fb2_readiness_blocked")));
}

#[test]
fn readiness_status_never_exposes_secrets() {
    let status = readiness_status("not_configured", "missing_token");

    assert_eq!(status["secret_values_exposed"], false);
    assert!(status.get("FB2_MAIN_PROJECT_SHARED_SECRET").is_none());
}
