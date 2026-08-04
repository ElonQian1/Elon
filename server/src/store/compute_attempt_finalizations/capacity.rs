use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use rusqlite::Connection;

use crate::compute_federation::{
    capacity::{
        ComputeCapacityAccount, ComputeCapacityClaimBinding, ComputeCapacityClaimEffectBinding,
        ComputeCapacityClaimState, ComputeCapacityEventKind, ComputeCapacityLedgerTransaction,
        ComputeCapacityMeterMode, ComputeCapacityMovementLine, ComputeCapacityOfferBinding,
        COMPUTE_CAPACITY_TRANSACTION_SCHEMA,
    },
    execution::ComputeReservedCapacity,
    receipts::ComputeMeterReading,
};

use super::super::{
    compute_capacity_claim_activation::claim_account_net_units_on,
    compute_capacity_claim_rows::{
        finalize_claim_digest, stored_claim_on, update_claim_projection_on,
    },
    compute_capacity_posting::{
        active_attempt_causal_transaction_on, event_kind_value, finalize_transaction_digest,
        next_ledger_sequence_on, post_capacity_transaction_on, reservation_capacity_causal_binding,
    },
    compute_capacity_rows::stored_bucket_on,
    new_id,
};
use super::{ComputeAttemptCapacityTransactionRef, ComputeAttemptFinalizationReceipt};

pub(super) struct FinalizeAttemptCapacityInput<'a> {
    pub claim_id: &'a str,
    pub expected_revision: i64,
    pub expected_digest: &'a str,
    pub offer: ComputeCapacityOfferBinding,
    pub job_id: &'a str,
    pub reservation_id: &'a str,
    pub attempt_lease_id: &'a str,
    pub fencing_generation: i64,
    pub activation_request_digest: &'a str,
    pub execution_receipt_id: &'a str,
    pub compensable_usage: &'a [ComputeMeterReading],
    pub finalization_request_digest: &'a str,
    pub idempotency_scope: &'a str,
    pub idempotency_key: &'a str,
    pub effective_at: &'a str,
}

pub(super) struct FinalizeAttemptCapacityReceipt {
    pub terminal_claim: ComputeCapacityClaimBinding,
    pub compensable_usage: Vec<ComputeReservedCapacity>,
    pub capacity_consumed: Vec<ComputeReservedCapacity>,
    pub capacity_returned: Vec<ComputeReservedCapacity>,
    pub transactions: Vec<ComputeAttemptCapacityTransactionRef>,
}

