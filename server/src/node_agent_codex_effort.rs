pub(crate) fn normalize_codex_reasoning_effort(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => "none".to_string(),
        "minimal" => "minimal".to_string(),
        "low" => "low".to_string(),
        "medium" => "medium".to_string(),
        "high" => "high".to_string(),
        "xhigh" | "max" | "ultra" | "extra_high" => "xhigh".to_string(),
        _ => "medium".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_legacy_and_invalid_efforts_to_codex_supported_values() {
        assert_eq!(normalize_codex_reasoning_effort("ultra"), "xhigh");
        assert_eq!(normalize_codex_reasoning_effort("MAX"), "xhigh");
        assert_eq!(normalize_codex_reasoning_effort("unexpected"), "medium");
        assert_eq!(normalize_codex_reasoning_effort("high"), "high");
    }
}
