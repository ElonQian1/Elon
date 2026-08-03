use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::capacity::{
    ComputeCapacityAccount, ComputeCapacityCausalBinding, ComputeCapacityClaim,
    ComputeCapacityClaimEffectBinding, ComputeCapacityClaimState, ComputeCapacityEventKind,
    ComputeCapacityLedgerTransaction, ComputeCapacityMovementLine, ComputeCapacityOfferBinding,
    COMPUTE_CAPACITY_TRANSACTION_SCHEMA,
};

use super::{
    compute_capacity_claim_rows::{
        claim_state_value, finalize_claim_digest, stored_claim_on, update_claim_projection_on,
    },
    compute_capacity_ledger::ComputeCapacityLedgerWriteReceipt,
    compute_capacity_posting::{
        balances_for_transaction_on, capacity_causal_transaction_on, event_kind_value,
        finalize_transaction_digest, held_claim_causal_transaction_on, next_ledger_sequence_on,
        post_capacity_transaction_on, reservation_capacity_causal_binding,
        StoredCapacityCausalTransaction,
    },
    compute_capacity_request_digest::finish_claim_request_digest,
    compute_capacity_rows::stored_bucket_on,
    new_id, now, Store,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComputeCapacityClaimTerminalAction {
    Release,
    Expire,
}

#[derive(Debug, Clone)]
pub(crate) struct FinishComputeCapacityClaim {
    pub claim_id: String,
    pub expected_revision: i64,
    pub action: ComputeCapacityClaimTerminalAction,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FinishComputeCapacityClaimReceipt {
    pub claim_id: String,
    pub claim_digest: String,
    pub state: String,
    pub revision: i64,
    pub request_digest: String,
    pub recorded_at: String,
    pub replayed: bool,
    pub ledger: ComputeCapacityLedgerWriteReceipt,
}

impl Store {
    pub(crate) fn finish_compute_capacity_claim(
        &self,
        input: FinishComputeCapacityClaim,
    ) -> Result<FinishComputeCapacityClaimReceipt> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let receipt = finish_compute_capacity_claim_on(&tx, input, None)?;
        tx.commit()?;
        Ok(receipt)
    }
}

pub(super) fn finish_compute_capacity_reservation_claim_on(
    tx: &Transaction<'_>,
    input: FinishComputeCapacityClaim,
    offer: ComputeCapacityOfferBinding,
    job_id: &str,
    reservation_id: &str,
) -> Result<FinishComputeCapacityClaimReceipt> {
    let expected_causal_binding =
        reservation_capacity_causal_binding(offer, job_id, reservation_id)?;
    finish_compute_capacity_claim_on(tx, input, Some(&expected_causal_binding))
}

fn finish_compute_capacity_claim_on(
    tx: &Transaction<'_>,
    input: FinishComputeCapacityClaim,
    expected_causal_binding: Option<&ComputeCapacityCausalBinding>,
) -> Result<FinishComputeCapacityClaimReceipt> {
    validate_finish_input(&input)?;
    let request_digest = finish_claim_request_digest(&input)?;
    let transaction_scope = transaction_scope(&input);
    let claim =
        stored_claim_on(tx, input.claim_id.trim())?.ok_or_else(|| anyhow!("容量 Claim 不存在"))?;
    validate_finish_claim_binding(&claim, expected_causal_binding)?;

    if let Some(existing) =
        read_existing_transition_on(tx, &transaction_scope, input.idempotency_key.trim())?
    {
        return replay_existing_transition_on(
            tx,
            &input,
            &request_digest,
            expected_causal_binding,
            existing,
        );
    }

    if claim.revision != input.expected_revision {
        bail!("容量 Claim revision 已变化，拒绝执行旧终态请求");
    }
    if claim.state != ComputeCapacityClaimState::Held {
        bail!("只有 held 容量 Claim 可以释放或到期；active 必须走 Attempt 归还路径");
    }
    let from_account = ComputeCapacityAccount::Held;
    let recorded_at = now();
    let occurred_at = parse_utc("容量 Claim 终态发生时间", input.occurred_at.trim())?;
    let recorded_at_utc = parse_utc("容量 Claim 终态记录时间", &recorded_at)?;
    let held_recorded_at = parse_utc("容量 Claim held 记录时间", &claim.created_at)?;
    if occurred_at < held_recorded_at {
        bail!("容量 Claim 终态发生时间不能早于原始 held 记录时间");
    }
    if occurred_at > recorded_at_utc {
        bail!("容量 Claim 终态发生时间不能晚于当前记录时间");
    }
    let occurred_at_value = occurred_at.with_timezone(&Utc).to_rfc3339();
    enforce_expiry_boundary(
        input.action,
        claim.expires_at.as_deref(),
        input.occurred_at.trim(),
        &recorded_at,
    )?;

    let mut balances = BTreeMap::new();
    let mut movements = Vec::with_capacity(claim.lines.len());
    for line in &claim.lines {
        let held_units =
            claim_account_net_units_on(tx, &claim.claim_id, &line.bucket.bucket_id, "held")?;
        if held_units != i128::from(line.quantity_units) {
            bail!(
                "容量 Claim {} 在 bucket {} 的 held 归属为 {}，与 Claim 数量 {} 不一致",
                claim.claim_id,
                line.bucket.bucket_id,
                held_units,
                line.quantity_units
            );
        }
        let active_units =
            claim_account_net_units_on(tx, &claim.claim_id, &line.bucket.bucket_id, "active")?;
        if active_units != 0 {
            bail!(
                "容量 Claim {} 在 bucket {} 仍拥有 active 容量，必须走 Attempt 归还路径",
                claim.claim_id,
                line.bucket.bucket_id
            );
        }
        let stored = stored_bucket_on(tx, &line.bucket.bucket_id)?
            .ok_or_else(|| anyhow!("容量 Claim 引用的 bucket 不存在"))?;
        if stored.balance.binding != line.bucket {
            bail!("容量 Claim 资源绑定在终态推进前发生变化");
        }
        movements.push(ComputeCapacityMovementLine {
            line_no: line.line_no,
            bucket: line.bucket.clone(),
            quantity_units: line.quantity_units,
            from_account,
            to_account: ComputeCapacityAccount::Available,
        });
        balances.insert(line.bucket.bucket_id.clone(), stored.balance);
    }

    let causal_predecessor =
        held_claim_causal_transaction_on(tx, &claim.claim_id, claim.idempotency_key.as_str())?
            .ok_or_else(|| anyhow!("容量 Claim 缺少原始 held 账本事务"))?;
    validate_original_held_transaction(&claim, &causal_predecessor)?;
    validate_finish_causal_binding(&causal_predecessor.causal_binding, expected_causal_binding)?;
    let causal_transaction_id = Some(causal_predecessor.transaction_id);
    let (event_kind, next_state, claim_effect) = terminal_parts(input.action);
    let mut ledger_transaction = ComputeCapacityLedgerTransaction {
        schema: COMPUTE_CAPACITY_TRANSACTION_SCHEMA.to_string(),
        transaction_id: new_id("capacity_tx"),
        transaction_digest: String::new(),
        pool: claim.pool.clone(),
        delivery_window: claim.delivery_window.clone(),
        ledger_sequence: next_ledger_sequence_on(tx, &claim.pool)?,
        event_kind,
        claim_effect: Some(ComputeCapacityClaimEffectBinding {
            claim_id: claim.claim_id.clone(),
            claim_effect: claim_effect.to_string(),
            claim_effect_key: input.idempotency_key.trim().to_string(),
        }),
        causal_binding: causal_predecessor.causal_binding,
        idempotency_scope: transaction_scope,
        idempotency_key: input.idempotency_key.trim().to_string(),
        request_digest,
        subject_kind: claim.subject_kind.clone(),
        subject_id: claim.subject_id.clone(),
        causal_transaction_id,
        movements,
        occurred_at: occurred_at_value,
        recorded_at: recorded_at.clone(),
    };
    finalize_transaction_digest(&mut ledger_transaction)?;
    let current_balances = post_capacity_transaction_on(tx, &ledger_transaction, balances)?;

    let previous_revision = claim.revision;
    let previous_state = claim.state;
    let mut next_claim = claim;
    next_claim.state = next_state;
    next_claim.revision = next_claim
        .revision
        .checked_add(1)
        .ok_or_else(|| anyhow!("容量 Claim revision 溢出"))?;
    next_claim.updated_at = recorded_at.clone();
    next_claim.terminal_at = Some(recorded_at);
    finalize_claim_digest(&mut next_claim)?;
    update_claim_projection_on(tx, previous_revision, previous_state, &next_claim)?;

    let receipt = FinishComputeCapacityClaimReceipt {
        claim_id: next_claim.claim_id,
        claim_digest: next_claim.claim_digest,
        state: claim_state_value(next_claim.state).to_string(),
        revision: next_claim.revision,
        request_digest: ledger_transaction.request_digest.clone(),
        recorded_at: ledger_transaction.recorded_at.clone(),
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
    Ok(receipt)
}

struct ExistingTransition {
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

fn read_existing_transition_on(
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

fn replay_existing_transition_on(
    conn: &Connection,
    input: &FinishComputeCapacityClaim,
    request_digest: &str,
    expected_causal_binding: Option<&ComputeCapacityCausalBinding>,
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
    validate_finish_claim_binding(&claim, expected_causal_binding)?;
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
    validate_finish_causal_binding(&original_held.causal_binding, expected_causal_binding)?;
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

fn validate_finish_claim_binding(
    claim: &ComputeCapacityClaim,
    expected_causal_binding: Option<&ComputeCapacityCausalBinding>,
) -> Result<()> {
    match expected_causal_binding.and_then(|binding| binding.reservation_id.as_deref()) {
        Some(reservation_id) => {
            if claim.subject_kind != "compute_reservation" || claim.subject_id != reservation_id {
                bail!("Reservation-bound 容量 Finish 的 Claim 身份不一致");
            }
        }
        None => {
            if claim.subject_kind == "compute_reservation" {
                bail!("compute_reservation 必须通过 Reservation-bound 事务内入口终结");
            }
        }
    }
    Ok(())
}

fn validate_finish_causal_binding(
    stored: &ComputeCapacityCausalBinding,
    expected: Option<&ComputeCapacityCausalBinding>,
) -> Result<()> {
    match expected {
        Some(expected) if stored == expected => Ok(()),
        Some(_) => bail!("Reservation-bound 容量 Finish 的业务因果绑定不一致"),
        None if stored.reservation_id.is_none() => Ok(()),
        None => bail!("Reservation-bound 容量 Claim 必须由外层事务内入口终结"),
    }
}

fn validate_original_held_transaction(
    claim: &ComputeCapacityClaim,
    held: &StoredCapacityCausalTransaction,
) -> Result<()> {
    if held.request_digest != claim.request_digest
        || held.pool_id != claim.pool.pool_id
        || held.capacity_epoch != claim.pool.capacity_epoch
        || held.delivery_window_id != claim.delivery_window.window_id
        || held.subject_kind != claim.subject_kind
        || held.subject_id != claim.subject_id
    {
        bail!("容量 Claim 与原始 held 账本事务绑定不一致");
    }
    Ok(())
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
    let deltas = statement.query_map(params![claim_id, bucket_id, account], |row| {
        row.get::<_, i64>(0)
    })?;
    let mut total = 0_i128;
    for delta in deltas {
        total = total
            .checked_add(i128::from(delta?))
            .ok_or_else(|| anyhow!("容量 Claim 归属余额溢出"))?;
    }
    Ok(total)
}

fn validate_finish_input(input: &FinishComputeCapacityClaim) -> Result<()> {
    for (label, value) in [
        ("容量 Claim ID", input.claim_id.as_str()),
        ("幂等范围", input.idempotency_scope.as_str()),
        ("幂等键", input.idempotency_key.as_str()),
        ("发生时间", input.occurred_at.as_str()),
    ] {
        if value.trim().is_empty() {
            bail!("{label}不能为空");
        }
    }
    if input.expected_revision <= 0 {
        bail!("容量 Claim expected_revision 必须为正整数");
    }
    parse_utc("容量 Claim 终态发生时间", input.occurred_at.trim())?;
    Ok(())
}

fn enforce_expiry_boundary(
    action: ComputeCapacityClaimTerminalAction,
    expires_at: Option<&str>,
    occurred_at: &str,
    recorded_at: &str,
) -> Result<()> {
    if action != ComputeCapacityClaimTerminalAction::Expire {
        return Ok(());
    }
    let expires_at = expires_at.ok_or_else(|| anyhow!("无到期时间的容量 Claim 不能自动过期"))?;
    let occurred_at = parse_utc("容量 Claim 到期发生时间", occurred_at)?;
    let recorded_at = parse_utc("容量 Claim 到期记录时间", recorded_at)?;
    let expires_at = parse_utc("容量 Claim 到期边界", expires_at)?;
    if recorded_at < expires_at {
        bail!("容量 Claim 尚未到期");
    }
    if occurred_at < expires_at {
        bail!("容量 Claim 到期事件的发生时间不能早于到期边界");
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

fn transaction_scope(input: &FinishComputeCapacityClaim) -> String {
    format!(
        "capacity_claim_{}:{}",
        match input.action {
            ComputeCapacityClaimTerminalAction::Release => "release",
            ComputeCapacityClaimTerminalAction::Expire => "expire",
        },
        input.idempotency_scope.trim()
    )
}

fn terminal_parts(
    action: ComputeCapacityClaimTerminalAction,
) -> (
    ComputeCapacityEventKind,
    ComputeCapacityClaimState,
    &'static str,
) {
    match action {
        ComputeCapacityClaimTerminalAction::Release => (
            ComputeCapacityEventKind::ReservationReleased,
            ComputeCapacityClaimState::Released,
            "released",
        ),
        ComputeCapacityClaimTerminalAction::Expire => (
            ComputeCapacityEventKind::ReservationExpired,
            ComputeCapacityClaimState::Expired,
            "expired",
        ),
    }
}
