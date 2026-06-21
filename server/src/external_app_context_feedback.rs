//! Feedback callbacks from generated main-project answers to child app context services.

use serde_json::{json, Value};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tracing::{info, warn};

use crate::{
    external_app_context_config::{
        fb2_base_url, fb2_context_token, fb2_request_context_headers, timeout_secs, FB2_APP_ID,
        FB2_CONTEXT_HEADER,
    },
    types::AppState,
};

const MAX_CITED_SOURCES: usize = 12;
const FEEDBACK_NOTE_MAX_CHARS: usize = 180;

pub(crate) fn spawn_generated_answer_feedback(
    state: Arc<AppState>,
    user_id: String,
    main_group_id: String,
    main_request_id: String,
    trigger: &'static str,
    external_context: Option<Value>,
    external_tool_results: Option<Value>,
    reply_text: String,
) {
    let Some(context) = external_context else {
        return;
    };
    if !is_fb2_context_pack(&context) {
        return;
    }

    tokio::spawn(async move {
        if let Err(error) = post_generated_answer_feedback(
            &state,
            &user_id,
            &main_group_id,
            &main_request_id,
            trigger,
            &context,
            external_tool_results.as_ref(),
            &reply_text,
        )
        .await
        {
            warn!(
                user_id,
                main_group_id,
                trigger,
                error = %error,
                "fb2 generated-answer feedback callback failed"
            );
        }
    });
}

async fn post_generated_answer_feedback(
    state: &Arc<AppState>,
    user_id: &str,
    main_group_id: &str,
    main_request_id: &str,
    trigger: &str,
    context: &Value,
    tool_results: Option<&Value>,
    reply_text: &str,
) -> anyhow::Result<()> {
    let Some(base_url) = fb2_base_url() else {
        return Ok(());
    };
    let Some(token) = fb2_context_token() else {
        return Ok(());
    };

    let external_user_id = state
        .store
        .external_account_for_main_user(FB2_APP_ID, user_id)
        .map_err(|error| {
            warn!("fb2 external account lookup failed before feedback callback: {error}");
            error
        })
        .ok()
        .flatten()
        .map(|account| account.external_user_id);

    let Some(payload) = generated_answer_feedback_payload(
        context,
        external_user_id.as_deref(),
        main_group_id,
        main_request_id,
        trigger,
        reply_text,
    ) else {
        return Ok(());
    };

    let url = format!("{base_url}/api/main-project/context/feedback");
    let response = state
        .http_client
        .post(&url)
        .header(FB2_CONTEXT_HEADER, token.as_str())
        .json(&payload)
        .timeout(Duration::from_secs(timeout_secs()))
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        warn!(
            status = status.as_u16(),
            body = %truncate_chars(&body, 240),
            "fb2 generated-answer feedback callback returned non-success HTTP status"
        );
        return Ok(());
    }

    let parsed: Value = serde_json::from_str(&body).unwrap_or_else(|_| json!({}));
    if parsed["success"].as_bool() == Some(true) {
        info!(
            main_request_id,
            trigger,
            context_audit_id = payload["context_audit_id"].as_str().unwrap_or("unknown"),
            cited_source_count = payload["cited_source_count"].as_u64().unwrap_or(0),
            "fb2 generated-answer feedback callback recorded"
        );
    } else {
        warn!(
            body = %truncate_chars(&body, 240),
            "fb2 generated-answer feedback callback returned unsuccessful payload"
        );
    }

    if let Err(error) = post_opinion_adoption_if_needed(
        state,
        external_user_id.as_deref(),
        main_request_id,
        trigger,
        &payload,
        tool_results,
        reply_text,
        &base_url,
        &token,
    )
    .await
    {
        warn!(
            main_request_id,
            trigger,
            error = %error,
            "fb2 opinion adoption callback failed"
        );
    }

    Ok(())
}

async fn post_opinion_adoption_if_needed(
    state: &Arc<AppState>,
    external_user_id: Option<&str>,
    main_request_id: &str,
    trigger: &str,
    feedback_payload: &Value,
    tool_results: Option<&Value>,
    reply_text: &str,
    base_url: &str,
    token: &str,
) -> anyhow::Result<()> {
    let opinion_memory_ids = mentioned_opinion_memory_ids(tool_results, reply_text);
    if opinion_memory_ids.is_empty() {
        return Ok(());
    }

    let Some(context_audit_id) = feedback_payload
        .get("context_audit_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let Some(group_id) = feedback_payload
        .get("group_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };

    let cited_sources = opinion_memory_ids
        .iter()
        .map(|id| {
            json!({
                "kind": "group_opinion_memory",
                "id": id,
                "label": "fb2 群观点记忆"
            })
        })
        .collect::<Vec<_>>();
    let mut arguments = json!({
        "idempotency_key": format!("{main_request_id}:opinion_adoption:v1"),
        "context_audit_id": context_audit_id,
        "main_request_id": main_request_id,
        "group_id": group_id,
        "answer_intent": trigger,
        "opinion_memory_ids": opinion_memory_ids,
        "cited_sources": cited_sources,
        "adoption_note": truncate_chars(
            "auto_record_opinion_adoption; reply explicitly mentioned opinion memory source id",
            FEEDBACK_NOTE_MAX_CHARS
        )
    });
    if let Some(external_user_id) = external_user_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        arguments["external_user_id"] = Value::String(external_user_id.to_string());
    }

    let request_id = format!("{main_request_id}:record_opinion_adoption");
    let payload = json!({
        "request_id": request_id,
        "tool_name": "record_opinion_adoption",
        "group_id": group_id,
        "external_user_id": external_user_id,
        "context_audit_id": context_audit_id,
        "arguments": arguments,
        "reason": "main_project_generated_answer_used_fb2_opinion_memory"
    });
    let url = format!("{base_url}/api/main-project/tools/execute");
    let mut request = state
        .http_client
        .post(&url)
        .header(FB2_CONTEXT_HEADER, token)
        .json(&payload)
        .timeout(Duration::from_secs(timeout_secs()));
    for (header, value) in fb2_request_context_headers(external_user_id, false) {
        request = request.header(header, value);
    }

    let response = request.send().await?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        warn!(
            status = status.as_u16(),
            body = %truncate_chars(&body, 240),
            "fb2 opinion adoption callback returned non-success HTTP status"
        );
        return Ok(());
    }

    let parsed: Value = serde_json::from_str(&body).unwrap_or_else(|_| json!({}));
    if parsed["success"].as_bool() == Some(true) {
        info!(
            main_request_id,
            trigger,
            context_audit_id,
            adopted_count = parsed["source_ids"]
                .as_array()
                .map(|items| items.len())
                .unwrap_or(0),
            "fb2 opinion adoption callback recorded"
        );
    } else {
        warn!(
            body = %truncate_chars(&body, 240),
            "fb2 opinion adoption callback returned unsuccessful payload"
        );
    }
    Ok(())
}

