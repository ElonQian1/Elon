use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const ANALYSIS_SCHEMA: &str = "yilong.web-ai.capture-analysis.v1";
const MAX_TOKENS: usize = 16;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureAnalysis {
    pub schema: String,
    pub analyzer_version: u16,
    pub policy_available: bool,
    pub decoded_frame_count: u32,
    pub accepted_frame_count: u32,
    pub assistant_frame_count: u32,
    pub progress_frame_count: u32,
    pub text_length: u32,
    pub rich_kinds: Vec<String>,
    pub content_types: Vec<String>,
    #[serde(default)]
    pub unsupported_rich_count: u32,
    pub completed: bool,
    pub parse_error: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResearchCaptureStatus {
    capture_count: usize,
    analyzed_capture_count: usize,
    latest_analyzed_at_ms: u64,
    compatibility: String,
    decoded_frame_count: u32,
    accepted_frame_count: u32,
    assistant_frame_count: u32,
    text_length: u32,
    rich_kinds: Vec<String>,
    content_types: Vec<String>,
    unsupported_rich_count: u32,
    completed: bool,
    truncated: bool,
}

impl Default for ResearchCaptureStatus {
    fn default() -> Self {
        Self {
            capture_count: 0,
            analyzed_capture_count: 0,
            latest_analyzed_at_ms: 0,
            compatibility: "not_available".to_string(),
            decoded_frame_count: 0,
            accepted_frame_count: 0,
            assistant_frame_count: 0,
            text_length: 0,
            rich_kinds: Vec::new(),
            content_types: Vec::new(),
            unsupported_rich_count: 0,
            completed: false,
            truncated: false,
        }
    }
}

pub(super) fn validate(analysis: Option<&CaptureAnalysis>) -> Result<(), String> {
    let Some(analysis) = analysis else {
        return Ok(());
    };
    if analysis.schema != ANALYSIS_SCHEMA
        || !matches!(analysis.analyzer_version, 1 | 2)
        || analysis.decoded_frame_count > 100_000
        || analysis.accepted_frame_count > analysis.decoded_frame_count
        || analysis.assistant_frame_count > analysis.decoded_frame_count
        || analysis.progress_frame_count > analysis.decoded_frame_count
        || analysis.text_length > 10_000_000
        || analysis.unsupported_rich_count > 32
        || !valid_tokens(&analysis.rich_kinds, 32)
        || !valid_tokens(&analysis.content_types, 40)
    {
        return Err("研究响应解析摘要无效。".to_string());
    }
    Ok(())
}

pub(super) fn read_status(root: &Path) -> Result<ResearchCaptureStatus, String> {
    if !root.is_dir() {
        return Ok(ResearchCaptureStatus::default());
    }
    let mut status = ResearchCaptureStatus::default();
    let mut latest: Option<(u64, CaptureAnalysis, bool)> = None;
    for entry in fs::read_dir(root)
        .map_err(display_error)?
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.ends_with(".meta.json"))
        {
            continue;
        }
        status.capture_count += 1;
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(metadata) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        if metadata.get("schema").and_then(Value::as_str)
            != Some("yilong.web-ai.research-capture.v1")
        {
            continue;
        }
        let Some(raw_analysis) = metadata.get("analysis").cloned() else {
            continue;
        };
        let Ok(analysis) = serde_json::from_value::<CaptureAnalysis>(raw_analysis) else {
            continue;
        };
        if validate(Some(&analysis)).is_err() {
            continue;
        }
        status.analyzed_capture_count += 1;
        let stored_at = metadata
            .get("storedAtMs")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let truncated = metadata
            .get("truncated")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if latest
            .as_ref()
            .is_none_or(|(known_at, _, _)| stored_at >= *known_at)
        {
            latest = Some((stored_at, analysis, truncated));
        }
    }
    if let Some((stored_at, analysis, truncated)) = latest {
        status.latest_analyzed_at_ms = stored_at;
        status.compatibility = compatibility(&analysis, truncated).to_string();
        status.decoded_frame_count = analysis.decoded_frame_count;
        status.accepted_frame_count = analysis.accepted_frame_count;
        status.assistant_frame_count = analysis.assistant_frame_count;
        status.text_length = analysis.text_length;
        status.rich_kinds = analysis.rich_kinds;
        status.content_types = analysis.content_types;
        status.unsupported_rich_count = analysis.unsupported_rich_count;
        status.completed = analysis.completed;
        status.truncated = truncated;
    }
    Ok(status)
}

