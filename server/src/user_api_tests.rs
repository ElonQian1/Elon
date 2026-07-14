use super::*;
use crate::types::CliPromptMode;

#[test]
fn copilot_agent_label_shows_provider_and_model() {
    let (provider, label) = agent_display_meta("copilot:gpt-4o", "gpt-4o");
    assert_eq!(provider, "copilot");
    assert_eq!(label, "GPT-4o");
}

#[test]
fn generic_agent_label_prefers_model_over_provider() {
    let (provider, label) = agent_display_meta("openai", "gpt-4o-mini");
    assert_eq!(provider, "openai");
    assert_eq!(label, "GPT-4o mini");
}

#[test]
fn cli_label_keeps_provider_identity() {
    let option = AiCliOption {
        id: "codex:gpt-5".into(),
        label: "Codex CLI / gpt-5".into(),
        provider: "codex".into(),
        model: Some("gpt-5".into()),
        reasoning_effort: None,
        reasoning_summary: None,
        verbosity: None,
        bin: "codex".into(),
        args: Vec::new(),
        prompt_mode: CliPromptMode::Arg,
        timeout_secs: 1800,
    };
    assert_eq!(cli_option_display_label(&option), "GPT-5");
}

#[test]
fn available_agents_dedupe_prefers_cli_for_same_name() {
    let agents = vec![
        serde_json::json!({
            "name": "copilot:gpt-4o",
            "backend": "api",
            "provider": "copilot",
            "model": "gpt-4o",
            "label": "GPT-4o"
        }),
        serde_json::json!({
            "name": "copilot:gpt-4o",
            "backend": "cli",
            "provider": "copilot",
            "model": "gpt-4o",
            "label": "GPT-4o"
        }),
        serde_json::json!({
            "name": "openai",
            "backend": "api",
            "provider": "openai",
            "model": "gpt-4o",
            "label": "GPT-4o"
        }),
    ];

    let deduped = dedupe_available_agents(agents);

    assert_eq!(deduped.len(), 2);
    let copilot = deduped
        .iter()
        .find(|agent| agent["name"].as_str() == Some("copilot:gpt-4o"))
        .expect("copilot option should remain");
    assert_eq!(copilot["backend"].as_str(), Some("cli"));
    assert!(deduped
        .iter()
        .any(|agent| agent["name"].as_str() == Some("openai")));
}
