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
    assert!(GOOGLE_AI_MODE.semantic_adapter);
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
fn adapter_command_does_not_accept_arbitrary_javascript() {
    assert!(adapter_command::build(
        CHATGPT.id,
        CHATGPT.display_name,
        GOOGLE_AI_MODE.id,
        "eval",
        Some("alert(1)".to_string()),
        None,
    )
    .is_err());
    assert!(adapter_command::build(
        CHATGPT.id,
        CHATGPT.display_name,
        GOOGLE_AI_MODE.id,
        "snapshot",
        None,
        None,
    )
    .is_ok());
    assert!(adapter_command::build(
        GOOGLE_AI_MODE.id,
        GOOGLE_AI_MODE.display_name,
        GOOGLE_AI_MODE.id,
        "send_prompt",
        Some("hi".to_string()),
        None,
    )
    .is_ok());
    assert!(adapter_command::build(
        GOOGLE_AI_MODE.id,
        GOOGLE_AI_MODE.display_name,
        GOOGLE_AI_MODE.id,
        "start_google_login",
        None,
        None,
    )
    .is_err());
}
