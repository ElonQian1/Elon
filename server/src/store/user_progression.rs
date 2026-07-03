//! User progression ledger aggregation.
//!
//! The level system is intentionally read-model only: it derives experience from
//! the existing trusted token ledger and node settlement ledger, so every agent
//! that already records usage through the shared accounting path is included.

use anyhow::Result;
use rusqlite::params;

use super::Store;

#[derive(Debug, Clone, Default)]
pub struct UserProgressionLedger {
    pub consumed_tokens: i64,
    pub consumed_call_count: i64,
    pub own_codex_tokens: i64,
    pub own_codex_call_count: i64,
    pub shared_codex_tokens: i64,
    pub shared_codex_call_count: i64,
    pub platform_tokens: i64,
    pub platform_call_count: i64,
    pub provided_tokens: i64,
    pub provided_run_count: i64,
    pub provider_earned_fen: i64,
}

impl Store {
    pub fn user_progression_ledger(&self, user_id: &str) -> Result<UserProgressionLedger> {
        let conn = self.conn.lock().unwrap();
        let (
            consumed_tokens,
            consumed_call_count,
            own_codex_tokens,
            own_codex_call_count,
            shared_codex_tokens,
            shared_codex_call_count,
            platform_tokens,
            platform_call_count,
        ): (i64, i64, i64, i64, i64, i64, i64, i64) = conn.query_row(
            "SELECT COALESCE(SUM(total_tokens), 0),
                    COUNT(*),
                    COALESCE(SUM(CASE WHEN COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') = 'own_codex' THEN total_tokens ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') = 'own_codex' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') = 'shared_codex' THEN total_tokens ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') = 'shared_codex' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') NOT IN ('own_codex', 'shared_codex') THEN total_tokens ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN COALESCE(NULLIF(TRIM(billing_source), ''), 'platform') NOT IN ('own_codex', 'shared_codex') THEN 1 ELSE 0 END), 0)
               FROM token_usage_events
              WHERE user_id = ?1
                AND usage_mode NOT IN ('client_reported', 'user_api_key_proxy')
                AND total_tokens > 0",
            params![user_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )?;

        let (provided_tokens, provided_run_count, provider_earned_fen): (i64, i64, i64) = conn
            .query_row(
                "SELECT COALESCE(SUM(prompt_tokens + completion_tokens), 0),
                        COUNT(*),
                        COALESCE(SUM(provider_earned_fen), 0)
                   FROM node_transactions
                  WHERE provider_user_id = ?1
                    AND consumer_user_id != provider_user_id
                    AND (prompt_tokens + completion_tokens) > 0",
                params![user_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;

        Ok(UserProgressionLedger {
            consumed_tokens: consumed_tokens.max(0),
            consumed_call_count: consumed_call_count.max(0),
            own_codex_tokens: own_codex_tokens.max(0),
            own_codex_call_count: own_codex_call_count.max(0),
            shared_codex_tokens: shared_codex_tokens.max(0),
            shared_codex_call_count: shared_codex_call_count.max(0),
            platform_tokens: platform_tokens.max(0),
            platform_call_count: platform_call_count.max(0),
            provided_tokens: provided_tokens.max(0),
            provided_run_count: provided_run_count.max(0),
            provider_earned_fen: provider_earned_fen.max(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{SettleParams, TokenUsageRecord};

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-user-progression-ledger-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn ledger_counts_trusted_consumption_and_other_user_node_usage() {
        let (store, path) = temp_store();
        let user = store
            .create_user("progression-user@example.com", "secret1", None, None)
            .expect("user should create");
        let consumer = store
            .create_user("progression-consumer@example.com", "secret1", None, None)
            .expect("consumer should create");

        store
            .record_token_usage(&TokenUsageRecord {
                user_id: &user.id,
                feature: "codex_cli_chat",
                usage_mode: "server_codex_cli",
                model: Some("gpt-5-codex"),
                input_tokens: 80_000,
                cached_input_tokens: 0,
                output_tokens: 40_000,
                reasoning_tokens: 0,
                total_tokens: 120_000,
                billing_source: None,
                resource_owner_user_id: None,
                idempotency_key: None,
            })
            .expect("trusted usage should record");
        store
            .record_token_usage(&TokenUsageRecord {
                user_id: &user.id,
                feature: "mobile_report",
                usage_mode: "client_reported",
                model: None,
                input_tokens: 1_000_000,
                cached_input_tokens: 0,
                output_tokens: 0,
                reasoning_tokens: 0,
                total_tokens: 1_000_000,
                billing_source: None,
                resource_owner_user_id: None,
                idempotency_key: None,
            })
            .expect("client report should record");

        store
            .settle_node_inference(SettleParams {
                consumer_user_id: &consumer.id,
                provider_user_id: &user.id,
                node_id: "node-a",
                model_id: "pc-cli/codex",
                feature: "pc_agent_cli_dev",
                usage_mode: "pc_agent_cli",
                compute_call_id: Some("progression:req-1"),
                token_usage_event_id: Some("tok-progression-1"),
                billing_event_id: Some("bev-progression-1"),
                prompt_tokens: 10_000,
                completion_tokens: 2_500,
                price_per_1k_credits: 1.0,
                billed_cost_rmb_fen: 100,
                accounting_status: Some("billed"),
                provider_revenue_share_x1000: 800,
                platform_fee_rate: 0.2,
            })
            .expect("provider settlement should record");
        store
            .settle_node_inference(SettleParams {
                consumer_user_id: &user.id,
                provider_user_id: &user.id,
                node_id: "node-self",
                model_id: "pc-cli/codex",
                feature: "pc_agent_cli_dev",
                usage_mode: "pc_agent_cli",
                compute_call_id: Some("progression:req-self"),
                token_usage_event_id: Some("tok-progression-self"),
                billing_event_id: Some("bev-progression-self"),
                prompt_tokens: 7_000,
                completion_tokens: 3_000,
                price_per_1k_credits: 1.0,
                billed_cost_rmb_fen: 80,
                accounting_status: Some("billed"),
                provider_revenue_share_x1000: 800,
                platform_fee_rate: 0.2,
            })
            .expect("self settlement should record");

        let ledger = store
            .user_progression_ledger(&user.id)
            .expect("ledger should load");
        assert_eq!(ledger.consumed_tokens, 120_000);
        assert_eq!(ledger.consumed_call_count, 1);
        assert_eq!(ledger.platform_tokens, 120_000);
        assert_eq!(ledger.own_codex_tokens, 0);
        assert_eq!(ledger.shared_codex_tokens, 0);
        assert_eq!(ledger.provided_tokens, 12_500);
        assert_eq!(ledger.provided_run_count, 1);
        assert_eq!(ledger.provider_earned_fen, 80);

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