pub(super) fn finalize_attempt_capacity_on(
    conn: &Connection,
    input: FinalizeAttemptCapacityInput<'_>,
) -> Result<FinalizeAttemptCapacityReceipt> {
    let mut claim = stored_claim_on(conn, input.claim_id)?
        .ok_or_else(|| anyhow!("Attempt 可信终态引用的 Capacity Claim 不存在"))?;
    if claim.revision != input.expected_revision
        || claim.claim_digest != input.expected_digest
        || claim.state != ComputeCapacityClaimState::Active
    {
        bail!("Attempt 可信终态只能收口当前精确版本的 active Capacity Claim");
    }
    if claim.subject_kind != "compute_reservation" || claim.subject_id != input.reservation_id {
        bail!("Attempt 可信终态的 Capacity Claim 与 Reservation 身份不一致");
    }

    let mut causal =
        reservation_capacity_causal_binding(input.offer, input.job_id, input.reservation_id)?;
    causal.attempt_lease_id = Some(input.attempt_lease_id.to_string());
    causal.fencing_generation = Some(input.fencing_generation);
    let activation =
        active_attempt_causal_transaction_on(conn, &claim.claim_id, input.attempt_lease_id)?
            .ok_or_else(|| anyhow!("Attempt 可信终态缺少原始 attempt_activated 容量事务"))?;
    if activation.causal_binding != causal
        || activation.request_digest != input.activation_request_digest
        || activation.pool_id != claim.pool.pool_id
        || activation.capacity_epoch != claim.pool.capacity_epoch
        || activation.delivery_window_id != claim.delivery_window.window_id
    {
        bail!("Attempt 可信终态与原始 active 容量因果链不一致");
    }

    let usage = usage_by_meter(input.compensable_usage)?;
    if usage.len() != claim.lines.len() {
        bail!("Execution Receipt 的 compensable usage 必须覆盖 Claim 全部 meter");
    }

    let mut compensable_usage = Vec::with_capacity(claim.lines.len());
    let mut capacity_consumed = Vec::new();
    let mut capacity_returned = Vec::new();
    let mut consume_movements = Vec::new();
    let mut return_movements = Vec::new();
    for line in &claim.lines {
        let quantity = *usage
            .get(line.bucket.meter.as_str())
            .ok_or_else(|| anyhow!("Execution Receipt 缺少 Claim meter {}", line.bucket.meter))?;
        if quantity < 0
            || quantity > line.quantity_units
            || quantity % line.bucket.quantum_units != 0
        {
            bail!("compensable usage 超过预留容量或不符合 meter 量子");
        }
        let active =
            claim_account_net_units_on(conn, &claim.claim_id, &line.bucket.bucket_id, "active")?;
        let held =
            claim_account_net_units_on(conn, &claim.claim_id, &line.bucket.bucket_id, "held")?;
        if active != i128::from(line.quantity_units) || held != 0 {
            bail!("Attempt 可信终态前 Claim 的 active/held 归属与合同不一致");
        }
        let stored = stored_bucket_on(conn, &line.bucket.bucket_id)?
            .ok_or_else(|| anyhow!("Attempt 可信终态引用的 Capacity Bucket 不存在"))?;
        if stored.balance.binding != line.bucket {
            bail!("Attempt 可信终态前 Capacity Bucket 绑定发生变化");
        }

        compensable_usage.push(ComputeReservedCapacity {
            meter: line.bucket.meter.clone(),
            quantity,
        });
        let consumed = match line.bucket.meter_mode {
            ComputeCapacityMeterMode::Consumable => quantity,
            ComputeCapacityMeterMode::Reusable => 0,
        };
        let returned = line
            .quantity_units
            .checked_sub(consumed)
            .ok_or_else(|| anyhow!("容量收口数量下溢"))?;
        if consumed > 0 {
            capacity_consumed.push(ComputeReservedCapacity {
                meter: line.bucket.meter.clone(),
                quantity: consumed,
            });
            consume_movements.push(ComputeCapacityMovementLine {
                line_no: line.line_no,
                bucket: line.bucket.clone(),
                quantity_units: consumed,
                from_account: ComputeCapacityAccount::Active,
                to_account: ComputeCapacityAccount::Consumed,
            });
        }
        if returned > 0 {
            capacity_returned.push(ComputeReservedCapacity {
                meter: line.bucket.meter.clone(),
                quantity: returned,
            });
            return_movements.push(ComputeCapacityMovementLine {
                line_no: line.line_no,
                bucket: line.bucket.clone(),
                quantity_units: returned,
                from_account: ComputeCapacityAccount::Active,
                to_account: ComputeCapacityAccount::Available,
            });
        }
    }

    let mut transactions = Vec::with_capacity(2);
    let mut causal_transaction_id = activation.transaction_id;
    if !consume_movements.is_empty() {
        let transaction = post_phase_on(
            conn,
            &claim,
            &causal,
            CapacityPhase {
                event_kind: ComputeCapacityEventKind::UsageConsumed,
                claim_effect: "usage_consumed",
                idempotency_scope: &format!("{}:capacity_consumed", input.idempotency_scope),
                idempotency_key: input.idempotency_key,
                request_digest: input.finalization_request_digest,
                execution_receipt_id: input.execution_receipt_id,
                causal_transaction_id: &causal_transaction_id,
                movements: consume_movements,
                effective_at: input.effective_at,
            },
        )?;
        causal_transaction_id = transaction.transaction_id.clone();
        transactions.push(transaction);
    }
    if !return_movements.is_empty() {
        transactions.push(post_phase_on(
            conn,
            &claim,
            &causal,
            CapacityPhase {
                event_kind: ComputeCapacityEventKind::AttemptReturned,
                claim_effect: "unused_returned",
                idempotency_scope: &format!("{}:capacity_returned", input.idempotency_scope),
                idempotency_key: input.idempotency_key,
                request_digest: input.finalization_request_digest,
                execution_receipt_id: input.execution_receipt_id,
                causal_transaction_id: &causal_transaction_id,
                movements: return_movements,
                effective_at: input.effective_at,
            },
        )?);
    }
    if transactions.is_empty() {
        bail!("Attempt 可信终态没有形成任何容量收口事务");
    }

    for line in &claim.lines {
        let active =
            claim_account_net_units_on(conn, &claim.claim_id, &line.bucket.bucket_id, "active")?;
        if active != 0 {
            bail!("Attempt 可信终态后仍有 active 容量未收口");
        }
    }
    let previous_revision = claim.revision;
    let previous_state = claim.state;
    claim.state = ComputeCapacityClaimState::Consumed;
    claim.revision = claim
        .revision
        .checked_add(1)
        .ok_or_else(|| anyhow!("Capacity Claim revision 溢出"))?;
    claim.updated_at = input.effective_at.to_string();
    claim.terminal_at = Some(input.effective_at.to_string());
    finalize_claim_digest(&mut claim)?;
    update_claim_projection_on(conn, previous_revision, previous_state, &claim)?;

    Ok(FinalizeAttemptCapacityReceipt {
        terminal_claim: ComputeCapacityClaimBinding {
            claim_id: claim.claim_id,
            claim_revision: claim.revision,
            claim_digest: claim.claim_digest,
        },
        compensable_usage,
        capacity_consumed,
        capacity_returned,
        transactions,
    })
}

