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
            provider_week_start_at: "2026-07-06T00:00:00+00:00".to_string(),
            provider_week_end_at: "2026-07-13T00:00:00+00:00".to_string(),
            provider_week_tokens: 30_000,
            provider_week_run_count: 2,
            provider_week_billed_fen: 100,
            provider_week_earned_fen: 42,
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
            provider_week_start_at: "2026-07-06T00:00:00+00:00".to_string(),
            provider_week_end_at: "2026-07-13T00:00:00+00:00".to_string(),
            provider_week_tokens: 40_000,
            provider_week_run_count: 2,
            provider_week_billed_fen: 100,
            provider_week_earned_fen: 42,
        },
    );
    assert_eq!(next.level, 3);
    assert!(next.consumed_progress_ratio > next.provided_progress_ratio);
    assert!(next.own_codex_progress_ratio > next.shared_codex_progress_ratio);
    assert!(next.platform_progress_ratio > 0.0);
    assert!((next.consumed_progress_ratio + next.provided_progress_ratio) <= 1.0);
}
