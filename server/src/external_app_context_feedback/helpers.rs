use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

use crate::external_app_context_config::FB2_APP_ID;
use crate::external_app_context_source_validation::validate_reply_sources;

use super::{MAX_CITED_SOURCES, FEEDBACK_NOTE_MAX_CHARS};

pub(super) fn feedback_context_needs_platform_order_scope(context: &Value) -> bool {
    // fb2 会按 context_audit_id 复核最初拉取 Context Pack 时的 scope。
    // 总结帖若引用了平台匿名订单摘要，feedback 回写也必须携带同一 scope，否则会被 403 拒绝。
    context
        .get("platform_order_summary")
        .map(value_has_content)
        .unwrap_or(false)
        || context
            .get("citation_sources")
            .and_then(Value::as_array)
            .map(|sources| {
                sources.iter().any(|source| {
                    source.get("kind").and_then(Value::as_str) == Some("platform_order_summary")
                })
            })
            .unwrap_or(false)
        || context
            .get("context_pack")
            .and_then(Value::as_str)
            .map(|pack| pack.contains("platform_order_summary"))
            .unwrap_or(false)
}

pub(super) fn value_has_content(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(_) | Value::Number(_) => true,
        Value::String(value) => !value.trim().is_empty(),
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => !values.is_empty(),
    }
}

pub(super) fn generated_answer_feedback_payload(
    context: &Value,
    external_user_id: Option<&str>,
    main_group_id: &str,
    main_request_id: &str,
    trigger: &str,
    reply_text: &str,
    tool_results: Option<&Value>,
    extra_citation_sources: &[Value],
) -> Option<Value> {
    let context_audit_id = context
        .get("context_audit_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let group_id = context
        .get("group")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(main_group_id);
    let cited_sources =
        feedback_citation_sources(context, reply_text, tool_results, extra_citation_sources);
    let cited_source_count = cited_sources.len();
    let source_validation = validate_reply_sources(
        context,
        tool_results,
        reply_text,
        &cited_sources,
        extra_citation_sources,
    );

    let mut payload = json!({
        "context_audit_id": context_audit_id,
        "main_request_id": main_request_id,
        "group_id": group_id,
        "cited_source_count": cited_source_count,
        "cited_sources": cited_sources,
        // 独立审计字段：记录本次回答引用是否闭环，不把 tool-only source 合成到 cited_sources。
        "answer_source_validation": source_validation.answer_source_validation_summary(
            main_request_id,
            context_audit_id,
            cited_source_count
        ),
        "missing_context": source_validation.has_missing_explicit_sources(),
        "wrong_context": source_validation.has_unmatched_sources(),
        "note": truncate_chars(
            &format!(
                "auto_generated_answer_feedback; trigger={trigger}; cited_sources={}; {}",
                cited_source_count,
                source_validation.note_fragment()
            ),
            FEEDBACK_NOTE_MAX_CHARS
        )
    });
    if let Some(external_user_id) = external_user_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        payload["external_user_id"] = Value::String(external_user_id.to_string());
    }
    Some(payload)
}

