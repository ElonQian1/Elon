//! server/src/external_app_context.rs
//! External business context pulled from child apps for chat AI.

use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tracing::{info, warn};

use crate::{
    external_app_context_budget::budgeted_context,
    external_app_context_config::{
        context_pack_enabled, discussion_limit, fb2_base_url, fb2_context_token,
        fb2_request_context_headers, infer_lottery_type, match_limit, order_limit,
        platform_order_summary_enabled, platform_order_summary_requested, timeout_secs, FB2_APP_ID,
        FB2_CONTEXT_HEADER,
    },
    external_app_context_quality::context_quality,
    external_app_context_readiness::live_context_readiness,
    external_app_context_response::{
        compact_error, fb2_pack_response_to_context, fb2_response_to_context,
    },
    external_app_http_client::fb2_direct_client,
    external_app_registry::{external_group_by_group_id, public_external_app_config},
    types::AppState,
};

pub(crate) async fn group_context_for_chat(
    state: &Arc<AppState>,
    user_id: &str,
    group_id: &str,
    topic_hint: Option<&str>,
) -> Option<Value> {
    let (app, group) = external_group_by_group_id(group_id)?;
    match app.id {
        FB2_APP_ID => {
            let context = fetch_fb2_business_context(
                state,
                app.id,
                user_id,
                group.external_group_id,
                topic_hint,
            )
            .await;
            log_context_fetch(
                app.id,
                group_id,
                group.external_group_id,
                user_id,
                topic_hint,
                &context,
            );
            Some(budgeted_context(context))
        }
        _ => None,
    }
}

async fn fetch_fb2_business_context(
    state: &Arc<AppState>,
    app_id: &str,
    user_id: &str,
    external_group_id: &str,
    topic_hint: Option<&str>,
) -> Value {
    let base_url = fb2_base_url();
    let token = fb2_context_token();
    let app_config = external_group_by_group_id(&format!("ext_{app_id}_{external_group_id}"))
        .map(|(app, _)| public_external_app_config(app))
        .unwrap_or_else(|| json!({ "id": app_id }));

    let Some(base_url) = base_url else {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "app": app_config,
            "status": "not_configured",
            "required_env": ["ELON_EXTERNAL_APP_FB2_BASE_URL", "ELON_FB2_BASE_URL", "FB2_BASE_URL"],
            "message": "未配置 fb2 上下文服务地址，群聊 AI 将只使用本群聊天记录。"
        });
    };
    let Some(token) = token else {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "app": app_config,
            "status": "not_configured",
            "required_env": ["ELON_EXTERNAL_APP_FB2_CONTEXT_TOKEN", "ELON_FB2_AI_CENTER_TOKEN", "ELON_EXTERNAL_APP_FB2_TOKEN", "FB2_MAIN_PROJECT_SHARED_SECRET"],
            "message": "未配置 fb2 上下文服务令牌，群聊 AI 将只使用本群聊天记录。"
        });
    };

    // readiness 是主项目调用 fb2 前的轻量预检；失败不直接中断，避免临时探针异常放大为聊天不可用。
    let preflight_readiness = live_context_readiness(app_id).await;
    log_readiness_preflight(app_id, external_group_id, user_id, &preflight_readiness);

    if context_pack_enabled() {
        let mut pack = fetch_fb2_context_pack(
            state,
            app_id,
            user_id,
            external_group_id,
            topic_hint,
            &base_url,
            &token,
        )
        .await;
        annotate_context_with_readiness(&mut pack, &preflight_readiness);
        if pack["status"].as_str() == Some("ready") {
            return pack;
        }
        warn!(
            "fb2 context pack unavailable, falling back to today-matches: {:?}",
            pack.get("error").or_else(|| pack.get("status"))
        );
    }

    let mut fallback =
        fetch_fb2_match_context(app_id, external_group_id, topic_hint, &base_url, &token).await;
    annotate_context_with_readiness(&mut fallback, &preflight_readiness);
    fallback
}

