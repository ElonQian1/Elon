use super::*;

fn url(value: &str) -> Url {
    value.parse().expect("test URL")
}

#[test]
fn bootstrap_and_vendor_navigation_are_allowed() {
    assert!(allows_navigation(&CHATGPT, &url("about:blank")));
    assert!(allows_navigation(
        &CHATGPT,
        &url("edge-error://edgewebdata/")
    ));
    assert!(allows_navigation(&CHATGPT, &url("https://chatgpt.com/")));
    assert!(allows_navigation(
        &CHATGPT,
        &url("https://auth.openai.com/login")
    ));
    assert!(allows_navigation(
        &CHATGPT,
        &url("https://accounts.google.com/o/oauth2/v2/auth")
    ));
}

#[test]
fn google_ai_mode_navigation_is_scoped_to_official_search_hosts() {
    assert!(allows_navigation(
        &GOOGLE_AI_MODE,
        &url("https://www.google.com/aimode")
    ));
    assert!(allows_navigation(
        &GOOGLE_AI_MODE,
        &url("https://www.google.com/search?udm=50&q=rust")
    ));
    assert!(!allows_navigation(
        &GOOGLE_AI_MODE,
        &url("https://accounts.google.com/v3/signin/identifier")
    ));
    assert!(!allows_navigation(
        &GOOGLE_AI_MODE,
        &url("https://mail.google.com/mail/u/0/")
    ));
    assert!(!allows_navigation(
        &GOOGLE_AI_MODE,
        &url("https://google.com.evil.example/aimode")
    ));
}

#[test]
fn google_ai_mode_is_registered_with_semantic_adapter() {
    let summary = provider_summary(&GOOGLE_AI_MODE);
    assert_eq!(summary.id, "google-ai-mode");
    assert_eq!(summary.login_mode, "guest_web_system_login");
    assert_eq!(summary.renderer_status, "active");
    assert_eq!(GOOGLE_AI_MODE.adapter, Some(ProviderAdapter::GoogleWeb));
    assert!(summary.adapter_actions.contains(&"send_prompt"));
}

#[test]
fn unsafe_navigation_is_rejected() {
    assert!(!allows_navigation(&CHATGPT, &url("http://chatgpt.com/")));
    assert!(!allows_navigation(
        &CHATGPT,
        &url("https://chatgpt.com:444/")
    ));
    assert!(!allows_navigation(
        &CHATGPT,
        &url("https://chatgpt.com.evil.example/")
    ));
    assert!(!allows_navigation(
        &CHATGPT,
        &url("https://mail.google.com/")
    ));
    assert!(!allows_navigation(
        &CHATGPT,
        &url("https://user@example.com/")
    ));
}

#[test]
fn owner_fingerprint_is_stable_separate_and_path_safe() {
    let first = owner_fingerprint("account-15692409892").unwrap();
    let second = owner_fingerprint("account-15692409892").unwrap();
    let other = owner_fingerprint("another-account").unwrap();
    assert_eq!(first, second);
    assert_ne!(first, other);
    assert_eq!(first.len(), 16);
    assert!(first.chars().all(|value| value.is_ascii_hexdigit()));
}

#[test]
fn cached_chatgpt_conversation_is_restored_without_restoring_auth_or_queries() {
    assert_eq!(
        restorable_start_url(&CHATGPT, Some("https://chatgpt.com/c/conversation_123"))
            .unwrap()
            .as_str(),
        "https://chatgpt.com/c/conversation_123"
    );
    assert_eq!(
        restorable_start_url(&CHATGPT, Some("https://auth.openai.com/login"))
            .unwrap()
            .as_str(),
        CHATGPT.start_url
    );
    assert_eq!(
        restorable_start_url(&CHATGPT, Some("https://chatgpt.com/c/one?token=private"))
            .unwrap()
            .as_str(),
        CHATGPT.start_url
    );
    assert_eq!(
        restorable_start_url(
            &GOOGLE_AI_MODE,
            Some("https://www.google.com/search?q=private&udm=50"),
        )
        .unwrap()
        .as_str(),
        GOOGLE_AI_MODE.start_url
    );
}

#[test]
fn adapter_command_does_not_accept_arbitrary_javascript() {
    assert!(adapter_command::build(
        CHATGPT.display_name,
        CHATGPT.adapter.unwrap().supported_actions(),
        "eval",
        Some("alert(1)".to_string()),
        None,
        None,
    )
    .is_err());
    assert!(adapter_command::build(
        CHATGPT.display_name,
        CHATGPT.adapter.unwrap().supported_actions(),
        "snapshot",
        None,
        None,
        Some("mcp_snapshot1".to_string()),
    )
    .is_ok());
    assert!(adapter_command::build(
        GOOGLE_AI_MODE.display_name,
        GOOGLE_AI_MODE.adapter.unwrap().supported_actions(),
        "send_prompt",
        Some("hi".to_string()),
        None,
        None,
    )
    .is_ok());
    assert!(adapter_command::build(
        GOOGLE_AI_MODE.display_name,
        GOOGLE_AI_MODE.adapter.unwrap().supported_actions(),
        "start_google_login",
        None,
        None,
        None,
    )
    .is_err());
}