pub(super) fn mentioned_citation_sources(context: &Value, reply_text: &str) -> Vec<Value> {
    let reply = reply_text.to_lowercase();
    context
        .get("citation_sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|source| citation_source_is_mentioned(source, &reply))
        .take(MAX_CITED_SOURCES)
        .cloned()
        .collect()
}

pub(super) fn feedback_citation_sources(
    context: &Value,
    reply_text: &str,
    tool_results: Option<&Value>,
    _extra_citation_sources: &[Value],
) -> Vec<Value> {
    let mut cited_sources = mentioned_citation_sources(context, reply_text);
    if cited_sources.len() >= MAX_CITED_SOURCES {
        return cited_sources;
    }

    let mut seen = cited_sources
        .iter()
        .filter_map(citation_source_key)
        .collect::<HashSet<_>>();
    for source in mentioned_tool_result_citation_sources(context, tool_results, reply_text) {
        let Some(key) = citation_source_key(&source) else {
            continue;
        };
        if seen.insert(key) {
            cited_sources.push(source);
            if cited_sources.len() >= MAX_CITED_SOURCES {
                return cited_sources;
            }
        }
    }
    cited_sources
}

pub(super) fn mentioned_tool_result_citation_sources(
    context: &Value,
    tool_results: Option<&Value>,
    reply_text: &str,
) -> Vec<Value> {
    let Some(results) = tool_results
        .and_then(|value| value.get("results"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    let reply = reply_text.to_lowercase();
    let context_sources = context_citation_sources_by_id(context);
    let mut cited_sources = Vec::new();
    let mut seen = HashSet::new();

    for result in results {
        if !tool_result_sources_are_allowed(result) {
            continue;
        }
        for source_id in result
            .get("source_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(source_id_as_string)
        {
            if source_id.chars().count() < 4 || !reply.contains(&source_id.to_lowercase()) {
                continue;
            }
            let Some(mapped_sources) = context_sources.get(&source_id.to_lowercase()).cloned()
            else {
                continue;
            };
            for source in mapped_sources {
                let Some(key) = citation_source_key(&source) else {
                    continue;
                };
                if seen.insert(key) {
                    cited_sources.push(source);
                    if cited_sources.len() >= MAX_CITED_SOURCES {
                        return cited_sources;
                    }
                }
            }
        }
    }

    cited_sources
}

pub(super) fn context_citation_sources_by_id(context: &Value) -> HashMap<String, Vec<Value>> {
    let mut out: HashMap<String, Vec<Value>> = HashMap::new();
    for source in context
        .get("citation_sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(id) = source
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        out.entry(id.to_lowercase())
            .or_default()
            .push(source.clone());
    }
    out
}

pub(super) fn source_id_as_string(value: &Value) -> Option<String> {
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

pub(super) fn tool_result_sources_are_allowed(result: &Value) -> bool {
    if result.get("success").and_then(Value::as_bool) != Some(true) {
        return false;
    }
    matches!(
        result
            .get("grounding")
            .and_then(|grounding| grounding.get("status"))
            .and_then(Value::as_str),
        Some("grounded" | "weak")
    )
}

pub(super) fn citation_source_key(source: &Value) -> Option<String> {
    let id = source
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let kind = source
        .get("kind")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("source");
    Some(format!("{kind}:{id}"))
}

pub(super) fn mentioned_opinion_memory_ids(
    feedback_payload: &Value,
    tool_results: Option<&Value>,
    reply_text: &str,
) -> Vec<String> {
    let reply = reply_text.to_lowercase();
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for source in feedback_payload
        .get("cited_sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        push_mentioned_opinion_memory_source(&mut out, &mut seen, source, &reply);
    }

    let Some(results) = tool_results
        .and_then(|value| value.get("results"))
        .and_then(Value::as_array)
    else {
        return out;
    };

    for result in results {
        if result.get("tool_name").and_then(Value::as_str) != Some("opinion_memories") {
            continue;
        }
        if result.get("success").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        if result
            .get("grounding")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            != Some("grounded")
        {
            continue;
        }

        for id in result
            .get("source_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            push_mentioned_opinion_memory_id(&mut out, &mut seen, id, id, &reply);
        }
        for memory in result
            .get("data")
            .and_then(|value| value.get("memories"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(memory_id) = memory.get("id").and_then(Value::as_str) else {
                continue;
            };
            push_mentioned_opinion_memory_id(&mut out, &mut seen, memory_id, memory_id, &reply);
            if let Some(source_message_id) = memory.get("source_message_id").and_then(Value::as_str)
            {
                push_mentioned_opinion_memory_id(
                    &mut out,
                    &mut seen,
                    memory_id,
                    source_message_id,
                    &reply,
                );
            }
        }
    }
    out
}

pub(super) fn push_mentioned_opinion_memory_source(
    out: &mut Vec<String>,
    seen: &mut HashSet<String>,
    source: &Value,
    lower_reply: &str,
) {
    let kind = source
        .get("kind")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(kind.as_str(), "opinion_memory" | "group_opinion_memory") {
        return;
    }
    let Some(memory_id) = source.get("id").and_then(Value::as_str) else {
        return;
    };
    push_mentioned_opinion_memory_id(out, seen, memory_id, memory_id, lower_reply);
    if let Some(label) = source.get("label").and_then(Value::as_str) {
        push_mentioned_opinion_memory_id(out, seen, memory_id, label, lower_reply);
    }
    if let Some(message_id) = source.get("message_id").and_then(Value::as_str) {
        push_mentioned_opinion_memory_id(out, seen, memory_id, message_id, lower_reply);
    }
    if let Some(source_message_id) = source.get("source_message_id").and_then(Value::as_str) {
        push_mentioned_opinion_memory_id(out, seen, memory_id, source_message_id, lower_reply);
    }
}

pub(super) fn push_mentioned_opinion_memory_id(
    out: &mut Vec<String>,
    seen: &mut HashSet<String>,
    memory_id: &str,
    marker: &str,
    lower_reply: &str,
) {
    let marker = marker.trim();
    if marker.chars().count() < 4 || !lower_reply.contains(&marker.to_lowercase()) {
        return;
    }
    let memory_id = memory_id.trim();
    if !memory_id.is_empty() && seen.insert(memory_id.to_string()) && out.len() < MAX_CITED_SOURCES
    {
        out.push(memory_id.to_string());
    }
}

pub(super) fn citation_source_is_mentioned(source: &Value, lower_reply: &str) -> bool {
    ["id", "label"].iter().any(|field| {
        source
            .get(*field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| value.chars().count() >= 4)
            .map(|value| lower_reply.contains(&value.to_lowercase()))
            .unwrap_or(false)
    })
}

pub(super) fn is_fb2_context_pack(context: &Value) -> bool {
    context.get("app_id").and_then(Value::as_str) == Some(FB2_APP_ID)
        && context.get("status").and_then(Value::as_str) == Some("ready")
        && context.get("source").and_then(Value::as_str)
            == Some("fb2:/api/main-project/context/pack")
        && context
            .get("context_audit_id")
            .and_then(Value::as_str)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

pub(super) fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut out = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}


#[cfg(test)]
#[path = "external_app_context_feedback_tests.rs"]
mod tests;
