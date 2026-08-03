use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::{
    capacity::{
        ComputeCapacityAccount, ComputeCapacityCausalBinding, ComputeCapacityClaim,
        ComputeCapacityClaimEffectBinding, ComputeCapacityClaimKind, ComputeCapacityClaimLine,
        ComputeCapacityClaimState, ComputeCapacityEventKind, ComputeCapacityLedgerTransaction,
        ComputeCapacityMovementLine, ComputeCapacityPoolBinding, COMPUTE_CAPACITY_CLAIM_SCHEMA,
        COMPUTE_CAPACITY_TRANSACTION_SCHEMA,
    },
    market::ComputeDeliveryWindowBinding,
};

use super::{
    compute_capacity_claim_rows::{claim_kind_value, finalize_claim_digest, insert_claim_on},
    compute_capacity_ledger::ComputeCapacityLedgerWriteReceipt,
    compute_capacity_pool_guards::{
        ensure_pool_operation_allowed_on, ComputeCapacityPoolOperation,
    },
    compute_capacity_posting::{
        balances_for_transaction_on, event_kind_value, finalize_transaction_digest,
        next_ledger_sequence_on, post_capacity_transaction_on,
    },
    compute_capacity_request_digest::hold_claim_request_digest,
    compute_capacity_rows::stored_bucket_on,
    new_id, now, Store,
};

