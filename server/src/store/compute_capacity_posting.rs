use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

use crate::compute_federation::capacity::{
    apply_capacity_transaction, expand_capacity_ledger_legs, ComputeCapacityAccount,
    ComputeCapacityBucketBalance, ComputeCapacityEventKind, ComputeCapacityLedgerTransaction,
    ComputeCapacityLegRole, ComputeCapacityPoolBinding,
};

use super::{compute_capacity_rows::stored_bucket_on, new_id};

pub(super) fn next_ledger_sequence_on(
    conn: &Connection,
    pool: &ComputeCapacityPoolBinding,
) -> Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(ledger_sequence), 0) + 1
           FROM compute_capacity_ledger_transactions
          WHERE pool_id=?1 AND capacity_epoch=?2",
        params![pool.pool_id.trim(), pool.capacity_epoch],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub(super) fn finalize_transaction_digest(
    transaction: &mut ComputeCapacityLedgerTransaction,
) -> Result<()> {
    let payload = serde_json::json!({
        "schema": transaction.schema,
        "transaction_id": transaction.transaction_id,
        "pool": transaction.pool,
        "delivery_window": transaction.delivery_window,
        "ledger_sequence": transaction.ledger_sequence,
        "event_kind": transaction.event_kind,
        "claim_effect": transaction.claim_effect,
        "causal_binding": transaction.causal_binding,
        "idempotency_scope": transaction.idempotency_scope,
        "idempotency_key": transaction.idempotency_key,
        "request_digest": transaction.request_digest,
        "subject_kind": transaction.subject_kind,
        "subject_id": transaction.subject_id,
        "causal_transaction_id": transaction.causal_transaction_id,
        "movements": transaction.movements,
        "occurred_at": transaction.occurred_at,
        "recorded_at": transaction.recorded_at,
    });
    transaction.transaction_digest = hex::encode(Sha256::digest(serde_json::to_vec(&payload)?));
    Ok(())
}

pub(super) fn post_capacity_transaction_on(
    conn: &Connection,
    transaction: &ComputeCapacityLedgerTransaction,
    mut balances: BTreeMap<String, ComputeCapacityBucketBalance>,
) -> Result<Vec<ComputeCapacityBucketBalance>> {
    let before = balances.clone();
    let legs = expand_capacity_ledger_legs(transaction).map_err(anyhow::Error::new)?;
    apply_capacity_transaction(&mut balances, transaction).map_err(anyhow::Error::new)?;

    let claim_effect = transaction.claim_effect.as_ref();
    let offer = transaction.causal_binding.offer.as_ref();
    conn.execute(
        "INSERT INTO compute_capacity_ledger_transactions (
            transaction_id, transaction_digest, pool_id, capacity_epoch,
            delivery_window_id, ledger_sequence, event_kind,
            claim_id, claim_effect, claim_effect_key,
            offer_id, offer_version, offer_digest, job_id, reservation_id,
            attempt_lease_id, fencing_generation, idempotency_scope,
            idempotency_key, request_digest, subject_kind, subject_id,
            causal_transaction_id, occurred_at, recorded_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25
         )",
        params![
            transaction.transaction_id,
            transaction.transaction_digest,
            transaction.pool.pool_id,
            transaction.pool.capacity_epoch,
            transaction.delivery_window.window_id,
            transaction.ledger_sequence,
            event_kind_value(transaction.event_kind),
            claim_effect.map(|binding| binding.claim_id.as_str()),
            claim_effect.map(|binding| binding.claim_effect.as_str()),
            claim_effect.map(|binding| binding.claim_effect_key.as_str()),
            offer.map(|binding| binding.offer_id.as_str()),
            offer.map(|binding| binding.offer_version),
            offer.map(|binding| binding.offer_digest.as_str()),
            transaction.causal_binding.job_id.as_deref(),
            transaction.causal_binding.reservation_id.as_deref(),
            transaction.causal_binding.attempt_lease_id.as_deref(),
            transaction.causal_binding.fencing_generation,
            transaction.idempotency_scope,
            transaction.idempotency_key,
            transaction.request_digest,
            transaction.subject_kind,
            transaction.subject_id,
            transaction.causal_transaction_id,
            transaction.occurred_at,
            transaction.recorded_at,
        ],
    )?;

    for leg in legs {
        conn.execute(
            "INSERT INTO compute_capacity_ledger_legs (
                leg_id, transaction_id, line_no, leg_role, bucket_id,
                meter, account, delta_units, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                new_id("capacity_leg"),
                transaction.transaction_id,
                leg.line_no,
                leg_role_value(leg.leg_role),
                leg.bucket.bucket_id,
                leg.bucket.meter,
                account_value(leg.account),
                leg.delta_units,
                transaction.recorded_at,
            ],
        )?;
    }

    for (bucket_id, balance) in &balances {
        let prior = before
            .get(bucket_id)
            .ok_or_else(|| anyhow!("容量 bucket 更新缺少原始余额"))?;
        let changed = conn.execute(
            "UPDATE compute_capacity_buckets SET
                issued_units=?1, available_units=?2, held_units=?3,
                active_units=?4, consumed_units=?5, retired_units=?6,
                balance_revision=?7, through_ledger_sequence=?8, updated_at=?9
              WHERE bucket_id=?10 AND balance_revision=?11",
            params![
                balance.issued_units,
                balance.available_units,
                balance.held_units,
                balance.active_units,
                balance.consumed_units,
                balance.retired_units,
                balance.balance_revision,
                balance.through_ledger_sequence,
                transaction.recorded_at,
                bucket_id,
                prior.balance_revision,
            ],
        )?;
        if changed != 1 {
            bail!("容量 bucket 余额版本已变化，账本事务未提交");
        }
    }

    Ok(balances.into_values().collect())
}

