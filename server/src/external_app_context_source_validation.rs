//! external_app_context_source_validation.rs
//! Validate fb2 source ids mentioned by generated answers.

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

const MAX_VALIDATION_IDS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceValidation {
    matched_source_ids: Vec<String>,
    unmatched_source_ids: Vec<String>,
    candidate_source_ids: Vec<String>,
    allowed_tool_source_ids: Vec<String>,
    matched_tool_source_ids: Vec<String>,
}

impl SourceValidation {
    pub(crate) fn has_unmatched_sources(&self) -> bool {
        !self.unmatched_source_ids.is_empty()
    }

    pub(crate) fn note_fragment(&self) -> String {
        if self.candidate_source_ids.is_empty() {
            return "source_validation=no_explicit_source_ids".to_string();
        }
        if self.unmatched_source_ids.is_empty() {
            return format!(
                "source_validation=ok matched={}",
                compact_ids(&self.matched_source_ids)
            );
        }
        format!(
            "source_validation=unmatched count={} ids={}",
            self.unmatched_source_ids.len(),
            compact_ids(&self.unmatched_source_ids)
        )
    }

    pub(crate) fn answer_source_validation_summary(
        &self,
        main_request_id: &str,
        context_audit_id: &str,
        cited_source_count: usize,
    ) -> Value {
        // 该摘要只用于审计单次回答引用闭环，不改变 fb2 cited_sources 的统计口径。
        let status = if self.candidate_source_ids.is_empty() {
            "no_explicit_source_ids"
        } else if self.unmatched_source_ids.is_empty() {
            "ok"
        } else {
            "unmatched"
        };
        json!({
            "schema": "external_app.answer_source_validation.v1",
            "main_request_id": main_request_id,
            "context_audit_id": context_audit_id,
            "status": status,
            "has_unmatched_sources": self.has_unmatched_sources(),
            "cited_source_count": cited_source_count,
            "candidate_source_ids": self.candidate_source_ids,
            "matched_source_ids": self.matched_source_ids,
            "unmatched_source_ids": self.unmatched_source_ids,
            "matched_tool_source_ids": self.matched_tool_source_ids,
            "allowed_tool_source_ids": self.allowed_tool_source_ids,
            "rule": "AI answer source ids must come from the Context Pack source registry, selected-message extras, the context audit id, or grounded/weak tool results."
        })
    }
}

