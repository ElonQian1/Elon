use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliTokenUsage {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub total_tokens: i64,
    pub model: Option<String>,
}

impl CliTokenUsage {
    pub fn has_tokens(&self) -> bool {
        self.total_tokens > 0 || self.input_tokens > 0 || self.output_tokens > 0
    }

    pub fn normalized(mut self) -> Option<Self> {
        if self.total_tokens <= 0 {
            self.total_tokens = self.input_tokens + self.output_tokens;
        }
        if self.input_tokens <= 0 && self.output_tokens <= 0 && self.total_tokens > 0 {
            self.input_tokens = self.total_tokens;
        }
        if self.has_tokens() {
            Some(self)
        } else {
            None
        }
    }

    fn add(&mut self, other: CliTokenUsage) {
        self.input_tokens += other.input_tokens.max(0);
        self.cached_input_tokens += other.cached_input_tokens.max(0);
        self.output_tokens += other.output_tokens.max(0);
        self.reasoning_tokens += other.reasoning_tokens.max(0);
        self.total_tokens += other.total_tokens.max(0);
        if self.model.is_none() {
            self.model = other.model;
        }
    }
}

#[allow(dead_code)]
pub fn usage_from_optional_parts(
    input_tokens: Option<u64>,
    cached_input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    reasoning_tokens: Option<u64>,
    total_tokens: Option<u64>,
    model: Option<String>,
) -> Option<CliTokenUsage> {
    CliTokenUsage {
        input_tokens: input_tokens.unwrap_or(0) as i64,
        cached_input_tokens: cached_input_tokens.unwrap_or(0) as i64,
        output_tokens: output_tokens.unwrap_or(0) as i64,
        reasoning_tokens: reasoning_tokens.unwrap_or(0) as i64,
        total_tokens: total_tokens.unwrap_or(0) as i64,
        model,
    }
    .normalized()
}

pub fn parse_cli_usage(text: &str) -> Option<CliTokenUsage> {
    let mut total = CliTokenUsage::default();
    let mut found = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !trimmed.contains("token")
            && !trimmed.contains("usage")
            && !trimmed.contains("turn.completed")
            && !trimmed.contains("response.done")
        {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        if let Some(usage) = usage_from_value(&value) {
            total.add(usage);
            found = true;
        }
    }

    if let Some(legacy_total) = parse_legacy_tokens_used(text) {
        total.add(CliTokenUsage {
            input_tokens: legacy_total,
            total_tokens: legacy_total,
            ..CliTokenUsage::default()
        });
        found = true;
    }

    found.then_some(total).and_then(CliTokenUsage::normalized)
}

pub fn usage_from_value(value: &Value) -> Option<CliTokenUsage> {
    let usage = value
        .get("usage")
        .or_else(|| value.pointer("/response/usage"))
        .or_else(|| value.get("token_count"))
        .or_else(|| value.get("info"))
        .unwrap_or(value);

    let input = pick_i64(
        usage,
        &[
            "input_tokens",
            "inputTokens",
            "prompt_tokens",
            "promptTokens",
            "input",
        ],
    );
    let cached = pick_i64(
        usage,
        &[
            "cached_input_tokens",
            "cachedInputTokens",
            "cache_read_input_tokens",
            "cacheReadInputTokens",
            "cached",
        ],
    )
    .or_else(|| {
        value_at_i64(
            usage,
            &[
                "/input_token_details/cached_tokens",
                "/input_tokens_details/cached_tokens",
                "/prompt_tokens_details/cached_tokens",
            ],
        )
    });
    let output = pick_i64(
        usage,
        &[
            "output_tokens",
            "outputTokens",
            "completion_tokens",
            "completionTokens",
            "output",
        ],
    );
    let reasoning = pick_i64(
        usage,
        &[
            "reasoning_output_tokens",
            "reasoningOutputTokens",
            "reasoning_tokens",
            "reasoningTokens",
            "reasoning",
        ],
    )
    .or_else(|| {
        value_at_i64(
            usage,
            &[
                "/output_token_details/reasoning_tokens",
                "/output_tokens_details/reasoning_tokens",
                "/completion_tokens_details/reasoning_tokens",
            ],
        )
    });
    let total = pick_i64(usage, &["total_tokens", "totalTokens", "total"]);
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/response/model").and_then(Value::as_str))
        .or_else(|| usage.get("model").and_then(Value::as_str))
        .map(str::to_string);

    CliTokenUsage {
        input_tokens: input.unwrap_or(0),
        cached_input_tokens: cached.unwrap_or(0),
        output_tokens: output.unwrap_or(0),
        reasoning_tokens: reasoning.unwrap_or(0),
        total_tokens: total.unwrap_or(0),
        model,
    }
    .normalized()
}

fn pick_i64(obj: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| obj.get(*key).and_then(value_to_i64))
        .filter(|n| *n > 0)
}

fn value_at_i64(obj: &Value, pointers: &[&str]) -> Option<i64> {
    pointers
        .iter()
        .find_map(|ptr| obj.pointer(ptr).and_then(value_to_i64))
        .filter(|n| *n > 0)
}

fn value_to_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|n| i64::try_from(n).ok()))
}

fn parse_legacy_tokens_used(text: &str) -> Option<i64> {
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower == "tokens used" {
            if let Some(next) = lines.peek().copied() {
                if let Some(n) = parse_number_token(next) {
                    return Some(n);
                }
            }
        } else if lower.starts_with("tokens used") {
            if let Some(n) = parse_number_token(trimmed) {
                return Some(n);
            }
        }
    }
    None
}

fn parse_number_token(text: &str) -> Option<i64> {
    let digits: String = text.chars().filter(|ch| ch.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<i64>().ok().filter(|n| *n > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_codex_token_count_usage() {
        let text = r#"{"type":"token_count","model":"gpt-5.4","usage":{"input_tokens":1200,"cached_input_tokens":300,"output_tokens":80,"total_tokens":1280}}"#;
        let usage = parse_cli_usage(text).expect("usage should parse");
        assert_eq!(usage.input_tokens, 1200);
        assert_eq!(usage.cached_input_tokens, 300);
        assert_eq!(usage.output_tokens, 80);
        assert_eq!(usage.total_tokens, 1280);
        assert_eq!(usage.model.as_deref(), Some("gpt-5.4"));
    }

    #[test]
    fn parses_realtime_response_done_usage() {
        let value: Value = serde_json::from_str(
            r#"{"type":"response.done","response":{"model":"gpt-realtime","usage":{"input_tokens":10,"output_tokens":20,"input_token_details":{"cached_tokens":4},"output_token_details":{"reasoning_tokens":3}}}}"#,
        )
        .unwrap();
        let usage = usage_from_value(&value).expect("usage should parse");
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.cached_input_tokens, 4);
        assert_eq!(usage.output_tokens, 20);
        assert_eq!(usage.reasoning_tokens, 3);
        assert_eq!(usage.total_tokens, 30);
        assert_eq!(usage.model.as_deref(), Some("gpt-realtime"));
    }

    #[test]
    fn parses_legacy_tokens_used_total() {
        let usage = parse_cli_usage("codex\n完成\n\ntokens used\n12,345\n").unwrap();
        assert_eq!(usage.input_tokens, 12_345);
        assert_eq!(usage.total_tokens, 12_345);
    }
}
