//! Token 用量统计 API
//!
//! - GET  /api/user/:user_id/usage/stats?days=30  —— 查询聚合统计（服务器权威数据）
//! - POST /api/user/:user_id/usage/report         —— APK 直连时客户端上报（Mode 2）
//!
//! 还提供两个 `pub(crate)` 辅助函数，供其他模块在记录用量时调用：
//! - `record_api_usage`           服务端调用 OpenAI API 后调用
//! - `record_codex_usage_from_stdout`  Codex CLI 运行完成后调用

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    store::{Store, TokenUsageRecord},
    types::AppState,
};

// ── GET /api/user/:user_id/usage/stats ───────────────────────────────────────

#[derive(Deserialize)]
pub struct StatsQuery {
    #[serde(default = "default_days")]
    pub days: i64,
}
fn default_days() -> i64 {
    30
}

pub async fn get_usage_stats(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Query(q): Query<StatsQuery>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    if user.id != user_id {
        return json_error(StatusCode::FORBIDDEN, "无权查看此用户的用量数据");
    }

    // 合法区间：1 ～ 365 天
    let days = q.days.clamp(1, 365);

    match state.store.get_usage_stats(&user_id, days) {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => {
            tracing::warn!("get_usage_stats error for {}: {}", user_id, e);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "查询失败，请稍后重试")
        }
    }
}

// ── POST /api/user/:user_id/usage/report ─────────────────────────────────────

#[derive(Deserialize)]
pub struct ClientUsageReport {
    pub feature: String,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub cached_input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub reasoning_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
}

pub async fn report_client_usage(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(body): Json<ClientUsageReport>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(u) => u,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "未登录"),
    };
    if user.id != user_id {
        return json_error(StatusCode::FORBIDDEN, "无权为此用户上报");
    }

    let input = body.input_tokens.unwrap_or(0).max(0);
    let cached = body.cached_input_tokens.unwrap_or(0).max(0);
    let output = body.output_tokens.unwrap_or(0).max(0);
    let reasoning = body.reasoning_tokens.unwrap_or(0).max(0);
    let total = body
        .total_tokens
        .unwrap_or(input + output)
        .max(0);

    let feature = sanitize_label(&body.feature, "unknown");

    let record = TokenUsageRecord {
        user_id: &user_id,
        feature: &feature,
        usage_mode: "client_reported",
        model: body.model.as_deref(),
        input_tokens: input,
        cached_input_tokens: cached,
        output_tokens: output,
        reasoning_tokens: reasoning,
        total_tokens: total,
    };

    if let Err(e) = state.store.record_token_usage(&record) {
        tracing::warn!("report_client_usage store error: {}", e);
    }

    Json(json!({ "ok": true })).into_response()
}

// ── 内部辅助：服务器 API Key 调用后记录 ──────────────────────────────────────

/// 服务端通过 OpenAI API 调用完成后，从响应中提取 usage 并写入数据库。
///
/// `response` 是 OpenAI chat completions 标准响应（含 `usage` 字段）。
pub(crate) fn record_api_usage(
    store: &Store,
    response: &Value,
    user_id: &str,
    feature: &str,
    model: &str,
) {
    let usage = &response["usage"];
    if usage.is_null() {
        return;
    }

    let input = usage["prompt_tokens"].as_i64().unwrap_or(0);
    let cached = usage["prompt_tokens_details"]["cached_tokens"]
        .as_i64()
        .unwrap_or(0);
    let output = usage["completion_tokens"].as_i64().unwrap_or(0);
    let reasoning = usage["completion_tokens_details"]["reasoning_tokens"]
        .as_i64()
        .unwrap_or(0);
    let total = usage["total_tokens"].as_i64().unwrap_or(input + output);

    if total == 0 {
        return;
    }

    let record = TokenUsageRecord {
        user_id,
        feature,
        usage_mode: "server_api_key",
        model: Some(model),
        input_tokens: input,
        cached_input_tokens: cached,
        output_tokens: output,
        reasoning_tokens: reasoning,
        total_tokens: total,
    };
    if let Err(e) = store.record_token_usage(&record) {
        tracing::debug!("record_api_usage: {}", e);
    }
}

// ── 内部辅助：Codex CLI stdout 中解析 token 用量 ─────────────────────────────

/// Codex CLI 运行完成后，扫描 stdout 中的 JSON 事件行，汇总 token 用量并写入数据库。
///
/// Codex 输出两类事件：
/// - `{"type":"token_count", "inputTokens":N, "outputTokens":N, ...}`
/// - `{"type":"turn.completed", "usage": {...}}`
pub(crate) fn record_codex_usage_from_stdout(
    store: &Store,
    user_id: &str,
    feature: &str,
    model: Option<&str>,
    stdout: &str,
) {
    let mut input: i64 = 0;
    let mut cached: i64 = 0;
    let mut output: i64 = 0;
    let mut reasoning: i64 = 0;
    let mut found = false;

    for line in stdout.lines() {
        let line = line.trim();
        if !line.contains("token") {
            continue;
        }
        let Ok(v): Result<Value, _> = serde_json::from_str(line) else {
            continue;
        };
        let ty = v["type"].as_str().unwrap_or("");

        match ty {
            "token_count" => {
                input += v["inputTokens"].as_i64().unwrap_or(0);
                cached += v["cachedInputTokens"].as_i64().unwrap_or(0);
                output += v["outputTokens"].as_i64().unwrap_or(0);
                reasoning += v["reasoningTokens"].as_i64().unwrap_or(0);
                found = true;
            }
            "turn.completed" => {
                let u = &v["usage"];
                if !u.is_null() {
                    input += u["input_tokens"].as_i64().unwrap_or(0);
                    cached += u["cached_input_tokens"].as_i64().unwrap_or(0);
                    output += u["output_tokens"].as_i64().unwrap_or(0);
                    reasoning += u["reasoning_output_tokens"].as_i64().unwrap_or(0);
                    found = true;
                }
            }
            _ => {}
        }
    }

    if !found || (input + output) == 0 {
        return;
    }

    let total = input + output;
    let record = TokenUsageRecord {
        user_id,
        feature,
        usage_mode: "server_codex_cli",
        model,
        input_tokens: input,
        cached_input_tokens: cached,
        output_tokens: output,
        reasoning_tokens: reasoning,
        total_tokens: total,
    };
    if let Err(e) = store.record_token_usage(&record) {
        tracing::debug!("record_codex_usage_from_stdout: {}", e);
    }
}

// ── 工具函数 ──────────────────────────────────────────────────────────────────

/// 清理标签字符串：只保留字母、数字、下划线和连字符，最长 64 字符。
fn sanitize_label(s: &str, fallback: &str) -> String {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(64)
        .collect();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}
