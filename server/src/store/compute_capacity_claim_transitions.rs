use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::capacity::{
    ComputeCapacityAccount, ComputeCapacityCausalBinding, ComputeCapacityClaimEffectBinding,
    ComputeCapacityClaimState, ComputeCapacityEventKind, ComputeCapacityLedgerTransaction,
    ComputeCapacityMovementLine, COMPUTE_CAPACITY_TRANSACTION_SCHEMA,
};

use super::{
    compute_capacity_claim_rows::{
        claim_state_value, finalize_claim_digest, stored_claim_on, update_claim_projection_on,
    },
    compute_capacity_ledger::ComputeCapacityLedgerWriteReceipt,
    compute_capacity_posting::{
        balances_for_transaction_on, event_kind_value, finalize_transaction_digest,
        next_ledger_sequence_on, post_capacity_transaction_on,
    },
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
    pub request_digest: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FinishComputeCapacityClaimReceipt {
    pub claim_id: String,
    pub claim_digest: String,
    pub state: String,
    pub revision: i64,
    pub request_digest: String,
    pub replayed: bool,
    pub ledger: ComputeCapacityLedgerWriteReceipt,
}

impl Store {
    pub(crate) fn finish_compute_capacity_claim(
        &self,
        input: FinishComputeCapacityClaim,
    ) -> Result<FinishComputeCapacityClaimReceipt> {
        validate_finish_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let transaction_scope = transaction_scope(&input);

        if let Some(existing) =
            read_existing_transition_on(&tx, &transaction_scope, input.idempotency_key.trim())?
        {
            let receipt = replay_existing_transition_on(&tx, &input, existing)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let claim = stored_claim_on(&tx, input.claim_id.trim())?
            .ok_or_else(|| anyhow!("容量 Claim 不存在"))?;
        if claim.revision != input.expected_revision {
            bail!("容量 Claim revision 已变化，拒绝执行旧终态请求");
        }
        let from_account = match claim.state {
            ComputeCapacityClaimState::Held => ComputeCapacityAccount::Held,
            ComputeCapacityClaimState::Active => ComputeCapacityAccount::Active,
            _ => bail!("容量 Claim 当前状态不能释放或到期"),
        };
        enforce_expiry_boundary(&input, claim.expires_at.as_deref())?;

        let mut balances = BTreeMap::new();
        let mut movements = Vec::with_capacity(claim.lines.len());
        for line in &claim.lines {
            let stored = stored_bucket_on(&tx, &line.bucket.bucket_id)?
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

        let causal_transaction_id = Some(
            latest_claim_transaction_id_on(&tx, &claim.claim_id)?
                .ok_or_else(|| anyhow!("容量 Claim 缺少前序账本事务"))?,
        );
        let recorded_at = now();
        let (event_kind, next_state, claim_effect) = terminal_parts(input.action);
        let mut ledger_transaction = ComputeCapacityLedgerTransaction {
            schema: COMPUTE_CAPACITY_TRANSACTION_SCHEMA.to_string(),
            transaction_id: new_id("capacity_tx"),
            transaction_digest: String::new(),
            pool: claim.pool.clone(),
            delivery_window: claim.delivery_window.clone(),
            ledger_sequence: next_ledger_sequence_on(&tx, &claim.pool)?,
            event_kind,
            claim_effect: Some(ComputeCapacityClaimEffectBinding {
                claim_id: claim.claim_id.clone(),
                claim_effect: claim_effect.to_string(),
                claim_effect_key: input.idempotency_key.trim().to_string(),
            }),
            causal_binding: ComputeCapacityCausalBinding {
                offer: None,
                job_id: None,
                reservation_id: None,
                attempt_lease_id: None,
                fencing_generation: None,
            },
            idempotency_scope: transaction_scope,
            idempotency_key: input.idempotency_key.trim().to_string(),
            request_digest: input.request_digest.trim().to_string(),
            subject_kind: claim.subject_kind.clone(),
            subject_id: claim.subject_id.clone(),
            causal_transaction_id,
            movements,
            occurred_at: input.occurred_at.trim().to_string(),
            recorded_at: recorded_at.clone(),
        };
        finalize_transaction_digest(&mut ledger_transaction)?;
        let current_balances = post_capacity_transaction_on(&tx, &ledger_transaction, balances)?;

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
        update_claim_projection_on(&tx, previous_revision, previous_state, &next_claim)?;

        let receipt = FinishComputeCapacityClaimReceipt {
            claim_id: next_claim.claim_id,
            claim_digest: next_claim.claim_digest,
            state: claim_state_value(next_claim.state).to_string(),
            revision: next_claim.revision,
            request_digest: ledger_transaction.request_digest.clone(),
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

struct ExistingTransition {
    transaction_id: String,
    transaction_digest: String,
    ledger_sequence: i64,
    event_kind: String,
    claim_id: String,
    claim_effect: String,
    request_digest: String,
}

fn read_existing_transition_on(
    conn: &Connection,
    idempotency_scope: &str,
    idempotency_key: &str,
) -> Result<Option<ExistingTransition>> {
    conn.query_row(
        "SELECT transaction_id, transaction_digest, ledger_sequence,
                event_kind, claim_id, claim_effect, request_digest
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
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn replay_existing_transition_on(
    conn: &Connection,
    input: &FinishComputeCapacityClaim,
    existing: ExistingTransition,
) -> Result<FinishComputeCapacityClaimReceipt> {
    let (expected_event, expected_state, expected_effect) = terminal_parts(input.action);
    if existing.claim_id != input.claim_id.trim()
        || existing.event_kind != event_kind_value(expected_event)
        || existing.claim_effect != expected_effect
        || existing.request_digest != input.request_digest.trim()
    {
        bail!("相同容量 Claim 终态幂等键不能用于不同请求");
    }
    let claim = stored_claim_on(conn, &existing.claim_id)?
        .ok_or_else(|| anyhow!("容量 Claim 终态重放时原 Claim 不存在"))?;
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

fn latest_claim_transaction_id_on(conn: &Connection, claim_id: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT transaction_id
           FROM compute_capacity_ledger_transactions
          WHERE claim_id=?1
          ORDER BY ledger_sequence DESC LIMIT 1",
        params![claim_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn validate_finish_input(input: &FinishComputeCapacityClaim) -> Result<()> {
    for (label, value) in [
        ("容量 Claim ID", input.claim_id.as_str()),
        ("幂等范围", input.idempotency_scope.as_str()),
        ("幂等键", input.idempotency_key.as_str()),
        ("请求摘要", input.request_digest.as_str()),
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
    input: &FinishComputeCapacityClaim,
    expires_at: Option<&str>,
) -> Result<()> {
    if input.action != ComputeCapacityClaimTerminalAction::Expire {
        return Ok(());
    }
    let expires_at = expires_at.ok_or_else(|| anyhow!("无到期时间的容量 Claim 不能自动过期"))?;
    let occurred_at = parse_utc("容量 Claim 到期发生时间", input.occurred_at.trim())?;
    let expires_at = parse_utc("容量 Claim 到期边界", expires_at)?;
    if occurred_at < expires_at {
        bail!("容量 Claim 尚未到期");
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
