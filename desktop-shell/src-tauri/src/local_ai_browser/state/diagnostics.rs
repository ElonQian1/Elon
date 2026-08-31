use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::super::adapter::{attachment_transport, private_rich_recovery as rich_recovery};

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
    const RICH_KINDS: &[&str] = &["finance", "chart", "weather", "media_gallery", "map"];
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

pub(super) fn private_rich_recovery(snapshot: Option<&Value>) -> Value {
    let Some(recovery) = snapshot.and_then(|event| event.get("privateRichRecovery")) else {
        return Value::Null;
    };
    rich_recovery::sanitize(Some(recovery))
}

pub(super) fn realtime_voice_state(event: Option<&Value>) -> Value {
    let field = |key: &str| event.and_then(|value| value.get(key));
    let bounded = |key: &str, max: u64| {
        field(key)
            .and_then(Value::as_u64)
            .unwrap_or_default()
            .min(max)
    };
    if event.and_then(Value::as_object).is_none() {
        return Value::Null;
    }
    json!({
        "type": "realtime_voice_state",
        "version": bounded("version", 4),
        "active": field("active").and_then(Value::as_bool).unwrap_or(false),
        "observedChannelCount": bounded("observedChannelCount", 32),
        "openChannelCount": bounded("openChannelCount", 32),
        "observedFrameCount": bounded("observedFrameCount", 1_000_000_000),
        "acceptedEventCount": bounded("acceptedEventCount", 1_000_000_000),
        "streamCount": bounded("streamCount", 32),
        "revision": bounded("revision", 1_000_000_000),
    })
}

pub(super) fn attachment_transport_state(event: Option<&Value>) -> Value {
    event
        .and_then(Value::as_object)
        .and_then(|event| attachment_transport::sanitize(event).ok())
        .unwrap_or(Value::Null)
}

fn has_text(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty())
}