fn compatibility(analysis: &CaptureAnalysis, truncated: bool) -> &'static str {
    if truncated {
        "truncated"
    } else if !analysis.policy_available {
        "analyzer_unavailable"
    } else if analysis.parse_error {
        "parse_error"
    } else if analysis.decoded_frame_count == 0 {
        "empty_stream"
    } else if analysis.unsupported_rich_count > 0 {
        "renderer_upgrade_required"
    } else if analysis.accepted_frame_count == 0 {
        "upstream_changed"
    } else if analysis
        .content_types
        .iter()
        .any(|value| value == "google_rpc")
        && analysis.assistant_frame_count == 0
        && analysis.rich_kinds.is_empty()
    {
        "structure_observed"
    } else if analysis.rich_kinds.is_empty()
        && analysis.assistant_frame_count == 0
        && analysis.text_length == 0
    {
        "incomplete"
    } else if analysis.rich_kinds.is_empty() {
        "text_compatible"
    } else {
        "rich_compatible"
    }
}

fn valid_tokens(values: &[String], max_length: usize) -> bool {
    values.len() <= MAX_TOKENS
        && values.iter().all(|value| {
            !value.is_empty()
                && value.len() <= max_length
                && value.bytes().enumerate().all(|(index, byte)| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || (index > 0 && matches!(byte, b'_' | b'-'))
                })
        })
}

fn display_error(error: impl std::fmt::Display) -> String {
    format!("无法读取本机网页 AI 研究解析状态：{error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoded_but_unaccepted_stream_signals_upstream_change() {
        let analysis = sample();
        assert_eq!(compatibility(&analysis, false), "upstream_changed");
    }

    #[test]
    fn recognized_finance_stream_is_rich_compatible() {
        let mut analysis = sample();
        analysis.accepted_frame_count = 3;
        analysis.assistant_frame_count = 2;
        analysis.text_length = 320;
        analysis.rich_kinds = vec!["finance".to_string()];
        assert_eq!(compatibility(&analysis, false), "rich_compatible");
        assert!(validate(Some(&analysis)).is_ok());
    }

    #[test]
    fn unsupported_private_widget_requires_renderer_upgrade_even_with_text() {
        let mut analysis = sample();
        analysis.analyzer_version = 2;
        analysis.accepted_frame_count = 3;
        analysis.assistant_frame_count = 1;
        analysis.text_length = 320;
        analysis.unsupported_rich_count = 1;
        assert_eq!(compatibility(&analysis, false), "renderer_upgrade_required");
        assert!(validate(Some(&analysis)).is_ok());
    }

    #[test]
    fn parsed_google_rpc_without_a_stable_content_mapping_is_observed() {
        let mut analysis = sample();
        analysis.decoded_frame_count = 1;
        analysis.accepted_frame_count = 1;
        analysis.content_types = vec!["google_rpc".to_string(), "batched_json".to_string()];
        assert_eq!(compatibility(&analysis, false), "structure_observed");
        assert!(validate(Some(&analysis)).is_ok());
    }

    fn sample() -> CaptureAnalysis {
        CaptureAnalysis {
            schema: ANALYSIS_SCHEMA.to_string(),
            analyzer_version: 1,
            policy_available: true,
            decoded_frame_count: 4,
            accepted_frame_count: 0,
            assistant_frame_count: 0,
            progress_frame_count: 0,
            text_length: 0,
            rich_kinds: Vec::new(),
            content_types: vec!["text".to_string()],
            unsupported_rich_count: 0,
            completed: true,
            parse_error: false,
        }
    }
}
