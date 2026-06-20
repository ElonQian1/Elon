//! External business context pulled from child apps for chat AI.

use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tracing::{info, warn};

use crate::{
    external_app_context_budget::budgeted_context,
    external_app_context_contract::{fb2_match_context, fb2_pack_context},
    external_app_registry::{external_group_by_group_id, public_external_app_config},
    types::AppState,
};

const FB2_APP_ID: &str = "fb2";
const FB2_CONTEXT_HEADER: &str = "X-FB2-AI-CENTER-TOKEN";
const DEFAULT_MATCH_LIMIT: u32 = 30;
const DEFAULT_DISCUSSION_LIMIT: u32 = 80;
const DEFAULT_ORDER_LIMIT: u32 = 20;
const DEFAULT_TIMEOUT_SECS: u64 = 6;

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
            log_context_fetch(app.id, group_id, group.external_group_id, user_id, &context);
            Some(budgeted_context(context))
        }
        _ => None,
    }
}

fn fb2_base_url() -> Option<String> {
    first_non_empty_env(&[
        "ELON_EXTERNAL_APP_FB2_BASE_URL",
        "ELON_FB2_BASE_URL",
        "FB2_BASE_URL",
    ])
    .map(|value| value.trim_end_matches('/').to_string())
}

fn fb2_context_token() -> Option<String> {
    first_non_empty_env(&[
        "ELON_EXTERNAL_APP_FB2_CONTEXT_TOKEN",
        "ELON_FB2_AI_CENTER_TOKEN",
        "ELON_EXTERNAL_APP_FB2_TOKEN",
        "FB2_MAIN_PROJECT_SHARED_SECRET",
    ])
}

fn first_non_empty_env(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| std::env::var(name).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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

    if context_pack_enabled() {
        let pack = fetch_fb2_context_pack(
            state,
            app_id,
            user_id,
            external_group_id,
            topic_hint,
            &base_url,
            &token,
        )
        .await;
        if pack["status"].as_str() == Some("ready") {
            return pack;
        }
        warn!(
            "fb2 context pack unavailable, falling back to today-matches: {:?}",
            pack.get("error").or_else(|| pack.get("status"))
        );
    }

    fetch_fb2_match_context(
        state,
        app_id,
        external_group_id,
        topic_hint,
        &base_url,
        &token,
    )
    .await
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
    if let Some(account) = external_account.as_ref() {
        query.push((
            "external_user_id".to_string(),
            account.external_user_id.clone(),
        ));
    }
    if let Some(topic) = topic_hint.and_then(clean_query_value) {
        query.push(("topic_hint".to_string(), topic.to_string()));
    }
    if let Some(lottery_type) = infer_lottery_type(topic_hint) {
        query.push(("lottery_type".to_string(), lottery_type));
    }

    let mut request = state
        .http_client
        .get(&url)
        .header(FB2_CONTEXT_HEADER, token)
        .query(&query)
        .timeout(Duration::from_secs(timeout_secs()));
    if platform_order_summary_enabled() {
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
    state: &Arc<AppState>,
    app_id: &str,
    external_group_id: &str,
    topic_hint: Option<&str>,
    base_url: &str,
    token: &str,
) -> Value {
    let limit = match_limit();
    let url = format!("{base_url}/api/main-project/context/today-matches");
    let mut request = state
        .http_client
        .get(&url)
        .header(FB2_CONTEXT_HEADER, token)
        .query(&[("limit", limit.to_string())])
        .timeout(Duration::from_secs(timeout_secs()));
    if let Some(lottery_type) = infer_lottery_type(topic_hint) {
        request = request.query(&[("lottery_type", lottery_type)]);
    }

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

async fn fb2_pack_response_to_context(
    app_id: &str,
    external_group_id: &str,
    response: reqwest::Response,
) -> Value {
    let status = response.status();
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "error": compact_error(&error.to_string())
            });
        }
    };
    if !status.is_success() {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "status": "unavailable",
            "status_code": status.as_u16(),
            "error": truncate_chars(&text, 300)
        });
    }

    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "error": format!("fb2 context pack JSON 解析失败：{}", compact_error(&error.to_string()))
            });
        }
    };
    if parsed["success"].as_bool() != Some(true) {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "status": "unavailable",
            "error": parsed["error"].as_str().unwrap_or("fb2 返回失败状态")
        });
    }

    let data = parsed.get("data").cloned().unwrap_or_else(|| json!({}));
    fb2_pack_context(app_id, external_group_id, data)
}