fn generated_answer_feedback_payload(
    context: &Value,
    external_user_id: Option<&str>,
    main_group_id: &str,
    main_request_id: &str,
    trigger: &str,
    reply_text: &str,
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
    let cited_sources = mentioned_citation_sources(context, reply_text);
    let cited_source_count = cited_sources.len();

    let mut payload = json!({
        "context_audit_id": context_audit_id,
        "main_request_id": main_request_id,
        "group_id": group_id,
        "cited_source_count": cited_source_count,
        "cited_sources": cited_sources,
        "missing_context": false,
        "wrong_context": false,
        "note": truncate_chars(
            &format!(
                "auto_generated_answer_feedback; trigger={trigger}; cited_sources={}",
                cited_source_count
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

fn mentioned_citation_sources(context: &Value, reply_text: &str) -> Vec<Value> {
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

fn mentioned_opinion_memory_ids(tool_results: Option<&Value>, reply_text: &str) -> Vec<String> {
    let Some(results) = tool_results
        .and_then(|value| value.get("results"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    let reply = reply_text.to_lowercase();
    let mut seen = HashSet::new();
    let mut out = Vec::new();

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

fn push_mentioned_opinion_memory_id(
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

fn citation_source_is_mentioned(source: &Value, lower_reply: &str) -> bool {
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

fn is_fb2_context_pack(context: &Value) -> bool {
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

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut out = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_mentions_sources_by_id_or_label() {
        let context = json!({
            "app_id": "fb2",
            "group": "official",
            "status": "ready",
            "source": "fb2:/api/main-project/context/pack",
            "context_audit_id": "audit-1",
            "citation_sources": [
                {"kind": "match", "id": "match-1234", "label": "中央海岸 vs 奥克兰FC"},
                {"kind": "order", "id": "order-5678", "label": "用户票据"}
            ]
        });

        let payload = generated_answer_feedback_payload(
            &context,
            Some("fb2-user-1"),
            "ext_fb2_official",
            "social_group_message:m1",
            "group_mention",
            "这场中央海岸 vs 奥克兰FC 风险偏高，可参考 match-1234。",
        )
        .expect("payload");

        assert_eq!(payload["context_audit_id"], "audit-1");
        assert_eq!(payload["group_id"], "official");
        assert_eq!(payload["external_user_id"], "fb2-user-1");
        assert_eq!(payload["cited_source_count"], 1);
        assert_eq!(payload["cited_sources"][0]["id"], "match-1234");
    }

    #[test]
    fn payload_requires_ready_fb2_context_pack_audit() {
        let context = json!({
            "app_id": "fb2",
            "status": "ready",
            "source": "fb2:/api/main-project/context/today-matches"
        });

        assert!(!is_fb2_context_pack(&context));
        assert!(generated_answer_feedback_payload(
            &context, None, "group", "request", "trigger", "reply"
        )
        .is_none());
    }

    #[test]
    fn opinion_memory_ids_require_grounded_tool_and_reply_reference() {
        let tool_results = json!({
            "results": [
                {
                    "tool_name": "opinion_memories",
                    "success": true,
                    "grounding": {"status": "grounded"},
                    "source_ids": ["opinion-memory-1"],
                    "data": {
                        "memories": [
                            {"id": "opinion-memory-2", "source_message_id": "group-msg-9999"},
                            {"id": "unmentioned-memory", "source_message_id": "group-msg-0000"}
                        ]
                    }
                },
                {
                    "tool_name": "opinion_memories",
                    "success": true,
                    "grounding": {"status": "ungrounded"},
                    "source_ids": ["unsafe-memory"]
                }
            ]
        });

        let ids = mentioned_opinion_memory_ids(
            Some(&tool_results),
            "AI 采纳了 opinion-memory-1，也参考了 group-msg-9999 的历史观点。",
        );

        assert_eq!(ids, vec!["opinion-memory-1", "opinion-memory-2"]);
    }

    #[test]
    fn opinion_memory_ids_ignore_unmentioned_sources() {
        let tool_results = json!({
            "results": [{
                "tool_name": "opinion_memories",
                "success": true,
                "grounding": {"status": "grounded"},
                "source_ids": ["opinion-memory-1"],
                "data": {"memories": [{"id": "opinion-memory-2", "source_message_id": "group-msg-9999"}]}
            }]
        });

        let ids =
            mentioned_opinion_memory_ids(Some(&tool_results), "这里只总结群观点，不引用具体来源。");

        assert!(ids.is_empty());
    }
}