async fn fetch_fb2_context_pack(
    state: &Arc<AppState>,
    app_id: &str,
    user_id: &str,
    external_group_id: &str,
    topic_hint: Option<&str>,
    base_url: &str,
    token: &str,
) -> Value {
    let url = format!("{base_url}/api/main-project/context/pack");
    let external_account = state
        .store
        .external_account_for_main_user(app_id, user_id)
        .map_err(|error| {
            warn!("fb2 external account lookup failed: {}", error);
            error
        })
        .ok()
        .flatten();

    let mut query = vec![
        ("group_id".to_string(), external_group_id.to_string()),
        ("limit".to_string(), match_limit().to_string()),
        (
            "discussion_limit".to_string(),
            discussion_limit().to_string(),
        ),
        ("order_limit".to_string(), order_limit().to_string()),
    ];
    let external_user_id = external_account
        .as_ref()
        .map(|account| account.external_user_id.trim())
        .filter(|value| !value.is_empty());
    if let Some(external_user_id) = external_user_id {
        query.push(("external_user_id".to_string(), external_user_id.to_string()));
    }
    if let Some(topic) = topic_hint.and_then(clean_query_value) {
        query.push(("topic_hint".to_string(), topic.to_string()));
    }
    if let Some(lottery_type) = infer_lottery_type(topic_hint) {
        query.push(("lottery_type".to_string(), lottery_type));
    }

    let include_platform_orders =
        platform_order_summary_enabled() && platform_order_summary_requested(topic_hint);
    let mut request = fb2_direct_client()
        .get(&url)
        .header(FB2_CONTEXT_HEADER, token)
        .query(&query)
        .timeout(Duration::from_secs(timeout_secs()));
    for (header, value) in fb2_request_context_headers(external_user_id, include_platform_orders) {
        request = request.header(header, value);
    }
    if include_platform_orders {
        request = request.query(&[("include_platform_orders", "true")]);
    }

    match request.send().await {
        Ok(response) => fb2_pack_response_to_context(app_id, external_group_id, response).await,
        Err(error) => {
            warn!("fb2 context pack request failed: {}", error);
            json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "source": url,
                "error": compact_error(&error.to_string()),
                "message": "fb2 业务上下文包暂时不可用，主项目将回退到今日比赛上下文。"
            })
        }
    }
}

async fn fetch_fb2_match_context(
    app_id: &str,
    external_group_id: &str,
    topic_hint: Option<&str>,
    base_url: &str,
    token: &str,
) -> Value {
    let limit = match_limit();
    let url = format!("{base_url}/api/main-project/context/today-matches");
    let mut query = vec![
        ("group_id".to_string(), external_group_id.to_string()),
        ("limit".to_string(), limit.to_string()),
    ];
    if let Some(topic) = topic_hint.and_then(clean_query_value) {
        query.push(("topic_hint".to_string(), topic.to_string()));
    }
    if let Some(lottery_type) = infer_lottery_type(topic_hint) {
        query.push(("lottery_type".to_string(), lottery_type));
    }
    let request = fb2_direct_client()
        .get(&url)
        .header(FB2_CONTEXT_HEADER, token)
        .query(&query)
        .timeout(Duration::from_secs(timeout_secs()));

    match request.send().await {
        Ok(response) => fb2_response_to_context(app_id, external_group_id, response).await,
        Err(error) => {
            warn!("fb2 match context request failed: {}", error);
            json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "source": url,
                "error": compact_error(&error.to_string()),
                "message": "fb2 今日比赛上下文暂时不可用，回答只能基于群聊内容。"
            })
        }
    }
}