async fn fb2_response_to_context(
    app_id: &str,
    external_group_id: &str,
    response: reqwest::Response,
) -> Value {
    let status = response.status();
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "error": compact_error(&error.to_string())
            });
        }
    };
    if !status.is_success() {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "status": "unavailable",
            "status_code": status.as_u16(),
            "error": truncate_chars(&text, 300)
        });
    }

    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "error": format!("fb2 JSON 解析失败：{}", compact_error(&error.to_string()))
            });
        }
    };
    if parsed["success"].as_bool() != Some(true) {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "status": "unavailable",
            "error": parsed["error"].as_str().unwrap_or("fb2 返回失败状态")
        });
    }

    let data = parsed.get("data").cloned().unwrap_or_else(|| json!({}));
    fb2_match_context(app_id, external_group_id, data)
}

fn infer_lottery_type(topic_hint: Option<&str>) -> Option<String> {
    let text = topic_hint?.to_ascii_lowercase();
    if text.contains("北单") || text.contains("beidan") {
        Some("BeiDan".to_string())
    } else if text.contains("竞彩") || text.contains("jingcai") {
        Some("JingCai".to_string())
    } else {
        None
    }
}

fn match_limit() -> u32 {
    std::env::var("ELON_EXTERNAL_APP_FB2_MATCH_CONTEXT_LIMIT")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(DEFAULT_MATCH_LIMIT)
        .clamp(1, 100)
}

fn discussion_limit() -> u32 {
    std::env::var("ELON_EXTERNAL_APP_FB2_DISCUSSION_CONTEXT_LIMIT")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(DEFAULT_DISCUSSION_LIMIT)
        .clamp(1, 200)
}

fn order_limit() -> u32 {
    std::env::var("ELON_EXTERNAL_APP_FB2_ORDER_CONTEXT_LIMIT")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(DEFAULT_ORDER_LIMIT)
        .clamp(1, 100)
}

fn context_pack_enabled() -> bool {
    env_flag("ELON_EXTERNAL_APP_FB2_CONTEXT_PACK_ENABLED", true)
}

fn platform_order_summary_enabled() -> bool {
    env_flag("ELON_EXTERNAL_APP_FB2_PLATFORM_ORDER_CONTEXT", false)
}

fn env_flag(name: &str, default_value: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default_value)
}

fn timeout_secs() -> u64 {
    std::env::var("ELON_EXTERNAL_APP_FB2_CONTEXT_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .clamp(2, 30)
}

fn compact_error(error: &str) -> String {
    truncate_chars(error, 220)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut out = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
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
    context: &Value,
) {
    let status = context["status"].as_str().unwrap_or("unknown");
    let source = context["source"].as_str().unwrap_or("unknown");
    let has_external_user_id = context["user_orders"]
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
        has_external_user_id,
        context_chars,
        "external app context fetched"
    );
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
    fn truncates_long_errors() {
        let text = "a".repeat(300);
        let truncated = truncate_chars(&text, 20);
        assert_eq!(truncated, "aaaaaaaaaaaaaaaaaaaa...");
    }

    #[test]
    fn env_flag_defaults_when_missing() {
        assert!(env_flag("__ELON_TEST_MISSING_FLAG__", true));
        assert!(!env_flag("__ELON_TEST_MISSING_FLAG__", false));
    }
}