#[derive(Debug, Clone)]
pub(crate) struct HoldComputeCapacityClaimLine {
    pub bucket_id: String,
    pub quantity_units: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct HoldComputeCapacityClaim {
    pub pool: ComputeCapacityPoolBinding,
    pub delivery_window: ComputeDeliveryWindowBinding,
    pub claim_kind: ComputeCapacityClaimKind,
    pub subject_kind: String,
    pub subject_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub lines: Vec<HoldComputeCapacityClaimLine>,
    pub expires_at: Option<String>,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct HoldComputeCapacityClaimReceipt {
    pub claim_id: String,
    pub claim_digest: String,
    pub claim_kind: String,
    pub state: String,
    pub revision: i64,
    pub request_digest: String,
    pub replayed: bool,
    pub ledger: ComputeCapacityLedgerWriteReceipt,
}

impl Store {
    pub(crate) fn hold_compute_capacity_claim(
        &self,
        input: HoldComputeCapacityClaim,
    ) -> Result<HoldComputeCapacityClaimReceipt> {
        validate_hold_input(&input)?;
        let request_digest = hold_claim_request_digest(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        let existing = read_existing_claim_on(
            &tx,
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
        )?;
        if let Some(existing) = existing {
            let receipt = replay_existing_hold_on(&tx, &input, &request_digest, existing)?;
            tx.commit()?;
            return Ok(receipt);
        }
        ensure_pool_operation_allowed_on(
            &tx,
            &input.pool,
            ComputeCapacityPoolOperation::HoldClaim,
        )?;

        let recorded_at = now();
        let recorded_at_utc = parse_utc("容量 Claim 记录时间", &recorded_at)?;
        let occurred_at = parse_utc("容量 Claim 发生时间", input.occurred_at.trim())?;
        let expires_at = parse_utc(
            "容量 Claim 到期时间",
            input
                .expires_at
                .as_deref()
                .ok_or_else(|| anyhow!("容量 Claim 必须设置到期时间"))?
                .trim(),
        )?;
        if occurred_at > recorded_at_utc {
            bail!("容量 Claim 发生时间不能晚于当前记录时间");
        }
        if expires_at <= recorded_at_utc {
            bail!("容量 Claim 到期时间必须晚于当前记录时间");
        }
        let occurred_at_value = occurred_at.with_timezone(&Utc).to_rfc3339();
        let expires_at_value = expires_at.with_timezone(&Utc).to_rfc3339();
        let mut balances = BTreeMap::new();
        let mut claim_lines = Vec::with_capacity(input.lines.len());
        let mut movements = Vec::with_capacity(input.lines.len());
        let mut bucket_ids = BTreeSet::new();
        let mut meters = BTreeSet::new();
        let mut window_bounds: Option<(String, String)> = None;
        for (line_no, input_line) in input.lines.iter().enumerate() {
            if !bucket_ids.insert(input_line.bucket_id.trim()) {
                bail!("同一容量 Claim 不能重复 bucket");
            }
            let stored = stored_bucket_on(&tx, &input_line.bucket_id)?
                .ok_or_else(|| anyhow!("容量 bucket {} 不存在", input_line.bucket_id))?;
            if stored.balance.binding.pool != input.pool
                || stored.balance.binding.delivery_window != input.delivery_window
            {
                bail!("容量 Claim bucket 不属于目标容量池与交付窗口");
            }
            let window_starts_at = parse_utc("容量 bucket 窗口开始时间", &stored.starts_at)?;
            let window_ends_at = parse_utc("容量 bucket 窗口结束时间", &stored.ends_at)?;
            if window_starts_at >= window_ends_at {
                bail!("容量 bucket 交付窗口边界无效");
            }
            if occurred_at >= window_ends_at || recorded_at_utc >= window_ends_at {
                bail!("已结束的交付窗口不能创建容量 Claim");
            }
            if expires_at > window_ends_at {
                bail!("容量 Claim 到期时间不能晚于交付窗口结束时间");
            }
            if window_bounds.as_ref().is_some_and(|(starts_at, ends_at)| {
                starts_at != &stored.starts_at || ends_at != &stored.ends_at
            }) {
                bail!("同一容量 Claim 的 bucket 必须共享精确交付窗口边界");
            }
            window_bounds = Some((stored.starts_at.clone(), stored.ends_at.clone()));
            if !meters.insert(stored.balance.binding.meter.as_str()) {
                bail!("同一容量 Claim 不能重复 meter");
            }
            let line_no = i64::try_from(line_no)?;
            claim_lines.push(ComputeCapacityClaimLine {
                line_no,
                bucket: stored.balance.binding.clone(),
                quantity_units: input_line.quantity_units,
            });
            movements.push(ComputeCapacityMovementLine {
                line_no,
                bucket: stored.balance.binding.clone(),
                quantity_units: input_line.quantity_units,
                from_account: ComputeCapacityAccount::Available,
                to_account: ComputeCapacityAccount::Held,
            });
            balances.insert(stored.balance.binding.bucket_id.clone(), stored.balance);
        }

        let mut claim = ComputeCapacityClaim {
            schema: COMPUTE_CAPACITY_CLAIM_SCHEMA.to_string(),
            claim_id: new_id("capacity_claim"),
            claim_digest: String::new(),
            pool: input.pool,
            delivery_window: input.delivery_window,
            claim_kind: input.claim_kind,
            state: ComputeCapacityClaimState::Held,
            revision: 1,
            parent_claim_id: None,
            subject_kind: input.subject_kind.trim().to_string(),
            subject_id: input.subject_id.trim().to_string(),
            idempotency_scope: input.idempotency_scope.trim().to_string(),
            idempotency_key: input.idempotency_key.trim().to_string(),
            request_digest,
            lines: claim_lines,
            created_at: recorded_at.clone(),
            updated_at: recorded_at.clone(),
            expires_at: Some(expires_at_value),
            terminal_at: None,
        };
        finalize_claim_digest(&mut claim)?;
        insert_claim_on(&tx, &claim)?;

        let mut ledger_transaction = ComputeCapacityLedgerTransaction {
            schema: COMPUTE_CAPACITY_TRANSACTION_SCHEMA.to_string(),
            transaction_id: new_id("capacity_tx"),
            transaction_digest: String::new(),
            pool: claim.pool.clone(),
            delivery_window: claim.delivery_window.clone(),
            ledger_sequence: next_ledger_sequence_on(&tx, &claim.pool)?,
            event_kind: ComputeCapacityEventKind::ReservationHeld,
            claim_effect: Some(ComputeCapacityClaimEffectBinding {
                claim_id: claim.claim_id.clone(),
                claim_effect: "held".to_string(),
                claim_effect_key: claim.idempotency_key.clone(),
            }),
            causal_binding: ComputeCapacityCausalBinding {
                offer: None,
                job_id: None,
                reservation_id: None,
                attempt_lease_id: None,
                fencing_generation: None,
            },
            idempotency_scope: format!("capacity_claim_hold:{}", claim.idempotency_scope),
            idempotency_key: claim.idempotency_key.clone(),
            request_digest: claim.request_digest.clone(),
            subject_kind: claim.subject_kind.clone(),
            subject_id: claim.subject_id.clone(),
            causal_transaction_id: None,
            movements,
            occurred_at: occurred_at_value,
            recorded_at,
        };
        finalize_transaction_digest(&mut ledger_transaction)?;
        let current_balances = post_capacity_transaction_on(&tx, &ledger_transaction, balances)?;
        let receipt = HoldComputeCapacityClaimReceipt {
            claim_id: claim.claim_id,
            claim_digest: claim.claim_digest,
            claim_kind: claim_kind_value(claim.claim_kind).to_string(),
            state: "held".to_string(),
            revision: claim.revision,
            request_digest: claim.request_digest,
            replayed: false,
            ledger: ComputeCapacityLedgerWriteReceipt {
                transaction_id: ledger_transaction.transaction_id,
                transaction_digest: ledger_transaction.transaction_digest,
                ledger_sequence: ledger_transaction.ledger_sequence,
                event_kind: event_kind_value(ledger_transaction.event_kind).to_string(),
                request_digest: ledger_transaction.request_digest,
                replayed: false,
                current_balances,
            },
        };
        tx.commit()?;
        Ok(receipt)
    }
}

struct ExistingClaim {
    claim_id: String,
    claim_digest: String,
    pool_id: String,
    capacity_epoch: i64,
    delivery_window_id: String,
    claim_kind: String,
    subject_kind: String,
    subject_id: String,
    state: String,
    revision: i64,
    request_digest: String,
}

struct ExistingHoldTransaction {
    transaction_id: String,
    transaction_digest: String,
    ledger_sequence: i64,
    event_kind: String,
    request_digest: String,
    pool_id: String,
    capacity_epoch: i64,
    delivery_window_id: String,
}

fn read_existing_claim_on(
    conn: &Connection,
    idempotency_scope: &str,
    idempotency_key: &str,
) -> Result<Option<ExistingClaim>> {
    conn.query_row(
        "SELECT claim_id, claim_digest, pool_id, capacity_epoch,
                delivery_window_id, claim_kind, subject_kind, subject_id,
                status, revision, request_digest
           FROM compute_capacity_claims
          WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![idempotency_scope, idempotency_key],
        |row| {
            Ok(ExistingClaim {
                claim_id: row.get(0)?,
                claim_digest: row.get(1)?,
                pool_id: row.get(2)?,
                capacity_epoch: row.get(3)?,
                delivery_window_id: row.get(4)?,
                claim_kind: row.get(5)?,
                subject_kind: row.get(6)?,
                subject_id: row.get(7)?,
                state: row.get(8)?,
                revision: row.get(9)?,
                request_digest: row.get(10)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn replay_existing_hold_on(
    conn: &Connection,
    input: &HoldComputeCapacityClaim,
    request_digest: &str,
    existing: ExistingClaim,
) -> Result<HoldComputeCapacityClaimReceipt> {
    if existing.request_digest != request_digest
        || existing.pool_id != input.pool.pool_id.trim()
        || existing.capacity_epoch != input.pool.capacity_epoch
        || existing.delivery_window_id != input.delivery_window.window_id.trim()
        || existing.claim_kind != claim_kind_value(input.claim_kind)
        || existing.subject_kind != input.subject_kind.trim()
        || existing.subject_id != input.subject_id.trim()
    {
        bail!("相同容量 Claim 幂等键不能用于不同预留请求");
    }

    let ledger = conn
        .query_row(
            "SELECT transaction_id, transaction_digest, ledger_sequence,
                    event_kind, request_digest, pool_id, capacity_epoch,
                    delivery_window_id
               FROM compute_capacity_ledger_transactions
              WHERE claim_id=?1 AND claim_effect='held' AND claim_effect_key=?2",
            params![existing.claim_id, input.idempotency_key.trim()],
            |row| {
                Ok(ExistingHoldTransaction {
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
        .optional()?
        .ok_or_else(|| anyhow!("容量 Claim 已存在但缺少 held 账本事务"))?;
    if ledger.event_kind != "reservation_held"
        || ledger.request_digest != request_digest
        || ledger.pool_id != input.pool.pool_id.trim()
        || ledger.capacity_epoch != input.pool.capacity_epoch
        || ledger.delivery_window_id != input.delivery_window.window_id.trim()
    {
        bail!("容量 Claim 幂等重放的账本绑定不一致");
    }
    let current_balances = balances_for_transaction_on(conn, &ledger.transaction_id)?;
    if current_balances.iter().any(|balance| {
        balance.binding.pool != input.pool
            || balance.binding.delivery_window != input.delivery_window
    }) {
        bail!("容量 Claim 幂等重放的资源绑定不一致");
    }

    Ok(HoldComputeCapacityClaimReceipt {
        claim_id: existing.claim_id,
        claim_digest: existing.claim_digest,
        claim_kind: existing.claim_kind,
        state: existing.state,
        revision: existing.revision,
        request_digest: existing.request_digest,
        replayed: true,
        ledger: ComputeCapacityLedgerWriteReceipt {
            transaction_id: ledger.transaction_id,
            transaction_digest: ledger.transaction_digest,
            ledger_sequence: ledger.ledger_sequence,
            event_kind: ledger.event_kind,
            request_digest: ledger.request_digest,
            replayed: true,
            current_balances,
        },
    })
}

fn validate_hold_input(input: &HoldComputeCapacityClaim) -> Result<()> {
    for (label, value) in [
        ("容量池 ID", input.pool.pool_id.as_str()),
        ("容量池摘要", input.pool.pool_digest.as_str()),
        ("交付窗口 ID", input.delivery_window.window_id.as_str()),
        ("交付窗口摘要", input.delivery_window.window_digest.as_str()),
        ("主体类型", input.subject_kind.as_str()),
        ("主体 ID", input.subject_id.as_str()),
        ("幂等范围", input.idempotency_scope.as_str()),
        ("幂等键", input.idempotency_key.as_str()),
        ("发生时间", input.occurred_at.as_str()),
    ] {
        if value.trim().is_empty() {
            bail!("{label}不能为空");
        }
    }
    if input.pool.capacity_epoch <= 0 || input.pool.pool_revision <= 0 {
        bail!("容量池 epoch 和版本必须为正整数");
    }
    if !matches!(
        input.claim_kind,
        ComputeCapacityClaimKind::QuoteHold
            | ComputeCapacityClaimKind::Reservation
            | ComputeCapacityClaimKind::CapacityCommitment
    ) {
        bail!("只有报价、预约或容量承诺 Claim 可以直接进入 held 状态");
    }
    if input.lines.is_empty() || input.lines.len() > 64 {
        bail!("容量 Claim 必须包含 1 到 64 个 bucket");
    }
    if input
        .lines
        .iter()
        .any(|line| line.bucket_id.trim().is_empty() || line.quantity_units <= 0)
    {
        bail!("容量 Claim bucket 和数量必须有效");
    }
    let occurred_at = parse_utc("容量 Claim 发生时间", input.occurred_at.trim())?;
    let expires_at = input
        .expires_at
        .as_deref()
        .ok_or_else(|| anyhow!("容量 Claim 必须设置到期时间"))?;
    if expires_at.trim().is_empty() {
        bail!("容量 Claim 到期时间不能为空字符串");
    }
    if parse_utc("容量 Claim 到期时间", expires_at.trim())? <= occurred_at {
        bail!("容量 Claim 到期时间必须晚于发生时间");
    }
    Ok(())
}

fn parse_utc(label: &str, value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|_| anyhow!("{label}不是 RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("{label}必须使用 UTC 时区");
    }
    Ok(parsed)
}
