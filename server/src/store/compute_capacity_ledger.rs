use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, bail, Result};
use chrono::DateTime;
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::{
    capacity::{
        ComputeCapacityAccount, ComputeCapacityBucketBalance, ComputeCapacityCausalBinding,
        ComputeCapacityEventKind, ComputeCapacityLedgerTransaction, ComputeCapacityMovementLine,
        ComputeCapacityPoolBinding, COMPUTE_CAPACITY_TRANSACTION_SCHEMA,
    },
    market::ComputeDeliveryWindowBinding,
};

use super::{
    compute_capacity_pool_guards::{
        ensure_pool_operation_allowed_on, ComputeCapacityPoolOperation,
    },
    compute_capacity_posting::{
        balances_for_transaction_on, event_kind_value, finalize_transaction_digest,
        next_ledger_sequence_on, post_capacity_transaction_on,
    },
    compute_capacity_rows::stored_bucket_on,
    new_id, now, Store,
};

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
            let balances = balances_for_transaction_on(&tx, &existing.transaction_id)?;
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
        ensure_pool_operation_allowed_on(
            &tx,
            &input.pool,
            ComputeCapacityPoolOperation::AddSupply,
        )?;

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
        let ledger_sequence = next_ledger_sequence_on(&tx, &input.pool)?;
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
        finalize_transaction_digest(&mut transaction)?;
        let current_balances = post_capacity_transaction_on(&tx, &transaction, balances)?;
        let receipt = ComputeCapacityLedgerWriteReceipt {
            transaction_id: transaction.transaction_id,
            transaction_digest: transaction.transaction_digest,
            ledger_sequence: transaction.ledger_sequence,
            event_kind: event_kind_value(transaction.event_kind).to_string(),
            request_digest: transaction.request_digest,
            replayed: false,
            current_balances,
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
