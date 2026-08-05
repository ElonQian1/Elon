use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaimState, ComputeCapacityMeterMode},
    execution::{
        ComputeReservedCapacity, ATTEMPT_STATUS_TERMINAL, JOB_STATUS_RUNNING,
        JOB_STATUS_VERIFICATION_PENDING, RESERVATION_STATUS_ACTIVE, RESERVATION_STATUS_CONSUMED,
    },
};

use super::{
    finalization_event_digest, finalization_request_digest, normalize_finalization_request,
    StoredFinalization,
};
use crate::store::{
    compute_attempt_execution_receipts::compute_attempt_execution_receipt_on,
    compute_attempt_finalizations::{
        capacity::receipt_capacity_is_consistent, ComputeAttemptFinalizationReceipt,
        COMPUTE_ATTEMPT_FINALIZATION_SCHEMA,
    },
    compute_attempt_leases::current_lease_state_on,
    compute_attempt_terminals::compute_attempt_terminal_candidate_on,
    compute_capacity_claim_rows::{stored_claim_on, stored_claim_version_on},
    compute_job_registry::registered_job_version_on,
    compute_reservation_registry::{
        current_registered_reservation_on, registered_reservation_version_on,
    },
};

impl StoredFinalization {
    pub(in crate::store::compute_attempt_finalizations) fn into_receipt(
        self,
        conn: &Connection,
        replayed: bool,
    ) -> Result<ComputeAttemptFinalizationReceipt> {
        let normalized = normalize_finalization_request(&self.request)?;
        if normalized != self.request
            || finalization_request_digest(&normalized)? != self.request_digest
            || self.receipt.request_digest != self.request_digest
            || self.receipt.schema != COMPUTE_ATTEMPT_FINALIZATION_SCHEMA
            || self.receipt.finalization_id != self.finalization_id
            || self.receipt.lease_id != self.lease_id
            || self.receipt.execution_receipt_id != self.execution_receipt_id
            || self.receipt.execution_receipt_digest != self.execution_receipt_digest
            || self.receipt.event_digest != self.event_digest
            || self.receipt.finalized_by_user_id != self.finalized_by_user_id
            || self.receipt.effective_at != self.effective_at
            || self.receipt.finalized_at != self.finalized_at
            || self.created_at != self.finalized_at
            || self.idempotency_key != self.request.idempotency_key
            || self.idempotency_scope
                != format!(
                    "compute_attempt_finalization:{}",
                    self.request.finalized_by_user_id
                )
            || self.receipt.replayed
            || finalization_event_digest(&self.receipt)? != self.event_digest
        {
            bail!("Attempt 可信终态持久化字段或摘要审计失败");
        }
        ensure_request_bindings(&self.request, &self.receipt)?;
        audit_source_and_target(conn, &self.receipt)?;
        audit_capacity_transactions(conn, &self.receipt)?;
        audit_times(&self.receipt)?;

        let mut receipt = self.receipt;
        receipt.replayed = replayed;
        Ok(receipt)
    }
}

fn ensure_request_bindings(
    request: &super::super::FinalizeComputeAttemptRequest,
    receipt: &ComputeAttemptFinalizationReceipt,
) -> Result<()> {
    if request.lease_id != receipt.lease_id
        || request.expected_execution_receipt_id != receipt.execution_receipt_id
        || request.expected_execution_receipt_digest != receipt.execution_receipt_digest
        || request.expected_lease_revision != receipt.source_lease.revision
        || request.expected_lease_digest != receipt.source_lease.digest
        || request.expected_job_revision != receipt.source_job.job_revision
        || request.expected_job_digest != receipt.source_job.job_digest
        || request.expected_reservation_revision != receipt.source_reservation.revision
        || request.expected_reservation_digest != receipt.source_reservation.digest
        || request.expected_claim_revision != receipt.source_claim.claim_revision
        || request.expected_claim_digest != receipt.source_claim.claim_digest
        || request.finalized_by_user_id != receipt.finalized_by_user_id
        || request.expected_fencing_generation <= 0
        || receipt.terminal_lease.revision != receipt.source_lease.revision + 1
        || receipt.terminal_job.job_revision != receipt.source_job.job_revision + 1
        || receipt.terminal_reservation.revision != receipt.source_reservation.revision + 1
        || receipt.terminal_claim.claim_revision != receipt.source_claim.claim_revision + 1
    {
        bail!("Attempt 可信终态请求与源/目标版本绑定不一致");
    }
    Ok(())
}

