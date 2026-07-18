use std::collections::VecDeque;

use serde_json::{json, Map, Value};

use crate::node_agent_task_journal::TaskJournal;

const HEAD_LINES: usize = 8;
const TAIL_LINES: usize = 12;
const MAX_LINE_CHARS: usize = 600;
const MAX_MESSAGE_CHARS: usize = 4_000;
const MAX_COMMAND_CHARS: usize = 240;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RuntimeObservation {
    pub(crate) phase: Option<String>,
    pub(crate) current_command: Option<String>,
    pub(crate) progress: bool,
}

#[derive(Default)]
struct StreamAggregate {
    raw_lines: usize,
    raw_bytes: usize,
    head: Vec<String>,
    tail: VecDeque<String>,
}

impl StreamAggregate {
    fn observe(&mut self, text: &str) {
        self.raw_bytes = self.raw_bytes.saturating_add(text.len());
        for line in text.lines() {
            self.raw_lines = self.raw_lines.saturating_add(1);
            let line = truncate_chars(line, MAX_LINE_CHARS);
            if self.head.len() < HEAD_LINES {
                self.head.push(line.clone());
            }
            self.tail.push_back(line);
            while self.tail.len() > TAIL_LINES {
                self.tail.pop_front();
            }
        }
    }

    fn payload(&self, stream: &str) -> Option<Value> {
        if self.raw_lines == 0 && self.raw_bytes == 0 {
            return None;
        }
        let overlap = self
            .head
            .len()
            .saturating_add(self.tail.len())
            .saturating_sub(self.raw_lines);
        let tail = self.tail.iter().skip(overlap).cloned().collect::<Vec<_>>();
        let retained = self.head.len().saturating_add(tail.len());
        Some(json!({
            "type": "cli_output_summary",
            "stream": stream,
            "raw_line_count": self.raw_lines,
            "raw_byte_count": self.raw_bytes,
            "head": self.head,
            "tail": tail,
            "retained_line_count": retained,
            "truncated": self.raw_lines > retained,
        }))
    }
}

#[derive(Default)]
pub(crate) struct CliOutputJournalAggregate {
    stdout: StreamAggregate,
    stderr: StreamAggregate,
}

impl CliOutputJournalAggregate {
    pub(crate) fn observe(
        &mut self,
        journal: &TaskJournal,
        req_id: &str,
        stream: &str,
        text: &str,
    ) -> RuntimeObservation {
        if let Some((event, observation)) = codex_json_event(req_id, stream, text) {
            let _ = journal.append_event(event);
            return observation;
        }
        self.stream_mut(stream).observe(text);
        RuntimeObservation {
            phase: Some("reasoning".to_string()),
            current_command: None,
            progress: !text.trim().is_empty(),
        }
    }

    pub(crate) fn flush(&self, journal: &TaskJournal, req_id: &str) {
        for (stream, aggregate) in [("stdout", &self.stdout), ("stderr", &self.stderr)] {
            let Some(mut event) = aggregate.payload(stream) else {
                continue;
            };
            event["req_id"] = Value::String(req_id.to_string());
            event["at_ms"] = json!(now_ms());
            let _ = journal.append_event(event);
        }
    }

    fn stream_mut(&mut self, stream: &str) -> &mut StreamAggregate {
        if stream.eq_ignore_ascii_case("stderr") {
            &mut self.stderr
        } else {
            &mut self.stdout
        }
    }
}

pub(crate) fn progress_observation(text: &str) -> RuntimeObservation {
    codex_json_event("", "stdout", text)
        .map(|(_, observation)| observation)
        .unwrap_or_else(|| RuntimeObservation {
            phase: Some("reasoning".to_string()),
            current_command: None,
            progress: !text.trim().is_empty(),
        })
}

