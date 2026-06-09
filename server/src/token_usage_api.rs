//! Token 用量统计 API
//!
//! - GET  /api/user/:user_id/usage/stats?days=30  —— 查询聚合统计（服务器权威数据）
//! - POST /api/user/:user_id/usage/report         —— APK 直连时客户端上报（Mode 2）
//!
//! 还提供两个 `pub(crate)` 辅助函数，供其他模块在记录用量时调用：
//! - `record_api_usage`           服务端调用 OpenAI API 后调用
//! - `record_codex_usage_from_stdout_with_key`  Codex CLI 运行完成后调用

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
    cli_usage::{parse_cli_usage, usage_from_value, CliTokenUsage},
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
    let total = body.total_tokens.unwrap_or(input + output).max(0);

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
        idempotency_key: None,
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
    let Some(usage) = usage_from_value(response) else {
        return;
    };
    record_trusted_usage(
        store,
        user_id,
        feature,
        "server_api_key",
        Some(model),
        &usage,
    );
}

/// 记录服务器可信 token 用量，并同步执行预存余额扣费。
///
/// `client_reported` 不走这里；只有服务器 API key、服务器/PC CLI、节点 LLM 等
/// 服务端可验证来源才允许扣余额。
pub(crate) fn record_trusted_usage(
    store: &Store,
    user_id: &str,
    feature: &str,
    usage_mode: &str,
    model: Option<&str>,
    usage: &CliTokenUsage,
) {
    record_trusted_usage_with_key(store, user_id, feature, usage_mode, model, usage, None);
}

pub(crate) fn record_trusted_usage_with_key(
    store: &Store,
    user_id: &str,
    feature: &str,
    usage_mode: &str,
    model: Option<&str>,
    usage: &CliTokenUsage,
    idempotency_key: Option<&str>,
) {
    let Some(usage) = usage.clone().normalized() else {
        return;
    };
    let model = model.or(usage.model.as_deref());
    let record = TokenUsageRecord {
        user_id,
        feature,
        usage_mode,
        model,
        input_tokens: usage.input_tokens,
        cached_input_tokens: usage.cached_input_tokens,
        output_tokens: usage.output_tokens,
        reasoning_tokens: usage.reasoning_tokens,
        total_tokens: usage.total_tokens,
        idempotency_key,
    };
    match crate::billing::account_trusted_usage(store, &record) {
        Ok(result) => {
            tracing::debug!(
                user_id,
                feature,
                usage_mode,
                token_event_id = %result.token_usage_event_id,
                billing_event_id = ?result.billing_event_id,
                cost_rmb_fen = result.cost_rmb_fen,
                balance_after_fen = ?result.balance_after_fen,
                accounting_status = %result.accounting_status,
                idempotency_key = ?result.idempotency_key,
                deduplicated = result.deduplicated,
                "record_trusted_usage accounted"
            );
        }
        Err(e) => {
            tracing::error!(
                user_id,
                feature,
                usage_mode,
                "record_trusted_usage 记账失败: {}",
                e
            );
        }
    }
}

// ── 内部辅助：Codex CLI stdout 中解析 token 用量 ─────────────────────────────

/// Codex CLI 运行完成后，扫描 stdout 中的 JSON 事件行，汇总 token 用量并写入数据库。
///
/// Codex 输出两类事件（字段名可能是 camelCase 或 snake_case，两者都尝试）：
/// - `{"type":"token_count", "inputTokens":N, ...}` 或 `{"type":"token_count","input_tokens":N,...}`
/// - `{"type":"turn.completed", "usage": {"input_tokens":N, ...}}`
pub(crate) fn record_codex_usage_from_stdout_with_key(
    store: &Store,
    user_id: &str,
    feature: &str,
    model: Option<&str>,
    stdout: &str,
    idempotency_key: Option<&str>,
) {
    let Some(usage) = parse_cli_usage(stdout) else {
        tracing::debug!(
            user_id,
            feature,
            "record_codex_usage_from_stdout: 未找到 token 用量事件（stdout {} 字节）",
            stdout.len()
        );
        return;
    };
    tracing::info!(
        user_id,
        feature,
        input = usage.input_tokens,
        output = usage.output_tokens,
        total = usage.total_tokens,
        "记录 Codex CLI token 用量"
    );
    record_trusted_usage_with_key(
        store,
        user_id,
        feature,
        "server_codex_cli",
        model,
        &usage,
        idempotency_key,
    );
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
