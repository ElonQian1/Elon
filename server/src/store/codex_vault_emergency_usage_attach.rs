//! Exact-run audit attachment for shared Codex usage.
//!
//! Lease status is intentionally not part of the proof: a completion may arrive
//! after expiry, revoke, clear, or supersede. Every immutable execution and
//! accounting edge must still match before such a late completion is attached.

use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use super::{
    common::{clean_optional, new_id, now},
    Store,
};

impl Store {
    /// Attach shared-Codex usage to the exact run that consumed the lease.
    ///
    /// This is deliberately stricter than checking whether the lease is still
    /// active. It accepts a late completion for an inactive lease only after a
    /// single transaction proves the lease, run, token event, node settlement,
    /// and frozen billing allowance are the same bounded execution.
    #[allow(clippy::too_many_arguments)]
    pub fn attach_codex_vault_emergency_usage_strict(
        &self,
        lease_id: &str,
        compute_call_id: &str,
        token_usage_event_id: &str,
        billing_event_id: Option<&str>,
        node_transaction_id: &str,
        input_tokens: i64,
        output_tokens: i64,
        billed_cost_rmb_fen: i64,
        provider_earned_fen: i64,
        accounting_status: &str,
    ) -> Result<()> {
        let lease_id = required_value("lease_id", lease_id)?;
        let compute_call_id = required_value("compute_call_id", compute_call_id)?;
        let token_usage_event_id = required_value("token_usage_event_id", token_usage_event_id)?;
        let node_transaction_id = required_value("node_transaction_id", node_transaction_id)?;
        let accounting_status = required_value("accounting_status", accounting_status)?;
        let billing_event_id = clean_optional(billing_event_id);
        if input_tokens < 0
            || output_tokens < 0
            || billed_cost_rmb_fen < 0
            || provider_earned_fen < 0
            || provider_earned_fen > billed_cost_rmb_fen
        {
            bail!("共享 Codex 用量金额或 token 数无效");
        }
        let total_tokens = input_tokens
            .checked_add(output_tokens)
            .ok_or_else(|| anyhow::anyhow!("共享 Codex token 总数溢出"))?;

        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let proof_matches = tx.query_row(
            r#"SELECT EXISTS(
                 SELECT 1
                   FROM codex_vault_emergency_leases AS lease
                   JOIN node_compute_runs AS run
                     ON run.compute_call_id = ?2
                   JOIN token_usage_events AS usage
                     ON usage.id = ?3
                   JOIN node_transactions AS node_tx
                     ON node_tx.id = ?5
                   JOIN billing_reservations AS allowance
                     ON allowance.id = run.allowance_id
                  WHERE lease.id = ?1
                    AND lease.billing_source = 'shared_codex'
                    AND run.lease_id = lease.id
                    AND run.billing_source = 'shared_codex'
                    AND run.usage_mode = 'pc_agent_cli'
                    AND run.consumer_user_id = lease.consumer_user_id
                    AND run.resource_owner_user_id = lease.provider_user_id
                    AND run.node_id = lease.consumer_node_id
                    AND run.allowance_id IS NOT NULL
                    AND run.max_cost_rmb_fen >= 0
                    AND run.model_id IS NOT NULL
                    AND usage.user_id = run.consumer_user_id
                    AND usage.idempotency_key = run.compute_call_id
                    AND usage.billing_source = 'shared_codex'
                    AND usage.resource_owner_user_id = run.resource_owner_user_id
                    AND usage.feature = run.feature
                    AND usage.usage_mode = run.usage_mode
                    AND usage.model = run.model_id
                    AND usage.input_tokens = ?6
                    AND usage.output_tokens = ?7
                    AND usage.total_tokens = ?8
                    AND usage.cost_rmb_fen = ?9
                    AND usage.billing_event_id IS ?4
                    AND usage.accounting_status = ?11
                    AND node_tx.compute_call_id = run.compute_call_id
                    AND node_tx.token_usage_event_id = usage.id
                    AND node_tx.billing_event_id IS ?4
                    AND node_tx.consumer_user_id = lease.consumer_user_id
                    AND node_tx.provider_user_id = lease.provider_user_id
                    AND node_tx.node_id = lease.consumer_node_id
                    AND node_tx.feature = run.feature
                    AND node_tx.usage_mode = run.usage_mode
                    AND node_tx.model_id = run.model_id
                    AND node_tx.prompt_tokens = ?6
                    AND node_tx.completion_tokens = ?7
                    AND node_tx.billed_cost_rmb_fen = ?9
                    AND node_tx.provider_earned_fen = ?10
                    AND node_tx.provider_earned_fen <= node_tx.billed_cost_rmb_fen
                    AND node_tx.settlement_status = ?11
                    AND allowance.user_id = run.consumer_user_id
                    AND allowance.compute_call_id = run.compute_call_id
                    AND allowance.feature = run.feature
                    AND allowance.usage_mode = run.usage_mode
                    AND allowance.model = run.model_id
                    AND allowance.status = 'settled'
                    AND allowance.token_usage_event_id = usage.id
                    AND allowance.billing_event_id IS ?4
                    AND allowance.reserved_fen = run.max_cost_rmb_fen
                    AND allowance.reserved_fen >= 0
                    AND allowance.settled_cost_fen = usage.cost_rmb_fen
                    AND allowance.settled_cost_fen = node_tx.billed_cost_rmb_fen
                    AND allowance.settled_cost_fen >= 0
                    AND allowance.settled_cost_fen <= allowance.reserved_fen
                    AND allowance.refunded_fen >= 0
                    AND allowance.refunded_fen =
                        allowance.reserved_fen - allowance.settled_cost_fen
                    AND (
                        ?4 IS NULL
                        OR EXISTS(
                            SELECT 1
                              FROM billing_events AS billing
                             WHERE billing.id = ?4
                               AND billing.user_id = usage.user_id
                               AND billing.token_usage_event_id = usage.id
                               AND billing.model = run.model_id
                               AND billing.input_tokens = ?6
                               AND billing.output_tokens = ?7
                               AND billing.cost_rmb_fen = usage.cost_rmb_fen
                        )
                    )
             )"#,
            params![
                lease_id,
                compute_call_id,
                token_usage_event_id,
                billing_event_id.as_deref(),
                node_transaction_id,
                input_tokens,
                output_tokens,
                total_tokens,
                billed_cost_rmb_fen,
                provider_earned_fen,
                accounting_status,
            ],
            |row| row.get::<_, bool>(0),
        )?;
        if !proof_matches {
            bail!("共享 Codex 用量与 exact run、结算流水或冻结 allowance 不一致");
        }

