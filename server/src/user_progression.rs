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
    pub provided_tokens: i64,
    pub level_floor_tokens: i64,
    pub next_level_tokens: i64,
    pub tokens_into_level: i64,
    pub tokens_to_next_level: i64,
    pub level_progress_ratio: f64,
    pub consumed_progress_ratio: f64,
    pub provided_progress_ratio: f64,
    pub consumed_call_count: i64,
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
    let (consumed_progress_ratio, provided_progress_ratio) = progress_segments(
        consumed_tokens,
        provided_tokens,
        tokens_into_level,
        level_span,
    );

    UserProgressionSummary {
        user_id: user_id.to_string(),
        level,
        tier_name: tier_name(level).to_string(),
        total_xp_tokens,
        consumed_tokens,
        provided_tokens,
        level_floor_tokens,
        next_level_tokens,
        tokens_into_level,
        tokens_to_next_level,
        level_progress_ratio,
        consumed_progress_ratio,
        provided_progress_ratio,
        consumed_call_count: ledger.consumed_call_count.max(0),
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

fn progress_segments(
    consumed_tokens: i64,
    provided_tokens: i64,
    tokens_into_level: i64,
    level_span: i64,
) -> (f64, f64) {
    let total = consumed_tokens.saturating_add(provided_tokens);
    if total <= 0 || tokens_into_level <= 0 {
        return (0.0, 0.0);
    }
    let consumed_segment = ((tokens_into_level as f64) * (consumed_tokens as f64) / (total as f64))
        .round()
        .clamp(0.0, tokens_into_level as f64) as i64;
    let provided_segment = (tokens_into_level - consumed_segment).max(0);
    (
        ratio(consumed_segment, level_span),
        ratio(provided_segment, level_span),
    )
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
                provided_tokens: 40_000,
                consumed_call_count: 3,
                provided_run_count: 2,
                provider_earned_fen: 42,
            },
        );
        assert_eq!(next.level, 3);
        assert!(next.consumed_progress_ratio > next.provided_progress_ratio);
        assert!((next.consumed_progress_ratio + next.provided_progress_ratio) <= 1.0);
    }
}