struct CapacityPhase<'a> {
    event_kind: ComputeCapacityEventKind,
    claim_effect: &'a str,
    idempotency_scope: &'a str,
    idempotency_key: &'a str,
    request_digest: &'a str,
    execution_receipt_id: &'a str,
    causal_transaction_id: &'a str,
    movements: Vec<ComputeCapacityMovementLine>,
    effective_at: &'a str,
}

fn post_phase_on(
    conn: &Connection,
    claim: &crate::compute_federation::capacity::ComputeCapacityClaim,
    causal: &crate::compute_federation::capacity::ComputeCapacityCausalBinding,
    phase: CapacityPhase<'_>,
) -> Result<ComputeAttemptCapacityTransactionRef> {
    let mut balances = BTreeMap::new();
    for movement in &phase.movements {
        let stored = stored_bucket_on(conn, &movement.bucket.bucket_id)?
            .ok_or_else(|| anyhow!("容量收口引用的 Capacity Bucket 不存在"))?;
        if stored.balance.binding != movement.bucket {
            bail!("容量收口 Capacity Bucket 绑定发生变化");
        }
        balances.insert(movement.bucket.bucket_id.clone(), stored.balance);
    }
    let mut transaction = ComputeCapacityLedgerTransaction {
        schema: COMPUTE_CAPACITY_TRANSACTION_SCHEMA.to_string(),
        transaction_id: new_id("capacity_tx"),
        transaction_digest: String::new(),
        pool: claim.pool.clone(),
        delivery_window: claim.delivery_window.clone(),
        ledger_sequence: next_ledger_sequence_on(conn, &claim.pool)?,
        event_kind: phase.event_kind,
        claim_effect: Some(ComputeCapacityClaimEffectBinding {
            claim_id: claim.claim_id.clone(),
            claim_effect: phase.claim_effect.to_string(),
            claim_effect_key: phase.execution_receipt_id.to_string(),
        }),
        causal_binding: causal.clone(),
        idempotency_scope: phase.idempotency_scope.to_string(),
        idempotency_key: phase.idempotency_key.to_string(),
        request_digest: phase.request_digest.to_string(),
        subject_kind: "compute_execution_receipt".to_string(),
        subject_id: phase.execution_receipt_id.to_string(),
        causal_transaction_id: Some(phase.causal_transaction_id.to_string()),
        movements: phase.movements,
        occurred_at: phase.effective_at.to_string(),
        recorded_at: phase.effective_at.to_string(),
    };
    finalize_transaction_digest(&mut transaction)?;
    post_capacity_transaction_on(conn, &transaction, balances)?;
    Ok(ComputeAttemptCapacityTransactionRef {
        transaction_id: transaction.transaction_id,
        transaction_digest: transaction.transaction_digest,
        ledger_sequence: transaction.ledger_sequence,
        event_kind: event_kind_value(transaction.event_kind).to_string(),
    })
}

fn usage_by_meter(readings: &[ComputeMeterReading]) -> Result<BTreeMap<&str, i64>> {
    let mut usage = BTreeMap::new();
    for reading in readings {
        if reading.meter.trim().is_empty()
            || reading.meter != reading.meter.trim()
            || reading.quantity < 0
            || usage
                .insert(reading.meter.as_str(), reading.quantity)
                .is_some()
        {
            bail!("Execution Receipt compensable usage meter、数量或来源无效");
        }
    }
    Ok(usage)
}

pub(super) fn capacity_effect(receipt: &FinalizeAttemptCapacityReceipt) -> &'static str {
    match (
        receipt.capacity_consumed.is_empty(),
        receipt.capacity_returned.is_empty(),
    ) {
        (false, false) => "verified_usage_consumed_and_unused_returned",
        (false, true) => "verified_usage_consumed",
        (true, false) => "active_capacity_returned",
        (true, true) => "unchanged",
    }
}

pub(super) fn receipt_capacity_is_consistent(receipt: &ComputeAttemptFinalizationReceipt) -> bool {
    !receipt.capacity_transactions.is_empty()
        && (!receipt.capacity_consumed.is_empty() || !receipt.capacity_returned.is_empty())
}
