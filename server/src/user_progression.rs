//! User-facing level and experience API.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::{
    project_auth::{auth_from_headers, json_error},
    store::UserProgressionLedger,
    types::AppState,
};

const LEVEL_BASE_STEP_TOKENS: f64 = 50_000.0;
const LEVEL_GROWTH: f64 = 1.6;
const MAX_LEVEL: i64 = 200;

#[derive(Debug, Clone, Serialize)]
pub struct UserProgressionSummary {
    pub user_id: String,
    pub level: i64,
    pub tier_name: String,
    pub total_xp_tokens: i64,
    pub consumed_tokens: i64,
    pub own_codex_tokens: i64,
    pub shared_codex_tokens: i64,
    pub platform_tokens: i64,
    pub provided_tokens: i64,
    pub level_floor_tokens: i64,
    pub next_level_tokens: i64,
    pub tokens_into_level: i64,
    pub tokens_to_next_level: i64,
    pub level_progress_ratio: f64,
    pub consumed_progress_ratio: f64,
    pub own_codex_progress_ratio: f64,
    pub shared_codex_progress_ratio: f64,
    pub platform_progress_ratio: f64,
    pub provided_progress_ratio: f64,
    pub consumed_call_count: i64,
    pub own_codex_call_count: i64,
    pub shared_codex_call_count: i64,
    pub platform_call_count: i64,
    pub provided_run_count: i64,
    pub provider_earned_fen: i64,
}

pub async fn get_my_progression(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(err) => return json_error(StatusCode::UNAUTHORIZED, err.to_string()),
    };

    match state.store.user_progression_ledger(&user.id) {
        Ok(ledger) => Json(build_progression_summary(&user.id, ledger)).into_response(),
        Err(err) => {
            tracing::warn!(user_id = %user.id, "load user progression failed: {err}");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "等级经验读取失败，请稍后重试",
            )
        }
    }
}

pub(crate) fn build_progression_summary(
    user_id: &str,
    ledger: UserProgressionLedger,
) -> UserProgressionSummary {
    let consumed_tokens = ledger.consumed_tokens.max(0);
    let own_codex_tokens = ledger.own_codex_tokens.max(0);
    let shared_codex_tokens = ledger.shared_codex_tokens.max(0);
    let mut platform_tokens = ledger.platform_tokens.max(0);
    let classified_consumed = own_codex_tokens.saturating_add(shared_codex_tokens);
    if consumed_tokens > classified_consumed.saturating_add(platform_tokens) {
        platform_tokens = consumed_tokens.saturating_sub(classified_consumed);
    }
    let provided_tokens = ledger.provided_tokens.max(0);
    let total_xp_tokens = consumed_tokens.saturating_add(provided_tokens);
    let level = level_for_tokens(total_xp_tokens);
    let level_floor_tokens = tokens_required_for_level(level);
    let next_level_tokens =
        tokens_required_for_level(level.saturating_add(1)).max(level_floor_tokens + 1);
    let level_span = (next_level_tokens - level_floor_tokens).max(1);
    let tokens_into_level = (total_xp_tokens - level_floor_tokens).clamp(0, level_span);
    let tokens_to_next_level = (next_level_tokens - total_xp_tokens).max(0);
    let level_progress_ratio = ratio(tokens_into_level, level_span);
    let segments = progress_segments(
        &[
            own_codex_tokens,
            shared_codex_tokens,
            platform_tokens,
            provided_tokens,
        ],
        tokens_into_level,
        level_span,
    );
    let own_codex_progress_ratio = segments[0];
    let shared_codex_progress_ratio = segments[1];
    let platform_progress_ratio = segments[2];
    let provided_progress_ratio = segments[3];
    let consumed_progress_ratio =
        (own_codex_progress_ratio + shared_codex_progress_ratio + platform_progress_ratio)
            .clamp(0.0, 1.0);

    UserProgressionSummary {
        user_id: user_id.to_string(),
        level,
        tier_name: tier_name(level).to_string(),
        total_xp_tokens,
        consumed_tokens,
        own_codex_tokens,
        shared_codex_tokens,
        platform_tokens,
        provided_tokens,
        level_floor_tokens,
        next_level_tokens,
        tokens_into_level,
        tokens_to_next_level,
        level_progress_ratio,
        consumed_progress_ratio,
        own_codex_progress_ratio,
        shared_codex_progress_ratio,
        platform_progress_ratio,
        provided_progress_ratio,
        consumed_call_count: ledger.consumed_call_count.max(0),
        own_codex_call_count: ledger.own_codex_call_count.max(0),
        shared_codex_call_count: ledger.shared_codex_call_count.max(0),
        platform_call_count: ledger.platform_call_count.max(0),
        provided_run_count: ledger.provided_run_count.max(0),
        provider_earned_fen: ledger.provider_earned_fen.max(0),
    }
}

