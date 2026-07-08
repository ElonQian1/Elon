//! Feedback callbacks from generated main-project answers to child app context services.

use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tracing::{info, warn};

use crate::{
    external_app_context_config::{
        fb2_base_url, fb2_context_token, fb2_request_context_headers, timeout_secs, FB2_APP_ID,
        FB2_CONTEXT_HEADER,
    },
    external_app_context_source_validation::validate_reply_sources,
    external_app_http_client::{build_fb2_direct_client, fb2_direct_client},
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
    extra_citation_sources: Vec<Value>,
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
            &extra_citation_sources,
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
    extra_citation_sources: &[Value],
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
        tool_results,
        extra_citation_sources,
    ) else {
        return Ok(());
    };

    let url = format!("{base_url}/api/main-project/context/feedback");
    let response = send_feedback_request(
        &url,
        token.as_str(),
        external_user_id.as_deref(),
        feedback_context_needs_platform_order_scope(context),
        &payload,
    )
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
            wrong_context = payload["wrong_context"].as_bool().unwrap_or(false),
            "fb2 generated-answer feedback callback recorded"
        );
    } else {
        warn!(
            body = %truncate_chars(&body, 240),
            "fb2 generated-answer feedback callback returned unsuccessful payload"
        );
    }

    if let Err(error) = post_opinion_adoption_if_needed(
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
    external_user_id: Option<&str>,
    main_request_id: &str,
    trigger: &str,
    feedback_payload: &Value,
    tool_results: Option<&Value>,
    reply_text: &str,
    base_url: &str,
    token: &str,
) -> anyhow::Result<()> {
    let opinion_memory_ids =
        mentioned_opinion_memory_ids(feedback_payload, tool_results, reply_text);
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
    let mut request = fb2_direct_client()
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

async fn send_feedback_request(
    url: &str,
    token: &str,
    external_user_id: Option<&str>,
    include_platform_order_summary: bool,
    payload: &Value,
) -> anyhow::Result<reqwest::Response> {
    send_feedback_request_with_client(
        fb2_direct_client(),
        url,
        token,
        external_user_id,
        include_platform_order_summary,
        payload,
    )
    .await
}

async fn send_feedback_request_with_client(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    external_user_id: Option<&str>,
    include_platform_order_summary: bool,
    payload: &Value,
) -> anyhow::Result<reqwest::Response> {
    let timeout = Duration::from_secs(timeout_secs().max(10));
    let mut request = client
        .post(url)
        .header(FB2_CONTEXT_HEADER, token)
        .json(payload)
        .timeout(timeout);
    for (header, value) in
        fb2_request_context_headers(external_user_id, include_platform_order_summary)
    {
        request = request.header(header, value);
    }

    match request.send().await {
        Ok(response) => Ok(response),
        Err(first_error) => {
            warn!(
                error = %first_error,
                "fb2 generated-answer feedback callback initial send failed; retrying with fresh client"
            );
            let client = build_fb2_direct_client()?;
            let mut retry = client
                .post(url)
                .header(FB2_CONTEXT_HEADER, token)
                .json(payload)
                .timeout(timeout);
            for (header, value) in
                fb2_request_context_headers(external_user_id, include_platform_order_summary)
            {
                retry = retry.header(header, value);
            }
            retry.send().await.map_err(|second_error| {
                anyhow::anyhow!(
                    "initial send failed: {}; retry send failed: {}",
                    first_error,
                    second_error
                )
            })
        }
    }
}

mod helpers;
use self::helpers::*;

#[cfg(test)]
#[path = "external_app_context_feedback_tests.rs"]
mod tests;
