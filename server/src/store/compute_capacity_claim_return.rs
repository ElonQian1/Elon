use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use rusqlite::Connection;
use serde::Serialize;

use crate::compute_federation::capacity::{
    ComputeCapacityAccount, ComputeCapacityClaimBinding, ComputeCapacityClaimEffectBinding,
    ComputeCapacityClaimState, ComputeCapacityEventKind, ComputeCapacityLedgerTransaction,
    ComputeCapacityMovementLine, ComputeCapacityOfferBinding, COMPUTE_CAPACITY_TRANSACTION_SCHEMA,
};

use super::{
    compute_capacity_claim_activation::claim_account_net_units_on,
    compute_capacity_claim_rows::{
        finalize_claim_digest, stored_claim_on, update_claim_projection_on,
    },
    compute_capacity_ledger::ComputeCapacityLedgerWriteReceipt,
    compute_capacity_posting::{
        active_attempt_causal_transaction_on, event_kind_value, finalize_transaction_digest,
        next_ledger_sequence_on, post_capacity_transaction_on, reservation_capacity_causal_binding,
    },
    compute_capacity_rows::stored_bucket_on,
    new_id,
};

#[derive(Debug, Clone)]
pub(super) struct ReturnAttemptCapacityClaim {
    pub claim_id: String,
    pub expected_revision: i64,
    pub expected_digest: String,
    pub offer: ComputeCapacityOfferBinding,
    pub job_id: String,
    pub reservation_id: String,
    pub attempt_lease_id: String,
    pub fencing_generation: i64,
    pub activation_request_digest: String,
    pub abort_request_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub returned_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ReturnAttemptCapacityClaimReceipt {
    pub claim: ComputeCapacityClaimBinding,
    pub ledger: ComputeCapacityLedgerWriteReceipt,
}

pub(super) fn return_attempt_capacity_claim_on(
    conn: &Connection,
    input: ReturnAttemptCapacityClaim,
) -> Result<ReturnAttemptCapacityClaimReceipt> {
    let mut claim = stored_claim_on(conn, input.claim_id.trim())?
        .ok_or_else(|| anyhow!("Attempt 中止引用的容量 Claim 不存在"))?;
    if claim.revision != input.expected_revision
        || claim.claim_digest != input.expected_digest.trim()
        || claim.state != ComputeCapacityClaimState::Active
    {
        bail!("Attempt 中止只能归还当前精确版本的 active Capacity Claim");
    }
    if claim.subject_kind != "compute_reservation"
        || claim.subject_id != input.reservation_id.trim()
    {
        bail!("Attempt 中止的 Capacity Claim 与 Reservation 身份不一致");
    }

    let mut expected_causal = reservation_capacity_causal_binding(
        input.offer,
        input.job_id.trim(),
        input.reservation_id.trim(),
    )?;
    expected_causal.attempt_lease_id = Some(input.attempt_lease_id.trim().to_string());
    expected_causal.fencing_generation = Some(input.fencing_generation);
    let activation =
        active_attempt_causal_transaction_on(conn, &claim.claim_id, input.attempt_lease_id.trim())?
            .ok_or_else(|| anyhow!("Attempt 中止缺少原始 attempt_activated 事务"))?;
    if activation.causal_binding != expected_causal
        || activation.request_digest != input.activation_request_digest.trim()
        || activation.pool_id != claim.pool.pool_id
        || activation.capacity_epoch != claim.pool.capacity_epoch
        || activation.delivery_window_id != claim.delivery_window.window_id
        || activation.subject_kind != "compute_attempt_lease"
        || activation.subject_id != input.attempt_lease_id.trim()
    {
        bail!("Attempt 中止与原始 active 容量因果链不一致");
    }

    let mut balances = BTreeMap::new();
    let mut movements = Vec::with_capacity(claim.lines.len());
    for line in &claim.lines {
        let active_units =
            claim_account_net_units_on(conn, &claim.claim_id, &line.bucket.bucket_id, "active")?;
        let held_units =
            claim_account_net_units_on(conn, &claim.claim_id, &line.bucket.bucket_id, "held")?;
        if active_units != i128::from(line.quantity_units) || held_units != 0 {
            bail!("Attempt 中止前 Capacity Claim 的 active/held 归属与合同不一致");
        }
        let stored = stored_bucket_on(conn, &line.bucket.bucket_id)?
            .ok_or_else(|| anyhow!("Attempt 中止引用的 Capacity Bucket 不存在"))?;
        if stored.balance.binding != line.bucket {
            bail!("Attempt 中止前 Capacity Bucket 绑定发生变化");
        }
        balances.insert(line.bucket.bucket_id.clone(), stored.balance);
        movements.push(ComputeCapacityMovementLine {
            line_no: line.line_no,
            bucket: line.bucket.clone(),
            quantity_units: line.quantity_units,
            from_account: ComputeCapacityAccount::Active,
            to_account: ComputeCapacityAccount::Available,
        });
    }

    let mut transaction = ComputeCapacityLedgerTransaction {
        schema: COMPUTE_CAPACITY_TRANSACTION_SCHEMA.to_string(),
        transaction_id: new_id("capacity_tx"),
        transaction_digest: String::new(),
        pool: claim.pool.clone(),
        delivery_window: claim.delivery_window.clone(),
        ledger_sequence: next_ledger_sequence_on(conn, &claim.pool)?,
        event_kind: ComputeCapacityEventKind::AttemptReturned,
        claim_effect: Some(ComputeCapacityClaimEffectBinding {
            claim_id: claim.claim_id.clone(),
            claim_effect: "released".to_string(),
            claim_effect_key: input.idempotency_key.trim().to_string(),
        }),
        causal_binding: expected_causal,
        idempotency_scope: input.idempotency_scope,
        idempotency_key: input.idempotency_key,
        request_digest: input.abort_request_digest,
        subject_kind: "compute_attempt_lease".to_string(),
        subject_id: input.attempt_lease_id.trim().to_string(),
        causal_transaction_id: Some(activation.transaction_id),
        movements,
        occurred_at: input.returned_at.clone(),
        recorded_at: input.returned_at.clone(),
    };
    finalize_transaction_digest(&mut transaction)?;
    let current_balances = post_capacity_transaction_on(conn, &transaction, balances)?;

    let previous_revision = claim.revision;
    let previous_state = claim.state;
    claim.state = ComputeCapacityClaimState::Released;
    claim.revision = claim
        .revision
        .checked_add(1)
        .ok_or_else(|| anyhow!("Capacity Claim revision 溢出"))?;
    claim.updated_at = input.returned_at.clone();
    claim.terminal_at = Some(input.returned_at);
    finalize_claim_digest(&mut claim)?;
    update_claim_projection_on(conn, previous_revision, previous_state, &claim)?;

    Ok(ReturnAttemptCapacityClaimReceipt {
        claim: ComputeCapacityClaimBinding {
            claim_id: claim.claim_id,
            claim_revision: claim.revision,
            claim_digest: claim.claim_digest,
        },
        ledger: ComputeCapacityLedgerWriteReceipt {
            transaction_id: transaction.transaction_id,
            transaction_digest: transaction.transaction_digest,
            ledger_sequence: transaction.ledger_sequence,
            event_kind: event_kind_value(transaction.event_kind).to_string(),
            request_digest: transaction.request_digest,
            replayed: false,
            current_balances,
        },
    })
}
