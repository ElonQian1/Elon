use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::compute_federation::capacity::{
    ComputeCapacityAccount, ComputeCapacityClaimBinding, ComputeCapacityClaimEffectBinding,
    ComputeCapacityClaimState, ComputeCapacityEventKind, ComputeCapacityLedgerTransaction,
    ComputeCapacityMovementLine, ComputeCapacityOfferBinding, COMPUTE_CAPACITY_TRANSACTION_SCHEMA,
};

use super::{
    compute_capacity_claim_rows::{
        finalize_claim_digest, stored_claim_on, update_claim_projection_on,
    },
    compute_capacity_ledger::ComputeCapacityLedgerWriteReceipt,
    compute_capacity_posting::{
        event_kind_value, finalize_transaction_digest, held_claim_causal_transaction_on,
        next_ledger_sequence_on, post_capacity_transaction_on, reservation_capacity_causal_binding,
    },
    compute_capacity_rows::stored_bucket_on,
    new_id,
};

#[derive(Debug, Clone)]
pub(super) struct ActivateReservationCapacityClaim {
    pub claim_id: String,
    pub expected_revision: i64,
    pub expected_digest: String,
    pub offer: ComputeCapacityOfferBinding,
    pub job_id: String,
    pub reservation_id: String,
    pub attempt_lease_id: String,
    pub fencing_generation: i64,
    pub request_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub activated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ActivateReservationCapacityClaimReceipt {
    pub claim: ComputeCapacityClaimBinding,
    pub ledger: ComputeCapacityLedgerWriteReceipt,
}

pub(super) fn activate_reservation_capacity_claim_on(
    conn: &Connection,
    input: ActivateReservationCapacityClaim,
) -> Result<ActivateReservationCapacityClaimReceipt> {
    let mut claim = stored_claim_on(conn, input.claim_id.trim())?
        .ok_or_else(|| anyhow!("Attempt 激活引用的容量 Claim 不存在"))?;
    if claim.revision != input.expected_revision
        || claim.claim_digest != input.expected_digest.trim()
    {
        bail!("Attempt 激活只能基于 Capacity Claim 的当前精确版本");
    }
    if claim.state != ComputeCapacityClaimState::Held {
        bail!("Attempt 激活要求 Capacity Claim 当前处于 held 状态");
    }
    if claim.subject_kind != "compute_reservation"
        || claim.subject_id != input.reservation_id.trim()
    {
        bail!("Attempt 激活的 Capacity Claim 与 Reservation 身份不一致");
    }

    let expected_causal = reservation_capacity_causal_binding(
        input.offer,
        input.job_id.trim(),
        input.reservation_id.trim(),
    )?;
    let held =
        held_claim_causal_transaction_on(conn, &claim.claim_id, claim.idempotency_key.as_str())?
            .ok_or_else(|| anyhow!("Attempt 激活缺少原始 reservation_held 事务"))?;
    if held.causal_binding != expected_causal
        || held.request_digest != claim.request_digest
        || held.pool_id != claim.pool.pool_id
        || held.capacity_epoch != claim.pool.capacity_epoch
        || held.delivery_window_id != claim.delivery_window.window_id
        || held.subject_kind != claim.subject_kind
        || held.subject_id != claim.subject_id
    {
        bail!("Attempt 激活与原始 held 容量因果链不一致");
    }

    let mut balances = BTreeMap::new();
    let mut movements = Vec::with_capacity(claim.lines.len());
    for line in &claim.lines {
        let held_units =
            claim_account_net_units_on(conn, &claim.claim_id, &line.bucket.bucket_id, "held")?;
        let active_units =
            claim_account_net_units_on(conn, &claim.claim_id, &line.bucket.bucket_id, "active")?;
        if held_units != i128::from(line.quantity_units) || active_units != 0 {
            bail!("Attempt 激活前 Capacity Claim 的 held/active 归属与合同不一致");
        }
        let stored = stored_bucket_on(conn, &line.bucket.bucket_id)?
            .ok_or_else(|| anyhow!("Attempt 激活引用的 Capacity Bucket 不存在"))?;
        if stored.balance.binding != line.bucket {
            bail!("Attempt 激活前 Capacity Bucket 绑定发生变化");
        }
        balances.insert(line.bucket.bucket_id.clone(), stored.balance);
        movements.push(ComputeCapacityMovementLine {
            line_no: line.line_no,
            bucket: line.bucket.clone(),
            quantity_units: line.quantity_units,
            from_account: ComputeCapacityAccount::Held,
            to_account: ComputeCapacityAccount::Active,
        });
    }

    let mut causal_binding = expected_causal;
    causal_binding.attempt_lease_id = Some(input.attempt_lease_id.trim().to_string());
    causal_binding.fencing_generation = Some(input.fencing_generation);
    let mut transaction = ComputeCapacityLedgerTransaction {
        schema: COMPUTE_CAPACITY_TRANSACTION_SCHEMA.to_string(),
        transaction_id: new_id("capacity_tx"),
        transaction_digest: String::new(),
        pool: claim.pool.clone(),
        delivery_window: claim.delivery_window.clone(),
        ledger_sequence: next_ledger_sequence_on(conn, &claim.pool)?,
        event_kind: ComputeCapacityEventKind::AttemptActivated,
        claim_effect: Some(ComputeCapacityClaimEffectBinding {
            claim_id: claim.claim_id.clone(),
            claim_effect: "active".to_string(),
            claim_effect_key: input.attempt_lease_id.trim().to_string(),
        }),
        causal_binding,
        idempotency_scope: input.idempotency_scope,
        idempotency_key: input.idempotency_key,
        request_digest: input.request_digest,
        subject_kind: "compute_attempt_lease".to_string(),
        subject_id: input.attempt_lease_id.trim().to_string(),
        causal_transaction_id: Some(held.transaction_id),
        movements,
        occurred_at: input.activated_at.clone(),
        recorded_at: input.activated_at.clone(),
    };
    finalize_transaction_digest(&mut transaction)?;
    let current_balances = post_capacity_transaction_on(conn, &transaction, balances)?;

    let previous_revision = claim.revision;
    let previous_state = claim.state;
    claim.state = ComputeCapacityClaimState::Active;
    claim.revision = claim
        .revision
        .checked_add(1)
        .ok_or_else(|| anyhow!("Capacity Claim revision 溢出"))?;
    claim.updated_at = input.activated_at;
    finalize_claim_digest(&mut claim)?;
    update_claim_projection_on(conn, previous_revision, previous_state, &claim)?;

    Ok(ActivateReservationCapacityClaimReceipt {
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

fn claim_account_net_units_on(
    conn: &Connection,
    claim_id: &str,
    bucket_id: &str,
    account: &str,
) -> Result<i128> {
    let mut statement = conn.prepare(
        "SELECT l.delta_units
           FROM compute_capacity_ledger_legs l
           JOIN compute_capacity_ledger_transactions t
             ON t.transaction_id=l.transaction_id
          WHERE t.claim_id=?1 AND l.bucket_id=?2 AND l.account=?3
          ORDER BY t.ledger_sequence, l.line_no, l.leg_role",
    )?;
    let rows = statement.query_map(params![claim_id, bucket_id, account], |row| {
        row.get::<_, i64>(0)
    })?;
    let mut total = 0_i128;
    for row in rows {
        total = total
            .checked_add(i128::from(row?))
            .ok_or_else(|| anyhow!("Capacity Claim 归属余额溢出"))?;
    }
    Ok(total)
}
