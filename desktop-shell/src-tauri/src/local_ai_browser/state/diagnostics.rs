use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Default)]
pub(super) struct ContentCoverage {
    pub(super) part_counts: BTreeMap<String, u64>,
    pub(super) rich_kind_counts: BTreeMap<String, u64>,
    pub(super) citation_count: u64,
    pub(super) linked_citation_count: u64,
    pub(super) citation_logo_count: u64,
}

pub(super) fn content_coverage(snapshot: Option<&Value>) -> ContentCoverage {
    const PART_TYPES: &[&str] = &[
        "text",
        "markdown",
        "citation",
        "image",
        "file",
        "code",
        "table",
        "artifact",
        "audio",
        "video",
        "math",
        "chart",
        "map",
        "interactive",
        "rich_card",
    ];
    const RICH_KINDS: &[&str] = &["finance", "weather", "media_gallery", "map"];
    let mut coverage = ContentCoverage::default();
    let messages = snapshot
        .and_then(|value| value.get("messages"))
        .and_then(Value::as_array);
    for part in messages
        .into_iter()
        .flatten()
        .filter_map(|message| message.get("content").and_then(Value::as_array))
        .flatten()
    {
        let Some(part_type) = part
            .get("type")
            .and_then(Value::as_str)
            .filter(|value| PART_TYPES.contains(value))
        else {
            continue;
        };
        *coverage
            .part_counts
            .entry(part_type.to_string())
            .or_default() += 1;
        if part_type == "citation" {
            coverage.citation_count += 1;
            if has_text(part, "markerText") && has_text(part, "citationId") {
                coverage.linked_citation_count += 1;
            }
            if has_text(part, "iconUrl") {
                coverage.citation_logo_count += 1;
            }
        }
        if part_type == "rich_card" {
            let kind = part
                .get("richContent")
                .and_then(|value| value.get("kind"))
                .and_then(Value::as_str)
                .or_else(|| part.get("kind").and_then(Value::as_str));
            if let Some(kind) = kind.filter(|value| RICH_KINDS.contains(value)) {
                *coverage
                    .rich_kind_counts
                    .entry(kind.to_string())
                    .or_default() += 1;
            }
        }
    }
    coverage
}

fn has_text(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty())
}
