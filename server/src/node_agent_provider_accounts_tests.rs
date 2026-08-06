use serde_json::json;

use crate::{
    node_agent_cli_probe::LocalCliProbeSnapshot,
    node_agent_provider_accounts::{accounts_payload, normalize_provider_id},
    node_agent_provider_auth_protocol::{codex_login_instructions, select_gemini_auth_method},
};

#[test]
fn provider_catalog_keeps_web_chat_as_explicitly_reserved() {
    let payload = accounts_payload(&LocalCliProbeSnapshot::default(), None, None, None, None);
    let providers = payload["providers"].as_array().unwrap();
    assert_eq!(payload["schema"], "elon.ai_provider_accounts.v2");
    assert_eq!(payload["schema_version"], 2);
    assert_eq!(payload["state_machine"]["terminal_immutable"], true);
    assert_eq!(providers[0]["protocol"], "codex_app_server_jsonrpc");
    assert_eq!(
        providers[0]["credential_vault"]["explicit_consent_required"],
        true
    );
    assert_eq!(providers[0]["credential_vault"]["automatic_backup"], false);
    assert_eq!(providers[1]["protocol"], "acp_v1_stdio");
    assert_eq!(providers[2]["protocol"], "claude_auth_cli_v1");
    assert_eq!(providers[3]["protocol"], "copilot_oauth_web_flow_v1");
    assert_eq!(providers[3]["logout_supported"], false);
    assert_eq!(providers[4]["id"], "chatgpt_web");
    assert_eq!(providers[4]["implementation_state"], "reserved");
    assert_eq!(providers[4]["official_login"], false);
    assert_eq!(providers[4]["enabled"], false);
    assert_eq!(
        providers[4]["protocol"],
        "official_web_chat_adapter_reserved_v1"
    );
    assert_eq!(providers[4]["capabilities"]["web_chat"], false);
    assert_eq!(providers[5]["id"], "gemini_web");
    assert_eq!(providers[5]["implementation_state"], "reserved");
}

#[test]
fn codex_device_code_requires_https_instructions() {
    let result = json!({
        "type": "chatgptDeviceCode",
        "loginId": "upstream-login",
        "verificationUrl": "https://auth.openai.com/codex/device",
        "userCode": "ABCD-1234"
    });
    let instructions = codex_login_instructions(&result).unwrap();
    assert_eq!(
        instructions.upstream_login_id.as_deref(),
        Some("upstream-login")
    );
    assert_eq!(instructions.user_code.as_deref(), Some("ABCD-1234"));
    assert!(instructions.auth_url.is_none());

    let insecure = json!({
        "type": "chatgptDeviceCode",
        "verificationUrl": "http://example.test/device",
        "userCode": "ABCD-1234"
    });
    assert!(codex_login_instructions(&insecure).is_err());
}

#[test]
fn gemini_prefers_the_official_personal_oauth_method() {
    let initialize = json!({
        "authMethods": [
            {"id":"gemini-api-key","name":"Use Gemini API key","type":"env_var"},
            {"id":"oauth-personal","name":"Log in with Google"},
            {"id":"vertex-ai","name":"Vertex AI"}
        ]
    });
    assert_eq!(
        select_gemini_auth_method(&initialize).as_deref(),
        Some("oauth-personal")
    );
}

#[test]
fn provider_aliases_do_not_enable_reserved_web_surfaces() {
    assert_eq!(normalize_provider_id("codex"), Some("codex_cli"));
    assert_eq!(normalize_provider_id("gemini_cli"), Some("gemini_cli"));
    assert_eq!(normalize_provider_id("claude"), Some("claude_cli"));
    assert_eq!(normalize_provider_id("copilot"), Some("copilot_cli"));
    assert_eq!(normalize_provider_id("chatgpt_web"), None);
    assert_eq!(normalize_provider_id("gemini_web"), None);
}
