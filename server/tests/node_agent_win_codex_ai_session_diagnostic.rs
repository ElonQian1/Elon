#[path = "../src/node_agent_win_codex_control/ai_session_diagnostic.rs"]
mod ai_session_diagnostic;

use serde_json::{json, Value};

#[test]
fn rich_structure_counts_use_fixed_allowlists_and_bounds() {
    let sanitized = ai_session_diagnostic::sanitize(Some(&json!({
        "content_part_counts": {
            "markdown": 2,
            "citation": 900_000,
            "private_part": 7
        },
        "rich_card_kind_counts": {
            "finance": 1,
            "weather": 2,
            "private_card": 9
        },
        "citation_count": 3,
        "linked_citation_count": 900_000,
        "citation_logo_count": 2
    })))
    .unwrap()
    .unwrap();

    assert_eq!(
        sanitized["content_part_counts"],
        json!({
            "markdown": 2,
            "citation": 100_000
        })
    );
    assert_eq!(
        sanitized["rich_card_kind_counts"],
        json!({
            "finance": 1,
            "weather": 2
        })
    );
    assert_eq!(sanitized["citation_count"], 3);
    assert_eq!(sanitized["linked_citation_count"], 3);
    assert_eq!(sanitized["citation_logo_count"], 2);
}

#[test]
fn rich_structure_diagnostics_strip_unknown_fields_and_secrets() {
    let sanitized = ai_session_diagnostic::sanitize(Some(&json!({
        "content_part_counts": "private prompt",
        "rich_card_kind_counts": {
            "private_card": "https://chatgpt.com/private"
        },
        "cookie": "cookie-secret",
        "token": "token-secret",
        "owner": "owner-secret"
    })))
    .unwrap()
    .unwrap();

    assert_eq!(sanitized["content_part_counts"], json!({}));
    assert_eq!(sanitized["rich_card_kind_counts"], json!({}));
    assert_eq!(sanitized["citation_count"], 0);
    assert_eq!(sanitized["linked_citation_count"], 0);
    assert_eq!(sanitized["citation_logo_count"], 0);
    let encoded = sanitized.to_string();
    for secret in [
        "private prompt",
        "chatgpt.com",
        "cookie-secret",
        "token-secret",
        "owner-secret",
        "private_card",
    ] {
        assert!(!encoded.contains(secret));
    }
    assert_eq!(sanitized["last_command_ok"], Value::Null);
}