fn audit_source_and_target(
    conn: &Connection,
    receipt: &ComputeAttemptFinalizationReceipt,
) -> Result<()> {
    let execution = compute_attempt_execution_receipt_on(conn, &receipt.lease_id)?;
    let candidate = compute_attempt_terminal_candidate_on(conn, &receipt.lease_id)?
        .ok_or_else(|| anyhow!("Attempt 可信终态引用的 Provider 候选不存在"))?;
    if execution.receipt.receipt_id != receipt.execution_receipt_id
        || execution.receipt.receipt_digest != receipt.execution_receipt_digest
        || execution.receipt.execution_status != receipt.outcome
        || execution.receipt.finished_at != receipt.effective_at
        || candidate.outcome != receipt.outcome
        || candidate.reason_code != receipt.reason_code
        || candidate.declared_at != receipt.effective_at
        || candidate.provider_id != receipt.provider_id
        || candidate.consumer_account_id != receipt.consumer_account_id
        || candidate.source_lease_revision != receipt.source_lease.revision
        || candidate.source_lease_digest != receipt.source_lease.digest
    {
        bail!("Attempt 可信终态与 v193 回执或终态候选不一致");
    }

    let lease = current_lease_state_on(conn, &receipt.lease_id)?
        .ok_or_else(|| anyhow!("Attempt 可信终态对应 Lease 不存在"))?;
    if lease.lease_revision != receipt.terminal_lease.revision
        || lease.lease_digest != receipt.terminal_lease.digest
        || lease.lease.status != ATTEMPT_STATUS_TERMINAL
        || lease.lease.terminal_reason_code.as_deref() != Some(receipt.reason_code.as_str())
        || lease.updated_at != receipt.effective_at
    {
        bail!("Attempt Lease 当前可信终态与回执不一致");
    }

    let source_job = registered_job_version_on(
        conn,
        &receipt.source_job.job_id,
        receipt.source_job.job_revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 可信终态源 Job 历史版本不存在"))?;
    let terminal_job = registered_job_version_on(
        conn,
        &receipt.terminal_job.job_id,
        receipt.terminal_job.job_revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 可信终态目标 Job 历史版本不存在"))?;
    if source_job.job_digest != receipt.source_job.job_digest
        || source_job.job.status != JOB_STATUS_RUNNING
        || terminal_job.job_digest != receipt.terminal_job.job_digest
        || terminal_job.job.status != JOB_STATUS_VERIFICATION_PENDING
        || terminal_job.job.updated_at != receipt.effective_at
    {
        bail!("Attempt 可信终态 Job 历史版本审计失败");
    }

    let source_reservation = registered_reservation_version_on(
        conn,
        &execution.receipt.reservation_id,
        receipt.source_reservation.revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 可信终态源 Reservation 历史版本不存在"))?;
    let terminal_reservation = registered_reservation_version_on(
        conn,
        &source_reservation.reservation.reservation_id,
        receipt.terminal_reservation.revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 可信终态目标 Reservation 历史版本不存在"))?;
    let current_reservation =
        current_registered_reservation_on(conn, &source_reservation.reservation.reservation_id)?
            .ok_or_else(|| anyhow!("Attempt 可信终态当前 Reservation 不存在"))?;
    if source_reservation.reservation_digest != receipt.source_reservation.digest
        || source_reservation.reservation.status != RESERVATION_STATUS_ACTIVE
        || terminal_reservation.reservation_digest != receipt.terminal_reservation.digest
        || terminal_reservation.reservation.status != RESERVATION_STATUS_CONSUMED
        || terminal_reservation.reservation.updated_at != receipt.effective_at
        || terminal_reservation.reservation.consumed_at.as_deref()
            != Some(receipt.effective_at.as_str())
        || terminal_reservation.reservation.job != receipt.terminal_job
        || terminal_reservation.reservation.capacity_claim != receipt.terminal_claim
        || current_reservation.revision != receipt.terminal_reservation.revision
        || current_reservation.reservation_digest != receipt.terminal_reservation.digest
    {
        bail!("Attempt 可信终态 Reservation 历史或当前版本审计失败");
    }

    let source_claim = stored_claim_version_on(
        conn,
        &receipt.source_claim.claim_id,
        receipt.source_claim.claim_revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 可信终态源 Capacity Claim 历史版本不存在"))?;
    let terminal_claim = stored_claim_version_on(
        conn,
        &receipt.terminal_claim.claim_id,
        receipt.terminal_claim.claim_revision,
    )?
    .ok_or_else(|| anyhow!("Attempt 可信终态目标 Capacity Claim 历史版本不存在"))?;
    let current_claim = stored_claim_on(conn, &receipt.terminal_claim.claim_id)?
        .ok_or_else(|| anyhow!("Attempt 可信终态当前 Capacity Claim 不存在"))?;
    if source_claim.claim_digest != receipt.source_claim.claim_digest
        || source_claim.state != ComputeCapacityClaimState::Active
        || terminal_claim.claim_digest != receipt.terminal_claim.claim_digest
        || terminal_claim.state != ComputeCapacityClaimState::Consumed
        || terminal_claim.updated_at != receipt.effective_at
        || terminal_claim.terminal_at.as_deref() != Some(receipt.effective_at.as_str())
        || current_claim.revision != receipt.terminal_claim.claim_revision
        || current_claim.claim_digest != receipt.terminal_claim.claim_digest
    {
        bail!("Attempt 可信终态 Capacity Claim 历史或当前版本审计失败");
    }
    audit_capacity_shape(
        &execution.receipt.usage.compensable_usage,
        &source_claim,
        receipt,
    )
}

fn audit_capacity_shape(
    execution_usage: &[crate::compute_federation::receipts::ComputeMeterReading],
    source_claim: &crate::compute_federation::capacity::ComputeCapacityClaim,
    receipt: &ComputeAttemptFinalizationReceipt,
) -> Result<()> {
    let execution = execution_usage
        .iter()
        .map(|reading| (reading.meter.as_str(), reading.quantity))
        .collect::<BTreeMap<_, _>>();
    let compensable = meter_reading_map(&receipt.compensable_usage)?;
    let consumed = capacity_map(&receipt.capacity_consumed)?;
    let returned = capacity_map(&receipt.capacity_returned)?;
    if execution != compensable || !receipt_capacity_is_consistent(receipt) {
        bail!("Attempt 可信终态 compensable usage 或容量效果不一致");
    }
    for line in &source_claim.lines {
        let usage = *compensable.get(line.bucket.meter.as_str()).unwrap_or(&0);
        let expected_consumed = match line.bucket.meter_mode {
            ComputeCapacityMeterMode::Consumable => usage,
            ComputeCapacityMeterMode::Reusable => 0,
        };
        let expected_returned = line.quantity_units - expected_consumed;
        if consumed
            .get(line.bucket.meter.as_str())
            .copied()
            .unwrap_or(0)
            != expected_consumed
            || returned
                .get(line.bucket.meter.as_str())
                .copied()
                .unwrap_or(0)
                != expected_returned
        {
            bail!("Attempt 可信终态容量消费与归还数量无法由 Claim 重建");
        }
    }
    Ok(())
}

fn audit_capacity_transactions(
    conn: &Connection,
    receipt: &ComputeAttemptFinalizationReceipt,
) -> Result<()> {
    let mut ids = BTreeSet::new();
    let mut ledger_consumed = BTreeMap::new();
    let mut ledger_returned = BTreeMap::new();
    for reference in &receipt.capacity_transactions {
        if !ids.insert(reference.transaction_id.as_str()) {
            bail!("Attempt 可信终态容量事务重复");
        }
        let row = conn
            .query_row(
                "SELECT transaction_digest, ledger_sequence, event_kind, claim_id,
                        request_digest, subject_kind, subject_id
                   FROM compute_capacity_ledger_transactions WHERE transaction_id=?1",
                params![reference.transaction_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("Attempt 可信终态容量事务不存在"))?;
        if row.0 != reference.transaction_digest
            || row.1 != reference.ledger_sequence
            || row.2 != reference.event_kind
            || row.3.as_deref() != Some(receipt.terminal_claim.claim_id.as_str())
            || row.4 != receipt.request_digest
            || row.5 != "compute_execution_receipt"
            || row.6 != receipt.execution_receipt_id
        {
            bail!("Attempt 可信终态容量事务绑定或摘要审计失败");
        }
        let target = match reference.event_kind.as_str() {
            "usage_consumed" => &mut ledger_consumed,
            "attempt_returned" => &mut ledger_returned,
            _ => bail!("Attempt 可信终态容量事务类型不受支持"),
        };
        let mut statement = conn.prepare(
            "SELECT meter, account, delta_units FROM compute_capacity_ledger_legs
              WHERE transaction_id=?1 AND leg_role='to' ORDER BY line_no",
        )?;
        let rows = statement.query_map(params![reference.transaction_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (meter, account, quantity) = row?;
            let expected_account = if reference.event_kind == "usage_consumed" {
                "consumed"
            } else {
                "available"
            };
            if account != expected_account
                || quantity <= 0
                || target.insert(meter, quantity).is_some()
            {
                bail!("Attempt 可信终态容量分录无效或 meter 重复");
            }
        }
    }
    if ledger_consumed != owned_capacity_map(&receipt.capacity_consumed)?
        || ledger_returned != owned_capacity_map(&receipt.capacity_returned)?
    {
        bail!("Attempt 可信终态容量分录数量与回执不一致");
    }
    Ok(())
}

fn audit_times(receipt: &ComputeAttemptFinalizationReceipt) -> Result<()> {
    let effective = parse_utc("可信终态生效时间", &receipt.effective_at)?;
    let finalized = parse_utc("可信终态登记时间", &receipt.finalized_at)?;
    if finalized < effective
        || receipt.execution_effect != "trusted_terminal_applied"
        || receipt.lease_effect != "terminal"
        || receipt.job_effect != "verification_pending"
        || receipt.reservation_effect != "consumed"
        || receipt.money_effect != "preauthorization_unchanged"
        || receipt.settlement_effect != "pending"
    {
        bail!("Attempt 可信终态时间或效果字段无效");
    }
    Ok(())
}

fn capacity_map(values: &[ComputeReservedCapacity]) -> Result<BTreeMap<&str, i64>> {
    let mut result = BTreeMap::new();
    for value in values {
        if value.meter.trim().is_empty()
            || value.quantity < 0
            || result
                .insert(value.meter.as_str(), value.quantity)
                .is_some()
        {
            bail!("Attempt 可信终态容量列表无效");
        }
    }
    Ok(result)
}

fn meter_reading_map(
    values: &[crate::compute_federation::receipts::ComputeMeterReading],
) -> Result<BTreeMap<&str, i64>> {
    let mut result = BTreeMap::new();
    for value in values {
        if value.meter.trim().is_empty()
            || value.quantity < 0
            || result
                .insert(value.meter.as_str(), value.quantity)
                .is_some()
        {
            bail!("Attempt 可补偿用量列表无效");
        }
    }
    Ok(result)
}

fn owned_capacity_map(values: &[ComputeReservedCapacity]) -> Result<BTreeMap<String, i64>> {
    capacity_map(values).map(|map| {
        map.into_iter()
            .map(|(meter, quantity)| (meter.to_string(), quantity))
            .collect()
    })
}

fn parse_utc(label: &str, value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("{label} 不是 RFC3339"))?
        .with_timezone(&Utc))
}
