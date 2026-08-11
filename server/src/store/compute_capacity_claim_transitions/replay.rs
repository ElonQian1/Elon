use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    terminal_parts, validate_finish_causal_binding, validate_finish_claim_binding,
    validate_original_held_transaction, ExpectedFinishBinding, FinishComputeCapacityClaim,
    FinishComputeCapacityClaimReceipt,
};
use crate::store::{
    compute_capacity_claim_rows::{claim_state_value, stored_claim_on},
    compute_capacity_ledger::ComputeCapacityLedgerWriteReceipt,
    compute_capacity_posting::{
        balances_for_transaction_on, capacity_causal_transaction_on, event_kind_value,
        held_claim_causal_transaction_on,
    },
};

pub(super) struct ExistingTransition {
    transaction_id: String,
    transaction_digest: String,
    ledger_sequence: i64,
    event_kind: String,
    claim_id: String,
    claim_effect: String,
    request_digest: String,
    causal_transaction_id: Option<String>,
    recorded_at: String,
}

pub(super) fn read_existing_transition_on(
    conn: &Connection,
    idempotency_scope: &str,
    idempotency_key: &str,
) -> Result<Option<ExistingTransition>> {
    conn.query_row(
        "SELECT transaction_id, transaction_digest, ledger_sequence,
                event_kind, claim_id, claim_effect, request_digest,
                causal_transaction_id, recorded_at
           FROM compute_capacity_ledger_transactions
          WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![idempotency_scope, idempotency_key],
        |row| {
            Ok(ExistingTransition {
                transaction_id: row.get(0)?,
                transaction_digest: row.get(1)?,
                ledger_sequence: row.get(2)?,
                event_kind: row.get(3)?,
                claim_id: row.get(4)?,
                claim_effect: row.get(5)?,
                request_digest: row.get(6)?,
                causal_transaction_id: row.get(7)?,
                recorded_at: row.get(8)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn replay_existing_transition_on(
    conn: &Connection,
    input: &FinishComputeCapacityClaim,
    request_digest: &str,
    expected: Option<&ExpectedFinishBinding>,
    existing: ExistingTransition,
) -> Result<FinishComputeCapacityClaimReceipt> {
    let (expected_event, expected_state, expected_effect) = terminal_parts(input.action);
    if existing.claim_id != input.claim_id.trim()
        || existing.event_kind != event_kind_value(expected_event)
        || existing.claim_effect != expected_effect
        || existing.request_digest != request_digest
    {
        bail!("相同容量 Claim 终态幂等键不能用于不同请求");
    }
    let claim = stored_claim_on(conn, &existing.claim_id)?
        .ok_or_else(|| anyhow!("容量 Claim 终态重放时原 Claim 不存在"))?;
    validate_finish_claim_binding(&claim, expected)?;
    let original_held =
        held_claim_causal_transaction_on(conn, &claim.claim_id, claim.idempotency_key.as_str())?
            .ok_or_else(|| anyhow!("容量 Claim 终态重放时缺少原始 held 事务"))?;
    validate_original_held_transaction(&claim, &original_held)?;
    if existing.causal_transaction_id.as_deref() != Some(original_held.transaction_id.as_str()) {
        bail!("容量 Claim 终态事务没有引用原始 held 因果前序");
    }
    let terminal_causal = capacity_causal_transaction_on(conn, &existing.transaction_id)?;
    if terminal_causal.causal_binding != original_held.causal_binding {
        bail!("容量 Claim 终态事务未继承原始 held 业务因果绑定");
    }
    validate_finish_causal_binding(&original_held.causal_binding, expected)?;
    let expected_terminal_revision = input
        .expected_revision
        .checked_add(1)
        .ok_or_else(|| anyhow!("容量 Claim expected_revision 溢出"))?;
    if claim.state != expected_state || claim.revision != expected_terminal_revision {
        bail!("容量 Claim 终态重放与当前 Claim 状态不一致");
    }
    let current_balances = balances_for_transaction_on(conn, &existing.transaction_id)?;
    if current_balances.iter().any(|balance| {
        balance.binding.pool != claim.pool
            || balance.binding.delivery_window != claim.delivery_window
    }) {
        bail!("容量 Claim 终态重放的资源绑定不一致");
    }
    Ok(FinishComputeCapacityClaimReceipt {
        claim_id: claim.claim_id,
        claim_digest: claim.claim_digest,
        state: claim_state_value(claim.state).to_string(),
        revision: claim.revision,
        request_digest: existing.request_digest.clone(),
        recorded_at: existing.recorded_at,
        replayed: true,
        ledger: ComputeCapacityLedgerWriteReceipt {
            transaction_id: existing.transaction_id,
            transaction_digest: existing.transaction_digest,
            ledger_sequence: existing.ledger_sequence,
            event_kind: existing.event_kind,
            request_digest: existing.request_digest,
            replayed: true,
            current_balances,
        },
    })
}