fn level_for_tokens(tokens: i64) -> i64 {
    if tokens <= 0 {
        return 1;
    }
    let raw = (((tokens as f64 * (LEVEL_GROWTH - 1.0)) / LEVEL_BASE_STEP_TOKENS + 1.0).ln()
        / LEVEL_GROWTH.ln())
    .floor() as i64
        + 1;
    let mut level = raw.clamp(1, MAX_LEVEL);
    while level < MAX_LEVEL && tokens_required_for_level(level + 1) <= tokens {
        level += 1;
    }
    while level > 1 && tokens_required_for_level(level) > tokens {
        level -= 1;
    }
    level
}

fn tokens_required_for_level(level: i64) -> i64 {
    if level <= 1 {
        return 0;
    }
    let exponent = (level - 1).min(MAX_LEVEL - 1) as i32;
    ((LEVEL_BASE_STEP_TOKENS * (LEVEL_GROWTH.powi(exponent) - 1.0)) / (LEVEL_GROWTH - 1.0))
        .round()
        .max(0.0) as i64
}

fn progress_segments(values: &[i64], tokens_into_level: i64, level_span: i64) -> Vec<f64> {
    let total = values
        .iter()
        .fold(0i64, |sum, value| sum.saturating_add((*value).max(0)));
    if total <= 0 || tokens_into_level <= 0 {
        return vec![0.0; values.len()];
    }
    let mut remaining = tokens_into_level;
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let segment = if index + 1 == values.len() {
                remaining.max(0)
            } else {
                let raw = ((tokens_into_level as f64) * ((*value).max(0) as f64) / (total as f64))
                    .round() as i64;
                let segment = raw.clamp(0, remaining.max(0));
                remaining -= segment;
                segment
            };
            ratio(segment, level_span)
        })
        .collect()
}

fn ratio(value: i64, total: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        (value as f64 / total as f64).clamp(0.0, 1.0)
    }
}

fn tier_name(level: i64) -> &'static str {
    match level {
        1..=4 => "初阶算力",
        5..=9 => "稳定创作者",
        10..=19 => "云端开发者",
        20..=39 => "算力合伙人",
        40..=79 => "平台建造者",
        _ => "长期贡献者",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_curve_slows_down_as_tokens_grow() {
        assert_eq!(level_for_tokens(0), 1);
        assert_eq!(tokens_required_for_level(2), 50_000);
        assert_eq!(tokens_required_for_level(3), 130_000);
        assert_eq!(level_for_tokens(49_999), 1);
        assert_eq!(level_for_tokens(50_000), 2);
        assert_eq!(level_for_tokens(129_999), 2);
        assert_eq!(level_for_tokens(130_000), 3);
        assert!(
            tokens_required_for_level(10) - tokens_required_for_level(9)
                > tokens_required_for_level(3) - tokens_required_for_level(2)
        );
    }

    #[test]
    fn summary_splits_current_level_bar_by_consumed_and_provided_share() {
        let summary = build_progression_summary(
            "u1",
            UserProgressionLedger {
                consumed_tokens: 100_000,
                own_codex_tokens: 0,
                own_codex_call_count: 0,
                shared_codex_tokens: 0,
                shared_codex_call_count: 0,
                platform_tokens: 100_000,
                platform_call_count: 3,
                provided_tokens: 30_000,
                consumed_call_count: 3,
                provided_run_count: 2,
                provider_earned_fen: 42,
            },
        );
        assert_eq!(summary.level, 3);
        assert_eq!(summary.total_xp_tokens, 130_000);
        assert_eq!(summary.tokens_into_level, 0);
        assert_eq!(summary.tokens_to_next_level, 128_000);

        let next = build_progression_summary(
            "u1",
            UserProgressionLedger {
                consumed_tokens: 120_000,
                own_codex_tokens: 60_000,
                own_codex_call_count: 1,
                shared_codex_tokens: 20_000,
                shared_codex_call_count: 1,
                platform_tokens: 40_000,
                platform_call_count: 1,
                provided_tokens: 40_000,
                consumed_call_count: 3,
                provided_run_count: 2,
                provider_earned_fen: 42,
            },
        );
        assert_eq!(next.level, 3);
        assert!(next.consumed_progress_ratio > next.provided_progress_ratio);
        assert!(next.own_codex_progress_ratio > next.shared_codex_progress_ratio);
        assert!(next.platform_progress_ratio > 0.0);
        assert!((next.consumed_progress_ratio + next.provided_progress_ratio) <= 1.0);
    }
}