pub(crate) fn validate_reply_sources(
    context: &Value,
    tool_results: Option<&Value>,
    reply_text: &str,
    cited_sources: &[Value],
    extra_citation_sources: &[Value],
) -> SourceValidation {
    let allowed = allowed_source_ids(context, tool_results, cited_sources, extra_citation_sources);
    let allowed_tool_source_ids = allowed_tool_source_ids(tool_results);
    let tool_source_lookup = allowed_tool_source_ids
        .iter()
        .map(|id| id.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let candidates = reply_source_candidates(reply_text, &allowed);
    let mut matched_source_ids = Vec::new();
    let mut unmatched_source_ids = Vec::new();
    let mut matched_tool_source_ids = Vec::new();
    let mut matched_seen = HashSet::new();
    let mut matched_tool_seen = HashSet::new();
    let mut unmatched_seen = HashSet::new();

    for candidate in &candidates {
        let key = candidate.to_ascii_lowercase();
        if let Some(canonical) = allowed.get(&key) {
            if matched_seen.insert(canonical.to_ascii_lowercase()) {
                matched_source_ids.push(canonical.clone());
            }
            if tool_source_lookup.contains(&canonical.to_ascii_lowercase())
                && matched_tool_seen.insert(canonical.to_ascii_lowercase())
            {
                matched_tool_source_ids.push(canonical.clone());
            }
        } else if unmatched_seen.insert(key) {
            unmatched_source_ids.push(candidate.clone());
        }
        if matched_source_ids.len() >= MAX_VALIDATION_IDS
            && unmatched_source_ids.len() >= MAX_VALIDATION_IDS
        {
            break;
        }
    }

    SourceValidation {
        matched_source_ids,
        unmatched_source_ids,
        candidate_source_ids: candidates,
        allowed_tool_source_ids,
        matched_tool_source_ids,
    }
}

fn allowed_tool_source_ids(tool_results: Option<&Value>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for result in grounded_or_weak_tool_results(tool_results) {
        for source_id in result
            .get("source_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(source_id_as_string)
        {
            let key = source_id.to_ascii_lowercase();
            if seen.insert(key) {
                out.push(source_id);
                if out.len() >= MAX_VALIDATION_IDS {
                    return out;
                }
            }
        }
    }
    out
}

fn allowed_source_ids(
    context: &Value,
    tool_results: Option<&Value>,
    cited_sources: &[Value],
    extra_citation_sources: &[Value],
) -> HashMap<String, String> {
    let mut out = HashMap::new();

    for source in context
        .get("citation_sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        insert_source_fields(&mut out, source);
    }
    for source in cited_sources {
        insert_source_fields(&mut out, source);
    }
    for source in extra_citation_sources {
        insert_source_fields(&mut out, source);
    }
    if let Some(context_audit_id) = context.get("context_audit_id").and_then(Value::as_str) {
        insert_source_id(&mut out, context_audit_id);
    }

    for result in grounded_or_weak_tool_results(tool_results) {
        for source_id in result
            .get("source_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(source_id_as_string)
        {
            insert_source_id(&mut out, &source_id);
        }
        insert_tool_memory_sources(&mut out, result);
    }

    out
}

fn insert_source_fields(out: &mut HashMap<String, String>, source: &Value) {
    for field in ["id", "message_id", "source_message_id", "context_audit_id"] {
        if let Some(value) = source.get(field).and_then(Value::as_str) {
            insert_source_id(out, value);
        }
    }
}

fn insert_tool_memory_sources(out: &mut HashMap<String, String>, result: &Value) {
    for memory in result
        .get("data")
        .and_then(|value| value.get("memories"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(id) = memory.get("id").and_then(Value::as_str) {
            insert_source_id(out, id);
        }
        if let Some(message_id) = memory.get("source_message_id").and_then(Value::as_str) {
            insert_source_id(out, message_id);
        }
    }
}

fn insert_source_id(out: &mut HashMap<String, String>, value: &str) {
    let trimmed = value.trim();
    if trimmed.chars().count() < 4 {
        return;
    }
    out.entry(trimmed.to_ascii_lowercase())
        .or_insert_with(|| trimmed.to_string());
}

fn grounded_or_weak_tool_results(tool_results: Option<&Value>) -> Vec<&Value> {
    let Some(results) = tool_results
        .and_then(|value| value.get("results"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    results
        .iter()
        .filter(|result| result.get("success").and_then(Value::as_bool) == Some(true))
        .filter(|result| {
            matches!(
                result
                    .get("grounding")
                    .and_then(|grounding| grounding.get("status"))
                    .and_then(Value::as_str),
                Some("grounded" | "weak")
            )
        })
        .collect()
}

fn source_id_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn reply_source_candidates(reply_text: &str, allowed: &HashMap<String, String>) -> Vec<String> {
    let lower_reply = reply_text.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    // 先按已知来源表反查，避免 fb2 使用 UUID 或自定义 source id 时漏掉合法引用。
    for (lower_id, canonical) in allowed {
        if lower_reply.contains(lower_id) {
            push_candidate(&mut out, &mut seen, canonical);
        }
    }

    let mut token = String::new();
    for ch in reply_text.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':') {
            token.push(ch);
        } else {
            push_token_if_source_like(&mut out, &mut seen, &token);
            token.clear();
        }
    }
    push_token_if_source_like(&mut out, &mut seen, &token);

    out.truncate(MAX_VALIDATION_IDS * 2);
    out
}

fn push_token_if_source_like(out: &mut Vec<String>, seen: &mut HashSet<String>, token: &str) {
    let trimmed = token.trim_matches(|ch| matches!(ch, '-' | '_' | ':'));
    if !is_source_like_token(trimmed) {
        return;
    }
    push_candidate(out, seen, trimmed);
}

fn push_candidate(out: &mut Vec<String>, seen: &mut HashSet<String>, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    if seen.insert(trimmed.to_ascii_lowercase()) {
        out.push(trimmed.to_string());
    }
}

fn is_source_like_token(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    if lower.chars().count() < 4 || is_field_name(&lower) {
        return false;
    }
    if is_uuid_like(&lower) {
        return true;
    }
    if lower.starts_with("platform_order_summary:") {
        return true;
    }
    [
        "match-",
        "match_",
        "match:",
        "m-",
        "odds-",
        "odds_",
        "odds:",
        "order-",
        "order_",
        "order:",
        "ticket-",
        "ticket_",
        "ticket:",
        "gmsg_",
        "gai_",
        "gsp_",
        "opinion-",
        "opinion_",
        "memory-",
        "memory_",
        "context_audit",
        "audit-",
        "audit_",
        "ext-",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
}

fn is_field_name(value: &str) -> bool {
    matches!(
        value,
        "match_id"
            | "order_id"
            | "ticket_id"
            | "message_id"
            | "source_id"
            | "context_audit_id"
            | "memory_id"
            | "memory_ids"
            | "opinion_memory_id"
            | "opinion_memory_ids"
    )
}

fn is_uuid_like(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (index, byte) in bytes.iter().enumerate() {
        match index {
            8 | 13 | 18 | 23 => {
                if *byte != b'-' {
                    return false;
                }
            }
            _ => {
                if !byte.is_ascii_hexdigit() {
                    return false;
                }
            }
        }
    }
    true
}

fn compact_ids(ids: &[String]) -> String {
    if ids.is_empty() {
        return "none".to_string();
    }
    let mut compact = ids.iter().take(3).cloned().collect::<Vec<_>>().join(",");
    if ids.len() > 3 {
        compact.push_str(",...");
    }
    compact
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flags_unmatched_source_like_ids() {
        let context = json!({
            "context_audit_id": "audit-1",
            "citation_sources": [
                {"kind": "match", "id": "match-1", "label": "比赛1"}
            ]
        });
        let validation = validate_reply_sources(
            &context,
            None,
            "数据事实引用 match-1，但也写了不存在的 order-404。",
            &[],
            &[],
        );

        assert!(validation.has_unmatched_sources());
        assert_eq!(validation.unmatched_source_ids, vec!["order-404"]);
        assert_eq!(validation.matched_source_ids, vec!["match-1"]);
    }

    #[test]
    fn accepts_grounded_tool_source_ids_without_context_registry_match() {
        let context = json!({"context_audit_id": "audit-1", "citation_sources": []});
        let tool_results = json!({
            "results": [{
                "tool_name": "match_analysis_brief",
                "success": true,
                "grounding": {"status": "grounded"},
                "source_ids": ["order-tool-1"]
            }]
        });
        let validation = validate_reply_sources(
            &context,
            Some(&tool_results),
            "用户订单：引用 order-tool-1 作为当前用户票据来源。",
            &[],
            &[],
        );

        assert!(!validation.has_unmatched_sources());
        assert_eq!(validation.matched_source_ids, vec!["order-tool-1"]);
        assert_eq!(validation.matched_tool_source_ids, vec!["order-tool-1"]);
        assert_eq!(validation.allowed_tool_source_ids, vec!["order-tool-1"]);
        let summary = validation.answer_source_validation_summary("main-req-1", "audit-1", 0);
        assert_eq!(
            summary["schema"],
            "external_app.answer_source_validation.v1"
        );
        assert_eq!(summary["status"], "ok");
        assert_eq!(summary["main_request_id"], "main-req-1");
        assert_eq!(summary["context_audit_id"], "audit-1");
        assert_eq!(summary["matched_tool_source_ids"][0], "order-tool-1");
        assert_eq!(summary["cited_source_count"], 0);
    }

    #[test]
    fn ignores_dates_and_plain_field_names() {
        let context = json!({"context_audit_id": "audit-1"});
        let validation = validate_reply_sources(
            &context,
            None,
            "2026-06-23 的 match_id 与 opinion_memory_id 字段缺失，所以这里只能说明数据不足。",
            &[],
            &[],
        );

        assert!(!validation.has_unmatched_sources());
        assert!(validation.candidate_source_ids.is_empty());
    }
}
