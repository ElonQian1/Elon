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

use crate::store::{
    BillingPriceSnapshot, BillingReservationOutcome, BillingReservationRequest, Store,
    TokenUsageAccountingResult, TokenUsageBillingCharge, TokenUsageRecord,
};

const NEW_USER_TRIAL_CREDIT_CONFIG_KEY: &str = "new_user_trial_credit_fen";
const NEW_USER_TRIAL_CREDIT_ENV: &str = "NEW_USER_TRIAL_CREDIT_FEN";
const DEFAULT_NEW_USER_TRIAL_CREDIT_FEN: i64 = 30_000;
pub(crate) const NEW_USER_TRIAL_METHOD: &str = "new_user_trial";
const NEW_USER_TRIAL_OPERATOR: &str = "system";

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
    calc_cost_fen_with_price(
        model_price(model),
        input_tokens,
        cached_input_tokens,
        output_tokens,
        rate_x10000,
        markup_x1000,
    )
}

fn calc_cost_fen_with_price(
    price: (f64, f64, f64),
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
    rate_x10000: i64,
    markup_x1000: i64,
) -> i64 {
    let (inp, cac, out) = price;
    let usd = (input_tokens as f64 / 1_000_000.0) * inp
        + (cached_input_tokens as f64 / 1_000_000.0) * cac
        + (output_tokens as f64 / 1_000_000.0) * out;
    let rmb = usd * (rate_x10000 as f64 / 10_000.0);
    let marked_up = rmb * (markup_x1000 as f64 / 1_000.0);
    // 转分并向上取整（最低 1 分，避免 0 分调用不扣费）
    let fen = (marked_up * 100.0).ceil() as i64;
    fen.max(0)
}

fn price_snapshot_for_store(store: &Store, model: &str) -> BillingPriceSnapshot {
    match store.billing_find_price_rule(model) {
        Ok(Some(rule)) => rule.snapshot(),
        Ok(None) => {
            let (input, cached, output) = model_price(model);
            BillingPriceSnapshot::fallback(input, cached, output)
        }
        Err(e) => {
            warn!(
                model,
                "billing price rule lookup failed, using built-in fallback: {}", e
            );
            let (input, cached, output) = model_price(model);
            BillingPriceSnapshot::fallback(input, cached, output)
        }
    }
}

// ── 公开钩子 ──────────────────────────────────────────────────────────────────