fn clean_query_value(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn log_context_fetch(
    app_id: &str,
    group_id: &str,
    external_group_id: &str,
    user_id: &str,
    topic_hint: Option<&str>,
    context: &Value,
) {
    let status = context["status"].as_str().unwrap_or("unknown");
    let source = context["source"].as_str().unwrap_or("unknown");
    let topic_hint_present = topic_hint.and_then(clean_query_value).is_some();
    let fallback_used = context_fallback_used(context);
    let context_pack_version = context["context_pack_version"]
        .as_str()
        .unwrap_or("unknown");
    let context_audit_id = context["context_audit_id"].as_str().unwrap_or("unknown");
    let answer_policy_schema = context_answer_policy_schema(context);
    let context_quality_warning_count = context_quality_warning_count(context);
    let tool_readiness_status = context_tool_readiness_status(context);
    let user_order_context_present = context["user_orders"]
        .as_array()
        .map(|orders| !orders.is_empty())
        .unwrap_or(false);
    let context_chars = serde_json::to_string(context)
        .map(|text| text.chars().count())
        .unwrap_or(0);
    info!(
        app_id,
        group_id,
        external_group_id,
        user_id,
        status,
        source,
        topic_hint_present,
        fallback_used,
        user_order_context_present,
        context_pack_version,
        context_audit_id,
        answer_policy_schema,
        context_quality_warning_count,
        tool_readiness_status,
        context_chars,
        "external app context fetched"
    );
}

fn annotate_context_with_readiness(context: &mut Value, readiness: &Value) {
    context["preflight_readiness"] = readiness.clone();
    let expects_context_pack = context["source"]
        .as_str()
        .map(|source| source.contains("/context/pack"))
        .unwrap_or(false);
    if context["status"].as_str() == Some("ready") {
        context["context_quality"] = context_quality(context, expects_context_pack);
    }
}

fn log_readiness_preflight(
    app_id: &str,
    external_group_id: &str,
    user_id: &str,
    readiness: &Value,
) {
    let status = readiness["status"].as_str().unwrap_or("unknown");
    let warning_count = readiness["warnings"].as_array().map(Vec::len).unwrap_or(0);
    if status != "ready" {
        warn!(
            app_id,
            external_group_id,
            user_id,
            status,
            warning_count,
            "external app context readiness is not ready"
        );
    }
}

fn context_fallback_used(context: &Value) -> bool {
    context["source"]
        .as_str()
        .map(|source| source.contains("/today-matches"))
        .unwrap_or(false)
}

fn context_quality_warning_count(context: &Value) -> usize {
    context["context_quality"]["warnings"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0)
}

fn context_tool_readiness_status(context: &Value) -> &str {
    context["context_quality"]["tool_readiness"]["status"]
        .as_str()
        .unwrap_or("unknown")
}

fn context_answer_policy_schema(context: &Value) -> &str {
    context["answer_policy"]["schema"]
        .as_str()
        .unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_fb2_lottery_type_from_topic_hint() {
        assert_eq!(
            infer_lottery_type(Some("今天竞彩怎么看")),
            Some("JingCai".into())
        );
        assert_eq!(infer_lottery_type(Some("北单赛事")), Some("BeiDan".into()));
        assert_eq!(infer_lottery_type(Some("足球比赛")), None);
    }

    #[test]
    fn context_log_helpers_extract_observability_fields() {
        let context = json!({
            "source": "fb2:/api/main-project/context/today-matches",
            "answer_policy": {"schema": "fb2.answer_policy.v1"},
            "context_quality": {
                "warnings": ["missing_context_pack", "missing_tool_contract"],
                "tool_readiness": {"status": "partial"}
            }
        });

        assert!(context_fallback_used(&context));
        assert_eq!(context_quality_warning_count(&context), 2);
        assert_eq!(context_tool_readiness_status(&context), "partial");
        assert_eq!(
            context_answer_policy_schema(&context),
            "fb2.answer_policy.v1"
        );
    }

    #[test]
    fn readiness_annotation_updates_context_quality() {
        let mut context = json!({
            "app_id": "fb2",
            "group": "official",
            "status": "ready",
            "source": "fb2:/api/main-project/context/pack",
            "generated_at": "2026-06-22T11:30:00+08:00",
            "context_pack_version": "fb2-chat-pack-v1",
            "context_pack": "<fb2_context_pack>ok</fb2_context_pack>",
            "matches": [{"id": "m1"}],
            "tool_contract": {"tools": [{"name": "get_match_detail"}]},
            "metrics": {}
        });
        let readiness = json!({
            "schema": "external_app.live_context_readiness.v1",
            "status": "blocked",
            "warnings": ["fb2_readiness_blocked"]
        });

        annotate_context_with_readiness(&mut context, &readiness);

        assert_eq!(context["preflight_readiness"]["status"], "blocked");
        assert!(context["context_quality"]["warnings"]
            .as_array()
            .unwrap()
            .contains(&json!("fb2_readiness_blocked")));
    }
}
