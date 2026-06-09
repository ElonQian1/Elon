//! 预存计费业务逻辑层。
//!
//! 对外提供两个关键钩子：
//! - `check_can_call(store, user_id)` —— LLM 调用前调用，余额不足时返回错误
//! - `account_trusted_usage(store, record)` —— 可信用量落库并原子扣费
//! - `deduct_usage(store, user_id, model, input, cached, output)` —— 兼容旧调用点，直接扣费
//! - `deduct_from_response(store, user_id, model, response)` —— 兼容旧调用点，静默扣费
//!
//! 默认严格计费：若用户没有 `user_balance` 行，调用前自动创建 0 余额并拦截。
//! 仅当 `billing_required_for_all_users=false` 时进入兼容模式，缺余额行才放行。

use serde_json::Value;
use tracing::warn;

use crate::store::{Store, TokenUsageAccountingResult, TokenUsageBillingCharge, TokenUsageRecord};

// ── 模型定价表（USD / 1M tokens）─────────────────────────────────────────────
// 返回 (input_per_m, cached_per_m, output_per_m)，单位：美元 / 百万 token

fn model_price(model: &str) -> (f64, f64, f64) {
    let m = model.to_lowercase();
    if m.contains("gpt-4o-mini") || m.contains("gpt4o-mini") {
        (0.15, 0.075, 0.60)
    } else if m.contains("gpt-4o") || m.contains("gpt4o") {
        (2.5, 1.25, 10.0)
    } else if m.contains("o3-mini") {
        (1.1, 0.55, 4.4)
    } else if m.contains("claude-3-5-haiku") || m.contains("claude-3.5-haiku") {
        (0.25, 0.03, 1.25)
    } else if m.contains("claude-3-haiku") {
        (0.25, 0.03, 1.25)
    } else if m.contains("claude-opus-4") || m.contains("claude-opus") {
        (15.0, 1.5, 75.0)
    } else if m.contains("claude-sonnet-4") || m.contains("claude-3-7") || m.contains("claude-3.7")
    {
        (3.0, 0.3, 15.0)
    } else if m.contains("claude-3-5-sonnet") || m.contains("claude-3.5-sonnet") {
        (3.0, 0.3, 15.0)
    } else if m.contains("claude") {
        (3.0, 0.3, 15.0)
    } else if m.contains("deepseek") {
        (0.14, 0.014, 0.28)
    } else if m.contains("metered-image") {
        // 非 token 图片算力的内部计量单位，按 output units 计费。
        (0.0, 0.0, 5.0)
    } else if m.contains("metered-realtime") {
        // 实时语音按输入/输出音频时长折算的内部计量单位。
        (1.0, 0.0, 2.0)
    } else if m.contains("metered-asr") || m.contains("metered-tts") {
        // ASR/TTS Worker 或外部语音 API 的内部计量单位。
        (1.0, 0.0, 1.0)
    } else {
        // 未知模型保守估算
        (3.0, 0.3, 15.0)
    }
}

// ── 费用计算 ──────────────────────────────────────────────────────────────────

/// 计算本次 LLM 调用的费用（人民币分，向上取整到 1 分）。
///
/// - `rate_x10000`：汇率 × 10000（73000 = 7.3000）
/// - `markup_x1000`：加价比例 × 1000（1200 = ×1.2）
pub fn calc_cost_fen(
    model: &str,
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
    rate_x10000: i64,
    markup_x1000: i64,
) -> i64 {
    let (inp, cac, out) = model_price(model);
    let usd = (input_tokens as f64 / 1_000_000.0) * inp
        + (cached_input_tokens as f64 / 1_000_000.0) * cac
        + (output_tokens as f64 / 1_000_000.0) * out;
    let rmb = usd * (rate_x10000 as f64 / 10_000.0);
    let marked_up = rmb * (markup_x1000 as f64 / 1_000.0);
    // 转分并向上取整（最低 1 分，避免 0 分调用不扣费）
    let fen = (marked_up * 100.0).ceil() as i64;
    fen.max(0)
}

// ── 公开钩子 ──────────────────────────────────────────────────────────────────

/// LLM 调用前：检查余额是否充足。
///
/// - 用户没有 user_balance 行 → 默认创建 0 余额并拦截
/// - 兼容模式下用户没有 user_balance 行 → 视为未开通计费，直接放行
/// - 有行且 balance_fen > 0 → 放行
/// - 有行且 balance_fen <= 0 → 返回 Err（错误消息直接展示给用户）
pub fn check_can_call(store: &Store, user_id: &str) -> Result<(), String> {
    if let Err(e) = store.check_user_quota(user_id) {
        let msg = e.to_string();
        if msg.contains("用户已被封禁") || msg.contains("token 用量已达上限") {
            return Err(msg);
        }
        warn!("billing quota check db error for {}: {}", user_id, e);
        if !billing_db_fail_open() {
            return Err("计费系统暂时不可用，请稍后重试".to_string());
        }
    }

    match store.billing_get_balance(user_id) {
        Ok(None) if billing_required_for_all_users(store) => {
            if let Err(e) = store.billing_ensure_balance_row(user_id) {
                warn!("billing ensure balance row failed for {}: {}", user_id, e);
                if billing_db_fail_open() {
                    return Ok(());
                }
                return Err("计费系统暂时不可用，请稍后重试".to_string());
            }
            Err("余额不足（当前 0 分），请联系管理员充值后继续使用".to_string())
        }
        Ok(None) => Ok(()), // 兼容模式：未开通计费，放行
        Ok(Some(fen)) if fen > 0 => Ok(()),
        Ok(Some(fen)) => Err(format!(
            "余额不足（当前 {} 分），请联系管理员充值后继续使用",
            fen
        )),
        Err(e) => {
            warn!("billing check_can_call db error for {}: {}", user_id, e);
            if billing_db_fail_open() {
                Ok(())
            } else {
                Err("计费系统暂时不可用，请稍后重试".to_string())
            }
        }
    }
}