/// LLM 调用前：检查余额是否充足。
///
/// - 用户没有 user_balance 行 → 默认创建 0 余额并拦截
/// - 兼容模式下用户没有 user_balance 行 → 视为未开通计费，直接放行
/// - 有行且 balance_fen > 0 → 放行
/// - 有行且 balance_fen <= 0 → 返回 Err（错误消息直接展示给用户）
pub fn check_can_call(store: &Store, user_id: &str) -> Result<(), String> {
    release_expired_reservations_best_effort(store);

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
            if let Some(balance) = try_grant_new_user_trial_credit(store, user_id)? {
                if balance > 0 {
                    return Ok(());
                }
            }
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
        Ok(Some(fen)) => {
            let mut current_fen = fen;
            if let Some(balance) = try_grant_new_user_trial_credit(store, user_id)? {
                if balance > 0 {
                    return Ok(());
                }
                current_fen = balance;
            }
            Err(format!(
                "余额不足（当前 {} 分），请联系管理员充值后继续使用",
                current_fen
            ))
        }
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

fn try_grant_new_user_trial_credit(store: &Store, user_id: &str) -> Result<Option<i64>, String> {
    let amount_fen = new_user_trial_credit_fen(store);
    if amount_fen <= 0 {
        return Ok(None);
    }
    match store.billing_grant_once(
        user_id,
        amount_fen,
        NEW_USER_TRIAL_METHOD,
        NEW_USER_TRIAL_OPERATOR,
        Some("new user trial credit"),
    ) {
        Ok(balance) => Ok(balance),
        Err(e) => {
            warn!(
                "new user trial credit grant failed for {} ({} fen): {}",
                user_id, amount_fen, e
            );
            if billing_db_fail_open() {
                Ok(None)
            } else {
                Err("计费系统暂时不可用，请稍后重试".to_string())
            }
        }
    }
}

pub(crate) fn new_user_trial_credit_fen(store: &Store) -> i64 {
    std::env::var(NEW_USER_TRIAL_CREDIT_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .or_else(|| {
            store
                .billing_get_config(NEW_USER_TRIAL_CREDIT_CONFIG_KEY)
                .ok()
                .flatten()
                .and_then(|value| value.trim().parse::<i64>().ok())
        })
        .unwrap_or(DEFAULT_NEW_USER_TRIAL_CREDIT_FEN)
        .max(0)
}

pub fn reserve_trusted_call(
    store: &Store,
    user_id: &str,
    compute_call_id: &str,
    feature: &str,
    usage_mode: &str,
    model: Option<&str>,
    estimated_cost_fen: i64,
) -> Result<Option<BillingReservationOutcome>, String> {
    let compute_call_id = compute_call_id.trim();
    if compute_call_id.is_empty() {
        check_can_call(store, user_id)?;
        return Ok(None);
    }
    release_expired_reservations_best_effort(store);

    if let Err(e) = store.check_user_quota(user_id) {
        let msg = e.to_string();
        if msg.contains("用户已被封禁") || msg.contains("token 用量已达上限") {
            return Err(msg);
        }
        warn!(
            "billing reservation quota check db error for {}: {}",
            user_id, e
        );
        if !billing_db_fail_open() {
            return Err("计费系统暂时不可用，请稍后重试".to_string());
        }
    }

    if billing_required_for_all_users(store) {
        let should_try_trial = match store.billing_get_balance(user_id) {
            Ok(None) => true,
            Ok(Some(fen)) => fen < estimated_cost_fen.max(0),
            Err(e) => {
                warn!(
                    "billing balance lookup before reservation failed for {}: {}",
                    user_id, e
                );
                false
            }
        };
        if should_try_trial {
            let _ = try_grant_new_user_trial_credit(store, user_id)?;
        }
    }

    let request = BillingReservationRequest {
        user_id,
        compute_call_id,
        feature,
        usage_mode,
        model,
        reserve_fen: estimated_cost_fen.max(0),
        bill_missing_balance: billing_required_for_all_users(store),
    };
    match store.reserve_billing_call(&request) {
        Ok(outcome) => {
            if let Some(balance) = outcome.balance_after_fen {
                publish_low_balance_if_needed(store, user_id, balance);
            }
            Ok(Some(outcome))
        }
        Err(e) => {
            let msg = e.to_string();
            warn!(
                user_id,
                compute_call_id, feature, usage_mode, "billing reservation failed: {}", msg
            );
            if billing_db_fail_open() && !msg.contains("余额不足") {
                Ok(None)
            } else {
                Err(msg)
            }
        }
    }
}

pub fn release_trusted_call(store: &Store, user_id: &str, compute_call_id: &str, status: &str) {
    if compute_call_id.trim().is_empty() {
        return;
    }
    if let Err(e) = store.release_billing_call(user_id, compute_call_id, status) {
        warn!(
            user_id,
            compute_call_id, "billing reservation release failed: {}", e
        );
    }
}

pub fn configured_reservation_fen(store: &Store, key: &str, fallback: i64) -> i64 {
    std::env::var(key.to_ascii_uppercase())
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .or_else(|| {
            store
                .billing_get_config(key)
                .ok()
                .flatten()
                .and_then(|value| value.trim().parse::<i64>().ok())
        })
        .unwrap_or(fallback)
        .max(0)
}

pub fn estimate_cost_for_tokens(
    store: &Store,
    model: &str,
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
) -> i64 {
    let (rate, markup) = store.billing_get_rate_and_markup();
    let price = price_snapshot_for_store(store, model);
    calc_cost_fen_with_price(
        price.price_tuple(),
        input_tokens.max(0),
        cached_input_tokens.max(0),
        output_tokens.max(0),
        rate,
        markup,
    )
}

/// 可信用量记账：插入 token 用量，并在用户已开通预存计费时原子扣款。
pub fn account_trusted_usage(
    store: &Store,
    record: &TokenUsageRecord<'_>,
) -> anyhow::Result<TokenUsageAccountingResult> {
    let (rate, markup) = store.billing_get_rate_and_markup();
    let model = record.model.unwrap_or("unknown");
    let price = price_snapshot_for_store(store, model);
    let cost_fen = calc_cost_fen_with_price(
        price.price_tuple(),
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
        price_snapshot: price,
        bill_missing_balance: billing_required_for_all_users(store),
    };
    let result = store.record_token_usage_with_billing(record, &charge)?;
    if let Some(balance) = result.balance_after_fen {
        publish_low_balance_if_needed(store, record.user_id, balance);
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
    let price = price_snapshot_for_store(store, model);
    let cost_fen = calc_cost_fen_with_price(
        price.price_tuple(),
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
        price,
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

fn publish_low_balance_if_needed(store: &Store, user_id: &str, balance: i64) {
    let threshold = store
        .billing_get_config("low_balance_threshold_fen")
        .ok()
        .flatten()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(100);
    if balance <= threshold {
        crate::billing_events::publish_low_balance(user_id.to_string(), balance);
    }
}

fn release_expired_reservations_best_effort(store: &Store) {
    match store.release_expired_billing_reservations() {
        Ok(0) => {}
        Ok(n) => tracing::info!("released {} expired billing reservations", n),
        Err(e) => warn!("release expired billing reservations failed: {}", e),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::BillingPriceRuleUpsert;
    use uuid::Uuid;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon_billing_runtime_{}.db",
            Uuid::new_v4().simple()
        ));
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn strict_billing_grants_new_user_trial_credit_before_first_call() {
        let (store, path) = temp_store();
        let expected = new_user_trial_credit_fen(&store);
        if expected <= 0 {
            let _ = std::fs::remove_file(path);
            return;
        }

        let user = store
            .create_user(
                &format!("trial-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();
        assert_eq!(store.billing_get_balance(&user.id).unwrap(), None);

        check_can_call(&store, &user.id).unwrap();

        assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(expected));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn new_user_trial_credit_is_only_granted_once() {
        let (store, path) = temp_store();
        store
            .billing_set_config("new_user_trial_credit_fen", "100")
            .unwrap();
        let expected = new_user_trial_credit_fen(&store);
        if expected <= 0 {
            let _ = std::fs::remove_file(path);
            return;
        }

        let user = store
            .create_user(
                &format!("trial-once-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();

        check_can_call(&store, &user.id).unwrap();
        store
            .billing_deduct(
                &user.id,
                expected,
                None,
                0,
                0,
                0,
                73000,
                1200,
                BillingPriceSnapshot::legacy(),
            )
            .unwrap();
        assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(0));

        let err = check_can_call(&store, &user.id).unwrap_err();
        assert!(err.contains("余额不足"));
        assert_eq!(store.billing_get_balance(&user.id).unwrap(), Some(0));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn trial_credit_lookup_sums_topup_records() {
        let (store, path) = temp_store();
        let user = store
            .create_user(
                &format!("trial-topup-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();

        store
            .billing_recharge(
                &user.id,
                100,
                NEW_USER_TRIAL_METHOD,
                "system",
                Some("old trial"),
            )
            .unwrap();
        store
            .billing_recharge(
                &user.id,
                29_900,
                NEW_USER_TRIAL_METHOD,
                "system",
                Some("trial top-up"),
            )
            .unwrap();

        let grant = store
            .billing_find_recharge_by_method(&user.id, NEW_USER_TRIAL_METHOD)
            .unwrap()
            .expect("trial grant should be summarized");
        assert_eq!(grant.amount_fen, 30_000);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reservation_grants_trial_credit_before_holding_balance() {
        let (store, path) = temp_store();
        let expected = new_user_trial_credit_fen(&store);
        if expected <= 0 {
            let _ = std::fs::remove_file(path);
            return;
        }
        let reserve_fen = expected.min(10);
        let user = store
            .create_user(
                &format!("trial-reserve-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();

        reserve_trusted_call(
            &store,
            &user.id,
            "test-reservation",
            "chat",
            "server_api_key",
            Some("test-model"),
            reserve_fen,
        )
        .unwrap();

        assert_eq!(
            store.billing_get_balance(&user.id).unwrap(),
            Some(expected - reserve_fen)
        );
        release_trusted_call(&store, &user.id, "test-reservation", "released_no_usage");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn estimate_cost_uses_configured_price_rule() {
        let (store, path) = temp_store();
        store
            .billing_set_config("usd_to_rmb_rate_x10000", "10000")
            .unwrap();
        store.billing_set_config("markup_x1000", "1000").unwrap();
        let rule = store
            .billing_upsert_price_rule(&BillingPriceRuleUpsert {
                pattern: "custom-expensive".to_string(),
                input_usd_per_m: 1.0,
                cached_usd_per_m: 0.5,
                output_usd_per_m: 1000.0,
                priority: 999,
                enabled: true,
                note: Some("snapshot test".to_string()),
            })
            .unwrap();

        let cost = estimate_cost_for_tokens(&store, "custom-expensive-v1", 0, 0, 1_000_000);
        assert_eq!(cost, 100_000);
        let user = store
            .create_user(
                &format!("billing-price-{}@example.com", Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();
        store
            .billing_recharge(&user.id, 200_000, "test", "test", None)
            .unwrap();

        let record = TokenUsageRecord {
            user_id: &user.id,
            feature: "price_rule_feature",
            usage_mode: "server_codex_cli",
            model: Some("custom-expensive-v1"),
            input_tokens: 0,
            cached_input_tokens: 0,
            output_tokens: 1_000_000,
            reasoning_tokens: 0,
            total_tokens: 1_000_000,
            idempotency_key: Some("price-snapshot-key-1"),
        };
        let result = account_trusted_usage(&store, &record).unwrap();
        assert_eq!(result.accounting_status, "billed");
        assert_eq!(result.cost_rmb_fen, 100_000);

        let (events, total) = store.billing_list_events(&user.id, 1, 10).unwrap();
        assert_eq!(total, 1);
        let event = &events[0];
        assert_eq!(
            event.token_usage_event_id.as_deref(),
            Some(result.token_usage_event_id.as_str())
        );
        assert_eq!(event.price_rule_id.as_deref(), Some(rule.id.as_str()));
        assert_eq!(event.price_rule_version, Some(rule.version));
        assert_eq!(
            event.price_rule_pattern.as_deref(),
            Some("custom-expensive")
        );
        assert_eq!(event.input_usd_per_m, Some(1.0));
        assert_eq!(event.cached_usd_per_m, Some(0.5));
        assert_eq!(event.output_usd_per_m, Some(1000.0));
        assert_eq!(event.price_source.as_str(), "rule");
        let _ = std::fs::remove_file(path);
    }
}
