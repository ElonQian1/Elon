use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, bail, Result};
use chrono::DateTime;
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    capacity::{
        apply_capacity_transaction, expand_capacity_ledger_legs, ComputeCapacityAccount,
        ComputeCapacityBucketBalance, ComputeCapacityCausalBinding, ComputeCapacityEventKind,
        ComputeCapacityLedgerTransaction, ComputeCapacityMovementLine, ComputeCapacityPoolBinding,
        COMPUTE_CAPACITY_TRANSACTION_SCHEMA,
    },
    market::ComputeDeliveryWindowBinding,
};

use super::{compute_capacity_rows::stored_bucket_on, new_id, now, Store};

#[derive(Debug, Clone)]
pub(crate) struct AddComputeCapacitySupplyLine {
    pub bucket_id: String,
    pub quantity_units: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct AddComputeCapacitySupply {
    pub pool: ComputeCapacityPoolBinding,
    pub delivery_window: ComputeDeliveryWindowBinding,
    pub subject_kind: String,
    pub subject_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub lines: Vec<AddComputeCapacitySupplyLine>,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityLedgerWriteReceipt {
    pub transaction_id: String,
    pub transaction_digest: String,
    pub ledger_sequence: i64,
    pub event_kind: String,
    pub request_digest: String,
    pub replayed: bool,
    pub current_balances: Vec<ComputeCapacityBucketBalance>,
}

impl Store {
    pub(crate) fn add_compute_capacity_supply(
        &self,
        input: AddComputeCapacitySupply,
    ) -> Result<ComputeCapacityLedgerWriteReceipt> {
        validate_supply_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        let existing = tx
            .query_row(
                "SELECT transaction_id, transaction_digest, ledger_sequence,
                        event_kind, request_digest, pool_id, capacity_epoch,
                        delivery_window_id
                   FROM compute_capacity_ledger_transactions
                  WHERE idempotency_scope=?1 AND idempotency_key=?2",
                params![input.idempotency_scope.trim(), input.idempotency_key.trim()],
                |row| {
                    Ok(ExistingTransaction {
                        transaction_id: row.get(0)?,
                        transaction_digest: row.get(1)?,
                        ledger_sequence: row.get(2)?,
                        event_kind: row.get(3)?,
                        request_digest: row.get(4)?,
                        pool_id: row.get(5)?,
                        capacity_epoch: row.get(6)?,
                        delivery_window_id: row.get(7)?,
                    })
                },
            )
            .optional()?;
        if let Some(existing) = existing {
            if existing.request_digest != input.request_digest.trim()
                || existing.event_kind != "supply_added"
                || existing.pool_id != input.pool.pool_id.trim()
                || existing.capacity_epoch != input.pool.capacity_epoch
                || existing.delivery_window_id != input.delivery_window.window_id.trim()
            {
                bail!("相同容量幂等键不能用于不同发行请求");
            }
            let balances = balances_for_transaction(&tx, &existing.transaction_id)?;
            if balances.iter().any(|balance| {
                balance.binding.pool != input.pool
                    || balance.binding.delivery_window != input.delivery_window
            }) {
                bail!("容量幂等重放的资源绑定与原事务不一致");
            }
            tx.commit()?;
            return Ok(ComputeCapacityLedgerWriteReceipt {
                transaction_id: existing.transaction_id,
                transaction_digest: existing.transaction_digest,
                ledger_sequence: existing.ledger_sequence,
                event_kind: existing.event_kind,
                request_digest: existing.request_digest,
                replayed: true,
                current_balances: balances,
            });
        }

        let mut balances = BTreeMap::new();
        let mut movements = Vec::with_capacity(input.lines.len());
        let mut bucket_ids = BTreeSet::new();
        for (line_no, line) in input.lines.iter().enumerate() {
            if !bucket_ids.insert(line.bucket_id.trim()) {
                bail!("同一容量发行请求不能重复 bucket");
            }
            let stored = stored_bucket_on(&tx, &line.bucket_id)?
                .ok_or_else(|| anyhow!("容量 bucket {} 不存在", line.bucket_id))?;
            if stored.balance.binding.pool != input.pool
                || stored.balance.binding.delivery_window != input.delivery_window
            {
                bail!("容量发行 bucket 不属于目标容量池与交付窗口");
            }
            movements.push(ComputeCapacityMovementLine {
                line_no: i64::try_from(line_no)?,
                bucket: stored.balance.binding.clone(),
                quantity_units: line.quantity_units,
                from_account: ComputeCapacityAccount::Issuance,
                to_account: ComputeCapacityAccount::Available,
            });
            balances.insert(stored.balance.binding.bucket_id.clone(), stored.balance);
        }
        let before = balances.clone();
        let ledger_sequence: i64 = tx.query_row(
            "SELECT COALESCE(MAX(ledger_sequence), 0) + 1
               FROM compute_capacity_ledger_transactions
              WHERE pool_id=?1 AND capacity_epoch=?2",
            params![input.pool.pool_id.trim(), input.pool.capacity_epoch],
            |row| row.get(0),
        )?;
        let recorded_at = now();
        let mut transaction = ComputeCapacityLedgerTransaction {
            schema: COMPUTE_CAPACITY_TRANSACTION_SCHEMA.to_string(),
            transaction_id: new_id("capacity_tx"),
            transaction_digest: String::new(),
            pool: input.pool,
            delivery_window: input.delivery_window,
            ledger_sequence,
            event_kind: ComputeCapacityEventKind::SupplyAdded,
            claim_effect: None,
            causal_binding: ComputeCapacityCausalBinding {
                offer: None,
                job_id: None,
                reservation_id: None,
                attempt_lease_id: None,
                fencing_generation: None,
            },
            idempotency_scope: input.idempotency_scope.trim().to_string(),
            idempotency_key: input.idempotency_key.trim().to_string(),
            request_digest: input.request_digest.trim().to_string(),
            subject_kind: input.subject_kind.trim().to_string(),
            subject_id: input.subject_id.trim().to_string(),
            causal_transaction_id: None,
            movements,
            occurred_at: input.occurred_at.trim().to_string(),
            recorded_at,
        };
        transaction.transaction_digest = transaction_digest(&transaction)?;
        let legs = expand_capacity_ledger_legs(&transaction).map_err(anyhow::Error::new)?;
        apply_capacity_transaction(&mut balances, &transaction).map_err(anyhow::Error::new)?;

        tx.execute(
            "INSERT INTO compute_capacity_ledger_transactions (
                transaction_id, transaction_digest, pool_id, capacity_epoch,
                delivery_window_id, ledger_sequence, event_kind,
                claim_id, claim_effect, claim_effect_key,
                offer_id, offer_version, offer_digest, job_id, reservation_id,
                attempt_lease_id, fencing_generation, idempotency_scope,
                idempotency_key, request_digest, subject_kind, subject_id,
                causal_transaction_id, occurred_at, recorded_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, 'supply_added',
                NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL,
                ?7, ?8, ?9, ?10, ?11, NULL, ?12, ?13
             )",
            params![
                transaction.transaction_id,
                transaction.transaction_digest,
                transaction.pool.pool_id,
                transaction.pool.capacity_epoch,
                transaction.delivery_window.window_id,
                transaction.ledger_sequence,
                transaction.idempotency_scope,
                transaction.idempotency_key,
                transaction.request_digest,
                transaction.subject_kind,
                transaction.subject_id,
                transaction.occurred_at,
                transaction.recorded_at,
            ],
        )?;
        for leg in legs {
            tx.execute(
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
            let changed = tx.execute(
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
                bail!("容量 bucket 余额版本已变化，发行事务未提交");
            }
        }
        let receipt = ComputeCapacityLedgerWriteReceipt {
            transaction_id: transaction.transaction_id,
            transaction_digest: transaction.transaction_digest,
            ledger_sequence: transaction.ledger_sequence,
            event_kind: "supply_added".to_string(),
            request_digest: transaction.request_digest,
            replayed: false,
            current_balances: balances.into_values().collect(),
        };
        tx.commit()?;
        Ok(receipt)
    }
}

struct ExistingTransaction {
    transaction_id: String,
    transaction_digest: String,
    ledger_sequence: i64,
    event_kind: String,
    request_digest: String,
    pool_id: String,
    capacity_epoch: i64,
    delivery_window_id: String,
}

fn validate_supply_input(input: &AddComputeCapacitySupply) -> Result<()> {
    for (label, value) in [
        ("容量池 ID", input.pool.pool_id.as_str()),
        ("容量池摘要", input.pool.pool_digest.as_str()),
        ("交付窗口 ID", input.delivery_window.window_id.as_str()),
        ("交付窗口摘要", input.delivery_window.window_digest.as_str()),
        ("主体类型", input.subject_kind.as_str()),
        ("主体 ID", input.subject_id.as_str()),
        ("幂等范围", input.idempotency_scope.as_str()),
        ("幂等键", input.idempotency_key.as_str()),
        ("请求摘要", input.request_digest.as_str()),
        ("发生时间", input.occurred_at.as_str()),
    ] {
        if value.trim().is_empty() {
            bail!("{label}不能为空");
        }
    }
    if input.pool.capacity_epoch <= 0 || input.pool.pool_revision <= 0 {
        bail!("容量池 epoch 和版本必须为正整数");
    }
    if input.lines.is_empty() {
        bail!("容量发行至少需要一个 bucket");
    }
    if input
        .lines
        .iter()
        .any(|line| line.bucket_id.trim().is_empty() || line.quantity_units <= 0)
    {
        bail!("容量发行 bucket 和数量必须有效");
    }
    let occurred_at = DateTime::parse_from_rfc3339(input.occurred_at.trim())
        .map_err(|_| anyhow!("容量发行发生时间不是 RFC3339"))?;
    if occurred_at.offset().local_minus_utc() != 0 {
        bail!("容量发行发生时间必须使用 UTC 时区");
    }
    Ok(())
}

fn balances_for_transaction(
    conn: &rusqlite::Connection,
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

fn transaction_digest(transaction: &ComputeCapacityLedgerTransaction) -> Result<String> {
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
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&payload)?)))
}

fn leg_role_value(
    role: crate::compute_federation::capacity::ComputeCapacityLegRole,
) -> &'static str {
    match role {
        crate::compute_federation::capacity::ComputeCapacityLegRole::From => "from",
        crate::compute_federation::capacity::ComputeCapacityLegRole::To => "to",
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
