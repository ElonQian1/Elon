use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::compute_federation::capacity::{
    apply_capacity_transaction, expand_capacity_ledger_legs, ComputeCapacityAccount,
    ComputeCapacityBucketBalance, ComputeCapacityCausalBinding, ComputeCapacityEventKind,
    ComputeCapacityLedgerTransaction, ComputeCapacityLegRole, ComputeCapacityOfferBinding,
    ComputeCapacityPoolBinding,
};

use super::{compute_capacity_rows::stored_bucket_on, new_id};

#[derive(Debug, Clone)]
pub(super) struct StoredCapacityCausalTransaction {
    pub transaction_id: String,
    pub causal_binding: ComputeCapacityCausalBinding,
    pub request_digest: String,
    pub pool_id: String,
    pub capacity_epoch: i64,
    pub delivery_window_id: String,
    pub subject_kind: String,
    pub subject_id: String,
}

struct StoredCapacityCausalRow {
    offer_id: Option<String>,
    offer_version: Option<i64>,
    offer_digest: Option<String>,
    job_id: Option<String>,
    reservation_id: Option<String>,
    attempt_lease_id: Option<String>,
    fencing_generation: Option<i64>,
    request_digest: String,
    pool_id: String,
    capacity_epoch: i64,
    delivery_window_id: String,
    subject_kind: String,
    subject_id: String,
}

pub(super) fn reservation_capacity_causal_binding(
    offer: ComputeCapacityOfferBinding,
    job_id: &str,
    reservation_id: &str,
) -> Result<ComputeCapacityCausalBinding> {
    let offer = ComputeCapacityOfferBinding {
        offer_id: offer.offer_id.trim().to_string(),
        offer_version: offer.offer_version,
        offer_digest: offer.offer_digest.trim().to_string(),
    };
    let job_id = job_id.trim().to_string();
    let reservation_id = reservation_id.trim().to_string();
    if offer.offer_id.is_empty()
        || offer.offer_version <= 0
        || offer.offer_digest.is_empty()
        || job_id.is_empty()
        || reservation_id.is_empty()
    {
        bail!("Reservation-bound 容量事务的 Offer、Job 或 Reservation 绑定无效");
    }
    Ok(ComputeCapacityCausalBinding {
        offer: Some(offer),
        job_id: Some(job_id),
        reservation_id: Some(reservation_id),
        attempt_lease_id: None,
        fencing_generation: None,
    })
}

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

pub(super) fn capacity_causal_transaction_on(
    conn: &Connection,
    transaction_id: &str,
) -> Result<StoredCapacityCausalTransaction> {
    let stored = conn
        .query_row(
            "SELECT offer_id, offer_version, offer_digest, job_id,
                    reservation_id, attempt_lease_id, fencing_generation,
                    request_digest, pool_id, capacity_epoch,
                    delivery_window_id, subject_kind, subject_id
               FROM compute_capacity_ledger_transactions
              WHERE transaction_id=?1",
            params![transaction_id.trim()],
            |row| {
                Ok(StoredCapacityCausalRow {
                    offer_id: row.get(0)?,
                    offer_version: row.get(1)?,
                    offer_digest: row.get(2)?,
                    job_id: row.get(3)?,
                    reservation_id: row.get(4)?,
                    attempt_lease_id: row.get(5)?,
                    fencing_generation: row.get(6)?,
                    request_digest: row.get(7)?,
                    pool_id: row.get(8)?,
                    capacity_epoch: row.get(9)?,
                    delivery_window_id: row.get(10)?,
                    subject_kind: row.get(11)?,
                    subject_id: row.get(12)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("容量账本因果事务不存在"))?;
    let offer = match (stored.offer_id, stored.offer_version, stored.offer_digest) {
        (None, None, None) => None,
        (Some(offer_id), Some(offer_version), Some(offer_digest)) => {
            Some(ComputeCapacityOfferBinding {
                offer_id,
                offer_version,
                offer_digest,
            })
        }
        _ => bail!("容量账本 Offer 因果绑定列不完整"),
    };
    let causal_binding = ComputeCapacityCausalBinding {
        offer,
        job_id: stored.job_id,
        reservation_id: stored.reservation_id,
        attempt_lease_id: stored.attempt_lease_id,
        fencing_generation: stored.fencing_generation,
    };
    validate_stored_causal_binding(&causal_binding)?;
    Ok(StoredCapacityCausalTransaction {
        transaction_id: transaction_id.trim().to_string(),
        causal_binding,
        request_digest: stored.request_digest,
        pool_id: stored.pool_id,
        capacity_epoch: stored.capacity_epoch,
        delivery_window_id: stored.delivery_window_id,
        subject_kind: stored.subject_kind,
        subject_id: stored.subject_id,
    })
}

pub(super) fn held_claim_causal_transaction_on(
    conn: &Connection,
    claim_id: &str,
    claim_effect_key: &str,
) -> Result<Option<StoredCapacityCausalTransaction>> {
    let transaction_id = conn
        .query_row(
            "SELECT transaction_id
               FROM compute_capacity_ledger_transactions
              WHERE claim_id=?1
                AND claim_effect='held'
                AND claim_effect_key=?2
                AND event_kind='reservation_held'",
            params![claim_id.trim(), claim_effect_key.trim()],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    transaction_id
        .map(|transaction_id| capacity_causal_transaction_on(conn, &transaction_id))
        .transpose()
}

pub(super) fn active_attempt_causal_transaction_on(
    conn: &Connection,
    claim_id: &str,
    attempt_lease_id: &str,
) -> Result<Option<StoredCapacityCausalTransaction>> {
    let transaction_id = conn
        .query_row(
            "SELECT transaction_id
               FROM compute_capacity_ledger_transactions
              WHERE claim_id=?1
                AND claim_effect='active'
                AND claim_effect_key=?2
                AND event_kind='attempt_activated'",
            params![claim_id.trim(), attempt_lease_id.trim()],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    transaction_id
        .map(|transaction_id| capacity_causal_transaction_on(conn, &transaction_id))
        .transpose()
}

fn validate_stored_causal_binding(binding: &ComputeCapacityCausalBinding) -> Result<()> {
    if binding.offer.as_ref().is_some_and(|offer| {
        offer.offer_id.trim().is_empty()
            || offer.offer_version <= 0
            || offer.offer_digest.trim().is_empty()
    }) || binding
        .job_id
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
        || binding
            .reservation_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        || binding
            .attempt_lease_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        || binding.reservation_id.is_some() && binding.job_id.is_none()
    {
        bail!("容量账本持久化因果绑定无效");
    }
    match (&binding.attempt_lease_id, binding.fencing_generation) {
        (None, None) => Ok(()),
        (Some(_), Some(generation))
            if generation > 0 && binding.reservation_id.is_some() && binding.job_id.is_some() =>
        {
            Ok(())
        }
        _ => bail!("容量账本 Attempt 因果绑定无效"),
    }
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