/// 可信用量记账：插入 token 用量，并在用户已开通预存计费时原子扣款。
pub fn account_trusted_usage(
    store: &Store,
    record: &TokenUsageRecord<'_>,
) -> anyhow::Result<TokenUsageAccountingResult> {
    let (rate, markup) = store.billing_get_rate_and_markup();
    let model = record.model.unwrap_or("unknown");
    let cost_fen = calc_cost_fen(
        model,
        record.input_tokens.max(0),
        record.cached_input_tokens.max(0),
        record.output_tokens.max(0),
        rate,
        markup,
    );
    let charge = TokenUsageBillingCharge {
        model: record.model,
        input_tokens: record.input_tokens.max(0),
        cached_input_tokens: record.cached_input_tokens.max(0),
        output_tokens: record.output_tokens.max(0),
        cost_rmb_fen: cost_fen,
        exchange_rate_x10000: rate,
        markup_x1000: markup,
        bill_missing_balance: billing_required_for_all_users(store),
    };
    let result = store.record_token_usage_with_billing(record, &charge)?;
    if let Some(balance) = result.balance_after_fen {
        let threshold = store
            .billing_get_config("low_balance_threshold_fen")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(100);
        if balance <= threshold {
            crate::billing_events::publish_low_balance(record.user_id.to_string(), balance);
        }
    }
    Ok(result)
}

/// LLM 调用后：用已解析出的 token 用量执行扣费。
///
/// 兼容旧调用点：严格模式之外，用户没有 `user_balance` 行时不扣 RMB。
pub fn deduct_usage(
    store: &Store,
    user_id: &str,
    model: Option<&str>,
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
) -> anyhow::Result<Option<i64>> {
    let Some(_) = store.billing_get_balance(user_id)? else {
        return Ok(None);
    };
    if input_tokens <= 0 && output_tokens <= 0 {
        return Ok(None);
    }

    let (rate, markup) = store.billing_get_rate_and_markup();
    let model = model.unwrap_or("unknown");
    let cost_fen = calc_cost_fen(
        model,
        input_tokens.max(0),
        cached_input_tokens.max(0),
        output_tokens.max(0),
        rate,
        markup,
    );
    if cost_fen == 0 {
        return Ok(None);
    }

    let new_balance = store.billing_deduct(
        user_id,
        cost_fen,
        Some(model),
        input_tokens.max(0),
        cached_input_tokens.max(0),
        output_tokens.max(0),
        rate,
        markup,
    )?;

    let threshold = store
        .billing_get_config("low_balance_threshold_fen")
        .ok()
        .flatten()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(100);
    if new_balance <= threshold {
        crate::billing_events::publish_low_balance(user_id.to_string(), new_balance);
    }

    Ok(Some(new_balance))
}

/// LLM 调用后：从响应 JSON 中提取 token 用量并扣费（静默，不向上层传播错误）。
///
/// 兼容 OpenAI 格式（`prompt_tokens` / `completion_tokens`）。
pub fn deduct_from_response(store: &Store, user_id: &str, model: &str, response: &Value) {
    // 若用户未开通计费，快速跳出
    let has_billing = match store.billing_get_balance(user_id) {
        Ok(Some(_)) => true,
        _ => return,
    };
    if !has_billing {
        return;
    }

    let usage = &response["usage"];
    if usage.is_null() {
        return;
    }

    let input = usage["prompt_tokens"].as_i64().unwrap_or(0);
    let cached = usage["prompt_tokens_details"]["cached_tokens"]
        .as_i64()
        .unwrap_or(0);
    // 兼容 Anthropic 格式
    let input = if input == 0 {
        usage["input_tokens"].as_i64().unwrap_or(0)
    } else {
        input
    };
    let cached = if cached == 0 {
        usage["cache_read_input_tokens"].as_i64().unwrap_or(0)
    } else {
        cached
    };
    let output = usage["completion_tokens"]
        .as_i64()
        .unwrap_or_else(|| usage["output_tokens"].as_i64().unwrap_or(0));

    if input == 0 && output == 0 {
        return;
    }

    match deduct_usage(store, user_id, Some(model), input, cached, output) {
        Ok(_) => {}
        Err(e) => {
            // 扣费失败（不应发生，因为 check_can_call 已经验证过）
            warn!("billing deduct failed for {}: {}", user_id, e);
        }
    }
}

fn billing_db_fail_open() -> bool {
    std::env::var("BILLING_DB_FAIL_OPEN")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            value == "1" || value == "true" || value == "yes"
        })
        .unwrap_or(false)
}

fn billing_required_for_all_users(store: &Store) -> bool {
    std::env::var("BILLING_REQUIRED_FOR_ALL_USERS")
        .ok()
        .map(|value| truthy(&value))
        .or_else(|| {
            store
                .billing_get_config("billing_required_for_all_users")
                .ok()
                .flatten()
                .map(|value| truthy(&value))
        })
        .unwrap_or(true)
}

fn truthy(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value == "1" || value == "true" || value == "yes" || value == "on"
}
