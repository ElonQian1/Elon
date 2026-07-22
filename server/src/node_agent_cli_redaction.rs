//! UTF-8-safe redaction shared by every CLI output sink.

const REDACTED: &str = "[REDACTED]";
const OVERSIZE_LINE: &str = "[REDACTED: oversized CLI output line]\n";
const MAX_PENDING_BYTES: usize = 8 * 1024;

pub(crate) fn redact_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for part in input.split_inclusive('\n') {
        output.push_str(&redact_line(part));
    }
    output
}

fn redact_line(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut output = String::with_capacity(line.len());
    let mut copied = 0;
    let mut index = 0;
    while index < bytes.len() {
        if !is_key_byte(bytes[index]) || bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let key_start = index;
        while index < bytes.len() && is_key_byte(bytes[index]) {
            index += 1;
        }
        let key = &lower[key_start..index];
        if !sensitive_key(key) {
            continue;
        }
        let mut delimiter = index;
        while delimiter < bytes.len()
            && (bytes[delimiter].is_ascii_whitespace() || matches!(bytes[delimiter], b'"' | b'\''))
        {
            delimiter += 1;
        }
        if delimiter >= bytes.len() || !matches!(bytes[delimiter], b':' | b'=') {
            continue;
        }
        let mut value_start = delimiter + 1;
        while value_start < bytes.len() && bytes[value_start].is_ascii_whitespace() {
            value_start += 1;
        }
        let quote = if value_start < bytes.len() && matches!(bytes[value_start], b'"' | b'\'') {
            let quote = Some(bytes[value_start]);
            value_start += 1;
            quote
        } else {
            None
        };
        if key == "authorization" && lower[value_start..].starts_with("bearer ") {
            value_start += "bearer ".len();
        }
        let value_end = value_end(line, value_start, quote);
        if value_end == value_start {
            continue;
        }
        output.push_str(&line[copied..value_start]);
        output.push_str(REDACTED);
        copied = value_end;
        index = value_end;
    }
    output.push_str(&line[copied..]);
    output
}

fn value_end(line: &str, start: usize, quote: Option<u8>) -> usize {
    for (offset, ch) in line[start..].char_indices() {
        let terminal = match quote {
            Some(quote) => ch as u32 == quote as u32,
            None => ch.is_whitespace() || matches!(ch, ',' | ';' | '&'),
        };
        if terminal {
            return start + offset;
        }
    }
    line.len()
}

fn is_key_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

fn sensitive_key(key: &str) -> bool {
    matches!(
        key,
        "authorization"
            | "api_key"
            | "api-key"
            | "apikey"
            | "access_token"
            | "refresh_token"
            | "token"
            | "password"
            | "passwd"
            | "secret"
            | "credential"
    )
}

#[derive(Default)]
pub(crate) struct CliOutputRedactor {
    pending: String,
    discarding_oversize_line: bool,
}

impl CliOutputRedactor {
    pub(crate) fn push(&mut self, input: &str) -> String {
        let mut output = String::new();
        let mut remaining = input;
        if self.discarding_oversize_line {
            let Some(newline) = remaining.find('\n') else {
                return output;
            };
            remaining = &remaining[newline + 1..];
            self.discarding_oversize_line = false;
            output.push_str(OVERSIZE_LINE);
        }
        self.pending.push_str(remaining);
        if self.pending.len() > MAX_PENDING_BYTES {
            self.pending.clear();
            self.discarding_oversize_line = true;
            return output;
        }
        if let Some(end) = self.pending.rfind('\n').map(|index| index + 1) {
            let complete = self.pending[..end].to_string();
            self.pending.drain(..end);
            output.push_str(&redact_text(&complete));
        }
        output
    }

    pub(crate) fn finish(&mut self) -> String {
        if self.discarding_oversize_line {
            self.discarding_oversize_line = false;
            return OVERSIZE_LINE.to_string();
        }
        let pending = std::mem::take(&mut self.pending);
        redact_text(&pending)
    }

    #[cfg(test)]
    fn pending_bytes(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_secret_patterns_are_redacted_without_touching_unicode() {
        let input = "你好 TOKEN=fake-token-123 JSON=ok api_key=\"fake-key-456\"\nAuthorization: Bearer fake-bearer-789\n";
        let output = redact_text(input);
        assert!(output.contains("你好"));
        assert!(!output.contains("fake-token-123"));
        assert!(!output.contains("fake-key-456"));
        assert!(!output.contains("fake-bearer-789"));
        assert_eq!(output.matches(REDACTED).count(), 3);
    }

    #[test]
    fn streaming_redaction_handles_utf8_boundaries_and_split_keys() {
        let mut redactor = CliOutputRedactor::default();
        assert_eq!(redactor.push("进度 TOK"), "");
        let output = redactor.push("EN=fake-stream-only\n完成\n");
        assert!(output.contains("进度 TOKEN=[REDACTED]"));
        assert!(!output.contains("fake-stream-only"));
        assert!(output.contains("完成"));
        assert_eq!(redactor.finish(), "");
    }

    #[test]
    fn oversized_unterminated_output_is_bounded_and_discarded() {
        let mut redactor = CliOutputRedactor::default();
        assert_eq!(redactor.push(&"界".repeat(MAX_PENDING_BYTES)), "");
        assert!(redactor.pending_bytes() <= MAX_PENDING_BYTES);
        assert_eq!(redactor.push("tail\n"), OVERSIZE_LINE);
        assert_eq!(redactor.finish(), "");
    }

    #[test]
    fn durable_sidecar_journal_and_completion_sinks_apply_the_same_redaction() {
        let record = crate::node_agent_cli_sidecar_io::CliSidecarOutputRecord::chunk(
            "stdout",
            "TOKEN=fake-sidecar-only\n",
        );
        assert_eq!(record.text.as_deref(), Some("TOKEN=[REDACTED]\n"));

        let root = std::env::temp_dir().join(format!(
            "cli-redaction-sinks-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let journal = crate::node_agent_task_journal::TaskJournal::new(&root);
        journal
            .record_cli_chunk("task", "stdout", "api_key=fake-journal-only\n")
            .unwrap();
        let journal_output = journal.completion_output("task", 4_096).unwrap();
        assert!(journal_output.contains("api_key=[REDACTED]"));
        assert!(!journal_output.contains("fake-journal-only"));

        let (message, completion_output) = crate::node_agent_cli_done::cli_done_message_from_output(
            "task".into(),
            false,
            Some("password=fake-error-only".into()),
            "secret=fake-completion-only",
            "",
            None,
            None,
            None,
        );
        assert!(!completion_output.contains("fake-completion-only"));
        let serialized = serde_json::to_string(&message).unwrap();
        assert!(!serialized.contains("fake-error-only"));
        let _ = std::fs::remove_dir_all(root);
    }
}
