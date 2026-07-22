//! Detects a durable Codex JSON terminal signal without treating a process
//! exit or an intermediate assistant/tool item as successful completion.

use std::{collections::HashSet, time::Duration, time::Instant};

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodexTerminalOutcome {
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CodexCompletionDisposition {
    Complete { final_reply: String },
    Failed,
    ResumeRequired,
}

#[derive(Debug, Default)]
pub(crate) struct CodexTerminalHint {
    pending_line: String,
    in_flight_items: HashSet<String>,
    terminal: Option<(CodexTerminalOutcome, Instant)>,
}

impl CodexTerminalHint {
    pub(crate) fn observe(&mut self, chunk: &str, observed_at: Instant) {
        self.pending_line.push_str(chunk);
        while let Some(newline) = self.pending_line.find('\n') {
            let line = self.pending_line[..newline].trim().to_string();
            self.pending_line.drain(..=newline);
            self.observe_line(&line, observed_at);
        }

        let trailing = self.pending_line.trim();
        if !trailing.is_empty() {
            if let Ok(value) = serde_json::from_str::<Value>(trailing) {
                self.pending_line.clear();
                self.observe_value(&value, observed_at);
            }
        }
    }

    fn observe_line(&mut self, line: &str, observed_at: Instant) {
        if line.is_empty() {
            return;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            self.observe_value(&value, observed_at);
        }
    }

    pub(crate) fn outcome(
        &self,
        now: Instant,
        terminal_grace: Duration,
        _final_message_grace: Duration,
    ) -> Option<CodexTerminalOutcome> {
        if let Some((outcome, observed_at)) = self.terminal {
            return (now.saturating_duration_since(observed_at) >= terminal_grace)
                .then_some(outcome);
        }
        None
    }

    fn observe_value(&mut self, value: &Value, observed_at: Instant) {
        match value.get("type").and_then(Value::as_str).unwrap_or("") {
            "turn.started" => {
                self.terminal = None;
                self.in_flight_items.clear();
            }
            "turn.completed" => {
                self.terminal = Some((CodexTerminalOutcome::Success, observed_at));
            }
            "turn.failed" => {
                self.terminal = Some((CodexTerminalOutcome::Failure, observed_at));
            }
            "item.started" => {
                if let Some(id) = item_id(value) {
                    self.in_flight_items.insert(id.to_string());
                }
            }
            "item.completed" => {
                if let Some(id) = item_id(value) {
                    self.in_flight_items.remove(id);
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn codex_completion_disposition(output: &str) -> CodexCompletionDisposition {
    let mut terminal = None;
    let mut final_reply = None;
    for value in output
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
    {
        match value.get("type").and_then(Value::as_str).unwrap_or("") {
            "turn.started" => {
                terminal = None;
                final_reply = None;
            }
            "turn.completed" => terminal = Some(CodexTerminalOutcome::Success),
            "turn.failed" => terminal = Some(CodexTerminalOutcome::Failure),
            "item.completed" => {
                let item = value.get("item").unwrap_or(&value);
                if item.get("type").and_then(Value::as_str) == Some("agent_message") {
                    final_reply = item
                        .get("text")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|text| !text.is_empty())
                        .map(str::to_string);
                }
            }
            _ => {}
        }
    }
    match (terminal, final_reply) {
        (Some(CodexTerminalOutcome::Success), Some(final_reply)) => {
            CodexCompletionDisposition::Complete { final_reply }
        }
        (Some(CodexTerminalOutcome::Failure), _) => CodexCompletionDisposition::Failed,
        _ => CodexCompletionDisposition::ResumeRequired,
    }
}

fn item_id(value: &Value) -> Option<&str> {
    value
        .get("item")
        .and_then(|item| item.get("id"))
        .and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_turn_completion_wins_after_short_drain_grace() {
        let started = Instant::now();
        let mut hint = CodexTerminalHint::default();
        hint.observe(
            r#"{"type":"item.completed","item":{"id":"msg","type":"agent_message","text":"done"}}"#,
            started,
        );
        hint.observe(
            r#"{"type":"turn.completed","usage":{"output_tokens":5}}"#,
            started,
        );
        assert_eq!(
            hint.outcome(
                started + Duration::from_secs(1),
                Duration::from_millis(750),
                Duration::from_secs(30),
            ),
            Some(CodexTerminalOutcome::Success)
        );
    }

    #[test]
    fn split_json_line_is_buffered_until_complete() {
        let started = Instant::now();
        let mut hint = CodexTerminalHint::default();
        hint.observe(r#"{"type":"turn.com"#, started);
        assert_eq!(
            hint.outcome(
                started + Duration::from_secs(1),
                Duration::from_millis(750),
                Duration::from_secs(30),
            ),
            None
        );

        hint.observe(r#"pleted","usage":{"output_tokens":5}}"#, started);
        assert_eq!(
            hint.outcome(
                started + Duration::from_secs(1),
                Duration::from_millis(750),
                Duration::from_secs(30),
            ),
            Some(CodexTerminalOutcome::Success)
        );
    }

    #[test]
    fn intermediate_message_is_cleared_by_following_tool_activity() {
        let started = Instant::now();
        let mut hint = CodexTerminalHint::default();
        hint.observe(
            r#"{"type":"item.completed","item":{"id":"msg","type":"agent_message","text":"working"}}"#,
            started,
        );
        hint.observe(
            r#"{"type":"item.started","item":{"id":"call","type":"command_execution"}}"#,
            started + Duration::from_secs(1),
        );
        assert_eq!(
            hint.outcome(
                started + Duration::from_secs(60),
                Duration::from_millis(750),
                Duration::from_secs(30),
            ),
            None
        );
    }

    #[test]
    fn final_message_without_turn_terminal_never_becomes_success() {
        let started = Instant::now();
        let mut hint = CodexTerminalHint::default();
        hint.observe(
            r#"{"type":"item.completed","item":{"id":"msg","type":"agent_message","text":"done"}}"#,
            started,
        );
        assert_eq!(
            hint.outcome(
                started + Duration::from_secs(30),
                Duration::from_millis(750),
                Duration::from_secs(30),
            ),
            None
        );
    }

    #[test]
    fn trusted_success_requires_turn_completed_and_parseable_final_reply() {
        let no_final = concat!(
            "{\"type\":\"turn.started\"}\n",
            "{\"type\":\"item.started\",\"item\":{\"id\":\"tool\",\"type\":\"command_execution\"}}\n",
            "{\"type\":\"item.completed\",\"item\":{\"id\":\"tool\",\"type\":\"command_execution\"}}\n"
        );
        assert_eq!(
            codex_completion_disposition(no_final),
            CodexCompletionDisposition::ResumeRequired
        );
        let complete = concat!(
            "{\"type\":\"turn.started\"}\n",
            "{\"type\":\"item.completed\",\"item\":{\"id\":\"msg\",\"type\":\"agent_message\",\"text\":\"done\"}}\n",
            "{\"type\":\"turn.completed\"}\n"
        );
        assert_eq!(
            codex_completion_disposition(complete),
            CodexCompletionDisposition::Complete {
                final_reply: "done".to_string()
            }
        );
    }
}
