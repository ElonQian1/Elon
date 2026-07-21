//! Detects a durable Codex JSON terminal signal without treating every
//! assistant message as completion. A bounded fallback handles CLI versions
//! that emit the final agent message but fail to close the process.

use std::{collections::HashSet, time::Duration, time::Instant};

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodexTerminalOutcome {
    Success,
    Failure,
}

#[derive(Debug, Default)]
pub(crate) struct CodexTerminalHint {
    pending_line: String,
    in_flight_items: HashSet<String>,
    terminal: Option<(CodexTerminalOutcome, Instant)>,
    final_message_candidate_at: Option<Instant>,
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
        final_message_grace: Duration,
    ) -> Option<CodexTerminalOutcome> {
        if let Some((outcome, observed_at)) = self.terminal {
            return (now.saturating_duration_since(observed_at) >= terminal_grace)
                .then_some(outcome);
        }
        self.final_message_candidate_at
            .filter(|_| self.in_flight_items.is_empty())
            .filter(|observed_at| {
                now.saturating_duration_since(*observed_at) >= final_message_grace
            })
            .map(|_| CodexTerminalOutcome::Success)
    }

    fn observe_value(&mut self, value: &Value, observed_at: Instant) {
        match value.get("type").and_then(Value::as_str).unwrap_or("") {
            "turn.started" => {
                self.terminal = None;
                self.final_message_candidate_at = None;
            }
            "turn.completed" => {
                self.terminal = Some((CodexTerminalOutcome::Success, observed_at));
            }
            "turn.failed" => {
                self.terminal = Some((CodexTerminalOutcome::Failure, observed_at));
            }
            "item.started" => {
                self.final_message_candidate_at = None;
                if let Some(id) = item_id(value) {
                    self.in_flight_items.insert(id.to_string());
                }
            }
            "item.completed" => {
                if let Some(id) = item_id(value) {
                    self.in_flight_items.remove(id);
                }
                let item = value.get("item").unwrap_or(value);
                if item.get("type").and_then(Value::as_str) == Some("agent_message")
                    && item
                        .get("text")
                        .and_then(Value::as_str)
                        .is_some_and(|text| !text.trim().is_empty())
                {
                    self.final_message_candidate_at = Some(observed_at);
                } else {
                    self.final_message_candidate_at = None;
                }
            }
            _ => {}
        }
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
    fn final_message_fallback_requires_no_in_flight_items_and_long_grace() {
        let started = Instant::now();
        let mut hint = CodexTerminalHint::default();
        hint.observe(
            r#"{"type":"item.completed","item":{"id":"msg","type":"agent_message","text":"done"}}"#,
            started,
        );
        assert_eq!(
            hint.outcome(
                started + Duration::from_secs(29),
                Duration::from_millis(750),
                Duration::from_secs(30),
            ),
            None
        );
        assert_eq!(
            hint.outcome(
                started + Duration::from_secs(30),
                Duration::from_millis(750),
                Duration::from_secs(30),
            ),
            Some(CodexTerminalOutcome::Success)
        );
    }
}