        let (existing_count, exact_count): (i64, i64) = tx.query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(CASE
                      WHEN lease_id = ?1
                       AND billing_event_id IS ?3
                       AND node_transaction_id = ?4
                       AND input_tokens = ?5
                       AND output_tokens = ?6
                       AND total_tokens = ?7
                       AND billed_cost_rmb_fen = ?8
                       AND provider_earned_fen = ?9
                       AND accounting_status = ?10
                      THEN 1 ELSE 0 END), 0)
               FROM codex_vault_emergency_lease_usage_events
              WHERE token_usage_event_id = ?2",
            params![
                lease_id,
                token_usage_event_id,
                billing_event_id.as_deref(),
                node_transaction_id,
                input_tokens,
                output_tokens,
                total_tokens,
                billed_cost_rmb_fen,
                provider_earned_fen,
                accounting_status,
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if existing_count > 0 {
            if existing_count == 1 && exact_count == 1 {
                tx.commit()?;
                return Ok(());
            }
            bail!("共享 Codex token 用量已绑定到冲突的租约或结算流水");
        }

        let ts = now();
        tx.execute(
            "INSERT INTO codex_vault_emergency_lease_usage_events
             (id, lease_id, token_usage_event_id, billing_event_id, node_transaction_id,
              input_tokens, output_tokens, total_tokens, billed_cost_rmb_fen,
              provider_earned_fen, accounting_status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                new_id("cvlu"),
                lease_id,
                token_usage_event_id,
                billing_event_id.as_deref(),
                node_transaction_id,
                input_tokens,
                output_tokens,
                total_tokens,
                billed_cost_rmb_fen,
                provider_earned_fen,
                accounting_status,
                ts,
            ],
        )?;
        let changed = tx.execute(
            "UPDATE codex_vault_emergency_leases
                SET token_usage_event_id = ?2,
                    billing_event_id = ?3,
                    node_transaction_id = ?4,
                    input_tokens = input_tokens + ?5,
                    output_tokens = output_tokens + ?6,
                    total_tokens = total_tokens + ?7,
                    billed_cost_rmb_fen = billed_cost_rmb_fen + ?8,
                    provider_earned_fen = provider_earned_fen + ?9,
                    accounting_status = ?10,
                    updated_at = ?11
              WHERE id = ?1",
            params![
                lease_id,
                token_usage_event_id,
                billing_event_id.as_deref(),
                node_transaction_id,
                input_tokens,
                output_tokens,
                total_tokens,
                billed_cost_rmb_fen,
                provider_earned_fen,
                accounting_status,
                ts,
            ],
        )?;
        if changed != 1 {
            bail!("共享 Codex 租约在用量归档前消失");
        }
        tx.commit()?;
        Ok(())
    }
}

fn required_value<'a>(field: &str, value: &'a str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() || value.len() > 200 || value.chars().any(char::is_control) {
        bail!("{field} 无效");
    }
    Ok(value)
}

#[cfg(test)]
#[path = "codex_vault_emergency_usage_attach_tests.rs"]
mod tests;
