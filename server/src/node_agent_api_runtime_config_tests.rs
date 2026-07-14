use super::*;

#[test]
fn status_requires_key_and_model_for_route_b_ready() {
    let missing_model = status_from_lookup(|name| match name {
        "OPENAI_API_KEY" => Some("sk-test".to_string()),
        _ => None,
    });
    assert!(missing_model.key_configured);
    assert!(!missing_model.model_configured);
    assert!(!missing_model.ready);

    let ready = status_from_lookup(|name| match name {
        "OPENAI_API_KEY" => Some("sk-test".to_string()),
        "OPENAI_MODEL" => Some("gpt-test".to_string()),
        _ => None,
    });
    assert!(ready.ready);
    assert_eq!(ready.api_base, DEFAULT_OPENAI_API_BASE);
}

#[test]
fn validate_save_rejects_env_file_injection() {
    assert!(validate_save("sk-test\nOPENAI_MODEL=x", Some("gpt-test"), None).is_err());
    assert!(validate_save("sk-test", Some("gpt-test\rbad"), None).is_err());
}

#[test]
fn validate_save_normalizes_api_base() {
    let save = validate_save(
        "sk-test",
        Some(" gpt-test "),
        Some(" https://example.test/v1/ "),
    )
    .expect("valid route b config");
    assert_eq!(save.model.as_deref(), Some("gpt-test"));
    assert_eq!(save.api_base.as_deref(), Some("https://example.test/v1"));
}

#[test]
fn upsert_env_file_updates_commented_lines() {
    let path =
        std::env::temp_dir().join(format!("elon-api-runtime-env-{}.env", uuid::Uuid::new_v4()));
    std::fs::write(&path, "#OPENAI_MODEL=old\nOTHER=1\n").expect("seed env");
    upsert_env_file(&path, "OPENAI_MODEL", "new").expect("upsert env");
    let text = std::fs::read_to_string(&path).expect("read env");
    let _ = std::fs::remove_file(&path);
    assert!(text.contains("OPENAI_MODEL=new\n"));
    assert!(text.contains("OTHER=1"));
}

#[test]
fn tool_contract_exposes_route_b_capabilities_and_guardrails() {
    let contract = tool_contract();
    assert_eq!(contract.route, "route_b_api_runtime");
    assert!(contract
        .supported_tools
        .contains(&"search_files".to_string()));
    assert!(contract.supported_tools.contains(&"file_info".to_string()));
    assert!(contract.read_only_tools.contains(&"file_info".to_string()));
    assert!(contract
        .supported_tools
        .contains(&"read_file_range".to_string()));
    assert!(contract.supported_tools.contains(&"git_status".to_string()));
    assert!(contract.supported_tools.contains(&"git_diff".to_string()));
    assert!(contract.supported_tools.contains(&"git_log".to_string()));
    assert!(contract.supported_tools.contains(&"git_show".to_string()));
    assert!(contract.read_only_tools.contains(&"git_status".to_string()));
    assert!(contract.read_only_tools.contains(&"git_diff".to_string()));
    assert!(contract.read_only_tools.contains(&"git_log".to_string()));
    assert!(contract.read_only_tools.contains(&"git_show".to_string()));
    assert!(contract
        .supported_tools
        .contains(&"apply_patch".to_string()));
    assert!(contract
        .supported_tools
        .contains(&"run_command".to_string()));
    assert!(contract
        .approval_required_tools
        .contains(&"write_file".to_string()));
    assert!(contract
        .approval_required_tools
        .contains(&"apply_patch".to_string()));
    assert!(contract
        .command_policy
        .contains("structured_project_command_allowlist"));
    assert!(contract.command_policy.contains("danger_full_access"));
    assert!(contract
        .recovery_policy
        .contains("without_original_tty_reattach"));
}