fn codex_json_event(req_id: &str, stream: &str, text: &str) -> Option<(Value, RuntimeObservation)> {
    let parsed: Value = serde_json::from_str(text.trim()).ok()?;
    let event_type = parsed.get("type").and_then(Value::as_str)?;
    if !matches!(event_type, "item.started" | "item.completed") {
        return None;
    }
    let lifecycle = event_type.strip_prefix("item.").unwrap_or(event_type);
    let item = parsed.get("item")?;
    let item_type = item
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let bounded_item = bounded_codex_item(item);
    let current_command = if item_type == "command_execution" && lifecycle == "started" {
        item.get("command")
            .and_then(Value::as_str)
            .map(sanitize_command)
            .filter(|value| !value.is_empty())
    } else {
        None
    };
    let phase = phase_for_item(item_type, lifecycle, current_command.as_deref());
    let event = json!({
        "type": "codex_item",
        "req_id": req_id,
        "stream": if stream.eq_ignore_ascii_case("stderr") { "stderr" } else { "stdout" },
        "lifecycle": lifecycle,
        "item": bounded_item,
        "at_ms": now_ms(),
    });
    Some((
        event,
        RuntimeObservation {
            phase: Some(phase.to_string()),
            current_command,
            progress: true,
        },
    ))
}

fn bounded_codex_item(item: &Value) -> Value {
    let mut out = Map::new();
    for key in ["id", "type", "status", "exit_code"] {
        if let Some(value) = item.get(key) {
            out.insert(key.to_string(), value.clone());
        }
    }
    if let Some(command) = item.get("command").and_then(Value::as_str) {
        out.insert(
            "command".to_string(),
            Value::String(sanitize_command(command)),
        );
    }
    if let Some(text) = item.get("text").and_then(Value::as_str) {
        out.insert(
            "text".to_string(),
            Value::String(bounded_head_tail(text, MAX_MESSAGE_CHARS)),
        );
    }
    if let Some(output) = item.get("aggregated_output").and_then(Value::as_str) {
        out.insert("output".to_string(), output_summary(output));
    }
    if let Some(changes) = item.get("changes").and_then(Value::as_array) {
        let changes = changes
            .iter()
            .take(100)
            .filter_map(|change| {
                let path = change.get("path").and_then(Value::as_str)?.trim();
                if path.is_empty() {
                    return None;
                }
                Some(json!({
                    "path": truncate_chars(path, 500),
                    "kind": change.get("kind").and_then(Value::as_str).unwrap_or("changed"),
                }))
            })
            .collect::<Vec<_>>();
        out.insert("changes".to_string(), Value::Array(changes));
    }
    Value::Object(out)
}

fn output_summary(output: &str) -> Value {
    let lines = output.lines().collect::<Vec<_>>();
    let head = lines
        .iter()
        .take(HEAD_LINES)
        .map(|line| truncate_chars(line, MAX_LINE_CHARS))
        .collect::<Vec<_>>();
    let mut tail = lines
        .iter()
        .rev()
        .take(TAIL_LINES)
        .map(|line| truncate_chars(line, MAX_LINE_CHARS))
        .collect::<Vec<_>>();
    tail.reverse();
    json!({
        "raw_line_count": lines.len(),
        "raw_byte_count": output.len(),
        "head": head,
        "tail": tail,
        "truncated": lines.len() > HEAD_LINES + TAIL_LINES || output.chars().count() > MAX_MESSAGE_CHARS,
    })
}

fn phase_for_item(item_type: &str, lifecycle: &str, command: Option<&str>) -> &'static str {
    match item_type {
        "command_execution" if lifecycle == "started" => command_phase(command.unwrap_or("")),
        "command_execution" => "reasoning",
        "file_change" => "editing",
        "agent_message" if lifecycle == "completed" => "finalizing",
        _ => "reasoning",
    }
}

fn command_phase(command: &str) -> &'static str {
    let lower = command.to_ascii_lowercase();
    if [
        "test",
        "cargo check",
        "clippy",
        "npm run build",
        "npm run lint",
        "gradle",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        "verification"
    } else if ["finish-ai-task", "git push", "git commit", "publish-"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        "finalizing"
    } else {
        "command"
    }
}