pub(super) fn balances_for_transaction_on(
    conn: &Connection,
    transaction_id: &str,
) -> Result<Vec<ComputeCapacityBucketBalance>> {
    let mut statement = conn.prepare(
        "SELECT DISTINCT bucket_id FROM compute_capacity_ledger_legs
          WHERE transaction_id=?1 ORDER BY bucket_id",
    )?;
    let bucket_ids = statement
        .query_map(params![transaction_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    bucket_ids
        .into_iter()
        .map(|bucket_id| {
            stored_bucket_on(conn, &bucket_id)?
                .map(|stored| stored.balance)
                .ok_or_else(|| anyhow!("容量账本引用的 bucket 不存在"))
        })
        .collect()
}

pub(super) fn event_kind_value(event_kind: ComputeCapacityEventKind) -> &'static str {
    match event_kind {
        ComputeCapacityEventKind::SupplyAdded => "supply_added",
        ComputeCapacityEventKind::SupplyWithdrawn => "supply_withdrawn",
        ComputeCapacityEventKind::ReservationHeld => "reservation_held",
        ComputeCapacityEventKind::AttemptActivated => "attempt_activated",
        ComputeCapacityEventKind::AttemptReturned => "attempt_returned",
        ComputeCapacityEventKind::UsageConsumed => "usage_consumed",
        ComputeCapacityEventKind::ReservationReleased => "reservation_released",
        ComputeCapacityEventKind::ReservationExpired => "reservation_expired",
    }
}

fn leg_role_value(role: ComputeCapacityLegRole) -> &'static str {
    match role {
        ComputeCapacityLegRole::From => "from",
        ComputeCapacityLegRole::To => "to",
    }
}

fn account_value(account: ComputeCapacityAccount) -> &'static str {
    match account {
        ComputeCapacityAccount::Issuance => "issuance",
        ComputeCapacityAccount::Available => "available",
        ComputeCapacityAccount::Held => "held",
        ComputeCapacityAccount::Active => "active",
        ComputeCapacityAccount::Consumed => "consumed",
        ComputeCapacityAccount::Retired => "retired",
    }
}
