//! Real production redaction, compiled without unrelated CLI integration tests.
#![allow(dead_code, unused_imports)]

#[path = "../../../src/node_agent_cli_redaction.rs"]
mod node_agent_cli_redaction;
pub fn redact_text(input: &str) -> String {
    node_agent_cli_redaction::redact_text(input)
}
