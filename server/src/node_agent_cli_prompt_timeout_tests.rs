use super::node_agent_cli_prompt_runner::{
    cli_prompt_timeout_secs_with_config, DEFAULT_SUPERVISED_CODEX_TIMEOUT_SECS,
};

#[test]
fn codex_full_access_prompt_gets_development_timeout() {
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", Some("danger_full_access"), false, None),
        1200
    );
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", Some("full_access"), false, None),
        1200
    );
}

#[test]
fn ordinary_prompt_timeouts_stay_short() {
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", Some("read_only"), false, None),
        300
    );
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", None, false, None),
        300
    );
    assert_eq!(
        cli_prompt_timeout_secs_with_config(" Codex ", None, false, None),
        300
    );
    assert_eq!(
        cli_prompt_timeout_secs_with_config(
            "copilot",
            Some("danger_full_access"),
            true,
            Some("7200"),
        ),
        180
    );
}

#[test]
fn supervised_codex_is_not_hard_killed_at_twenty_minutes() {
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", Some("full_access"), true, None),
        DEFAULT_SUPERVISED_CODEX_TIMEOUT_SECS
    );
    assert!(DEFAULT_SUPERVISED_CODEX_TIMEOUT_SECS > 20 * 60);
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", Some("full_access"), true, Some("7200")),
        7200
    );
    assert_eq!(
        cli_prompt_timeout_secs_with_config("codex", Some("full_access"), true, Some("1200")),
        DEFAULT_SUPERVISED_CODEX_TIMEOUT_SECS
    );
}
