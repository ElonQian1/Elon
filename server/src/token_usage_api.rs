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
    store::{ComputeMeterEvent, Store, TokenUsageAccountingResult, TokenUsageRecord},
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
    usage_mode: &str,
) {
    let Some(usage) = usage_from_value(response) else {
        return;
    };
    if usage_mode == "user_api_key_proxy" {
        let Some(usage) = usage.normalized() else {
            return;
        };
        let record = TokenUsageRecord {
            user_id,
            feature,
            usage_mode,
            model: Some(model),
            input_tokens: usage.input_tokens,
            cached_input_tokens: usage.cached_input_tokens,
            output_tokens: usage.output_tokens,
            reasoning_tokens: usage.reasoning_tokens,
            total_tokens: usage.total_tokens,
            idempotency_key: None,
        };
        if let Err(e) = store.record_token_usage(&record) {
            tracing::warn!(
                user_id,
                feature,
                usage_mode,
                "BYOK token 用量记录失败: {}",
                e
            );
        }
        return;
    }
    let _ = record_trusted_usage(store, user_id, feature, usage_mode, Some(model), &usage);
}

/// 记录服务器可信 token 用量，并同步执行预存余额扣费。
///
/// `client_reported` 和用户自带 Key 不走这里；只有服务器 API key、
/// 服务器/PC CLI、节点 LLM 等服务端可验证且应由平台余额承载的来源才扣余额。
pub(crate) fn record_trusted_usage(
    store: &Store,
    user_id: &str,
    feature: &str,
    usage_mode: &str,
    model: Option<&str>,
    usage: &CliTokenUsage,
) -> Option<crate::store::TokenUsageAccountingResult> {
    record_trusted_usage_with_key(store, user_id, feature, usage_mode, model, usage, None)
}

pub(crate) fn record_trusted_usage_with_key(
    store: &Store,
    user_id: &str,
    feature: &str,
    usage_mode: &str,
    model: Option<&str>,
    usage: &CliTokenUsage,
    idempotency_key: Option<&str>,
) -> Option<crate::store::TokenUsageAccountingResult> {
    let Some(usage) = usage.clone().normalized() else {
        if let Some(key) = idempotency_key {
            crate::billing::release_trusted_call(store, user_id, key, "released_no_usage");
        }
        return None;
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
            record_token_meter_event(
                store,
                user_id,
                feature,
                usage_mode,
                model,
                &usage,
                idempotency_key,
                &result,
            );
            Some(result)
        }
        Err(e) => {
            tracing::error!(
                user_id,
                feature,
                usage_mode,
                "record_trusted_usage 记账失败: {}",
                e
            );
            None
        }
    }
}

fn record_token_meter_event(
    store: &Store,
    user_id: &str,
    feature: &str,
    usage_mode: &str,
    model: Option<&str>,
    usage: &CliTokenUsage,
    idempotency_key: Option<&str>,
    result: &TokenUsageAccountingResult,
) {
    if result.deduplicated || model.unwrap_or_default().starts_with("metered-") {
        return;
    }
    let event = ComputeMeterEvent {
        user_id,
        compute_call_id: idempotency_key,
        feature,
        usage_mode,
        model,
        source: "trusted_token_usage",
        input_unit_kind: "token",
        output_unit_kind: "token",
        input_units: usage.input_tokens,
        output_units: usage.output_tokens,
        metered_input_tokens: usage.input_tokens,
        metered_output_tokens: usage.output_tokens,
        token_usage_event_id: Some(result.token_usage_event_id.as_str()),
        billing_event_id: result.billing_event_id.as_deref(),
        cost_rmb_fen: result.cost_rmb_fen,
        accounting_status: result.accounting_status.as_str(),
    };
    if let Err(error) = store.record_compute_meter_event(&event) {
        tracing::warn!(
            user_id,
            feature,
            usage_mode,
            "record token compute meter event failed: {}",
            error
        );
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
        if let Some(key) = idempotency_key {
            crate::billing::release_trusted_call(store, user_id, key, "released_no_usage");
        }
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
    let _ = record_trusted_usage_with_key(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-token-meter-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).unwrap(), path)
    }

    #[test]
    fn trusted_token_usage_writes_compute_meter_event() {
        let (store, path) = temp_store();
        let user = store
            .create_user(
                &format!("token-meter-{}@example.com", uuid::Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();
        store
            .billing_recharge(&user.id, 1_000, "test", "test", None)
            .unwrap();
        let usage = CliTokenUsage {
            input_tokens: 12,
            output_tokens: 34,
            total_tokens: 46,
            model: Some("gpt-4o-mini".to_string()),
            ..CliTokenUsage::default()
        };

        let result = record_trusted_usage_with_key(
            &store,
            &user.id,
            "codex_cli_chat",
            "server_codex_cli",
            Some("gpt-4o-mini"),
            &usage,
            Some("token:test-meter"),
        )
        .unwrap();
        assert!(!result.deduplicated);

        let events = store.admin_compute_meter_events(30, 10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].input_unit_kind, "token");
        assert_eq!(events[0].output_unit_kind, "token");
        assert_eq!(events[0].input_units, 12);
        assert_eq!(events[0].output_units, 34);
        assert_eq!(events[0].metered_tokens, 46);

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
