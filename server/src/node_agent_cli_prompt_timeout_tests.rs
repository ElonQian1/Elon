use super::node_agent_cli_prompt_runner::cli_prompt_timeout_secs;

#[test]
fn codex_full_access_prompt_gets_development_timeout() {
    assert_eq!(
        cli_prompt_timeout_secs("codex", Some("danger_full_access")),
        1200
    );
    assert_eq!(cli_prompt_timeout_secs("codex", Some("full_access")), 1200);
}

#[test]
fn ordinary_prompt_timeouts_stay_short() {
    assert_eq!(cli_prompt_timeout_secs("codex", Some("read_only")), 300);
    assert_eq!(cli_prompt_timeout_secs("codex", None), 300);
    assert_eq!(cli_prompt_timeout_secs(" Codex ", None), 300);
    assert_eq!(
        cli_prompt_timeout_secs("copilot", Some("danger_full_access")),
        180
    );
}