pub(crate) fn sanitize_command(command: &str) -> String {
    let mut words = command.split_whitespace().peekable();
    let mut out = Vec::new();
    let mut redact_words = 0usize;
    while let Some(word) = words.next() {
        if redact_words > 0 {
            out.push("[redacted]".to_string());
            redact_words -= 1;
            continue;
        }
        let lower = word.to_ascii_lowercase();
        if lower == "bearer" {
            out.push(word.to_string());
            redact_words = 1;
            continue;
        }
        if [
            "token",
            "secret",
            "password",
            "api_key",
            "apikey",
            "authorization",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
        {
            if let Some((key, _)) = word.split_once('=') {
                out.push(format!("{key}=[redacted]"));
            } else {
                out.push(word.to_string());
                redact_words = if lower.contains("authorization") {
                    2
                } else {
                    1
                };
            }
            continue;
        }
        out.push(word.to_string());
    }
    truncate_chars(&out.join(" "), MAX_COMMAND_CHARS)
}

fn bounded_head_tail(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let half = max_chars / 2;
    let head = text.chars().take(half).collect::<String>();
    let tail = text
        .chars()
        .rev()
        .take(half)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{head}\n...（已截断，保留首尾）...\n{tail}")
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out = text.chars().take(max_chars).collect::<String>();
    out.push_str("…");
    out
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_preview_redacts_secrets() {
        let command =
            sanitize_command("curl -H Authorization Bearer abc --token=secret cargo test");
        assert!(!command.contains("abc"));
        assert!(!command.contains("secret"));
        assert!(command.contains("[redacted]"));
    }

    #[test]
    fn codex_command_event_keeps_exit_and_bounded_output() {
        let raw = serde_json::to_string(&json!({
            "type":"item.completed",
            "item":{
                "id":"item-1", "type":"command_execution", "status":"failed",
                "command":"rg token=secret", "exit_code":2,
                "aggregated_output":(0..100).map(|n| format!("line-{n}")).collect::<Vec<_>>().join("\n")
            }
        }))
        .unwrap();
        let (event, observation) = codex_json_event("task-1", "stdout", &raw).unwrap();
        assert_eq!(event["item"]["exit_code"], 2);
        assert_eq!(event["item"]["output"]["raw_line_count"], 100);
        assert_eq!(event["item"]["output"]["truncated"], true);
        assert!(!event.to_string().contains("secret"));
        assert!(observation.progress);
    }

    #[test]
    fn high_volume_raw_output_becomes_one_bounded_summary() {
        let root = std::env::temp_dir().join(format!(
            "elon-output-aggregate-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let journal = TaskJournal::new(&root);
        let mut aggregate = CliOutputJournalAggregate::default();
        for index in 0..500 {
            aggregate.observe(
                &journal,
                "task-rg",
                "stdout",
                &format!("rg-result-{index}\n"),
            );
        }
        aggregate.flush(&journal, "task-rg");
        let snapshot = journal.snapshot("task-rg", 0, 20).unwrap();
        assert_eq!(snapshot.events.len(), 1);
        assert_eq!(snapshot.events[0].event["type"], "cli_output_summary");
        assert_eq!(snapshot.events[0].event["raw_line_count"], 500);
        assert_eq!(snapshot.events[0].event["truncated"], true);
        assert_eq!(
            snapshot.events[0].event["head"].as_array().unwrap().len(),
            HEAD_LINES
        );
        assert_eq!(
            snapshot.events[0].event["tail"].as_array().unwrap().len(),
            TAIL_LINES
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn short_output_does_not_double_count_overlapping_head_and_tail() {
        let mut aggregate = StreamAggregate::default();
        aggregate.observe("one\ntwo\nthree\n");
        let payload = aggregate.payload("stdout").unwrap();
        assert_eq!(payload["raw_line_count"], 3);
        assert_eq!(payload["retained_line_count"], 3);
        assert_eq!(payload["tail"].as_array().unwrap().len(), 0);
        assert_eq!(payload["truncated"], false);
    }
}
