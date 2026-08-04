use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaim, ComputeCapacityClaimLine},
    receipts::ComputeMeterReading,
};

use super::{
    ComputeAttemptUsageDeclarationReceipt, ComputeDeclaredUsageContractLine,
    ComputeDeclaredUsageInput, DeclareComputeAttemptUsageRequest,
    COMPUTE_ATTEMPT_USAGE_DECLARATION_SCHEMA,
};

mod audit;

const JSON_SAFE_SEQUENCE_MAX: i64 = 9_007_199_254_740_991;

#[derive(Debug, Clone)]
pub(super) struct StoredUsageDeclaration {
    snapshot_id: String,
    lease_id: String,
    provider_id: String,
    consumer_account_id: String,
    pub(super) sequence_no: i64,
    source_lease_revision: i64,
    source_lease_digest: String,
    source_lease_status: String,
    fencing_generation: i64,
    job_id: String,
    job_revision: i64,
    job_digest: String,
    reservation_id: String,
    reservation_revision: i64,
    reservation_digest: String,
    capacity_claim_id: String,
    capacity_claim_revision: i64,
    capacity_claim_digest: String,
    executor_usage_ref: String,
    pub(super) cumulative_usage: Vec<ComputeMeterReading>,
    cumulative_usage_digest: String,
    reserved_contract: Vec<ComputeDeclaredUsageContractLine>,
    reserved_contract_digest: String,
    overage_meters: Vec<String>,
    pub(super) request_digest: String,
    event_digest: String,
    idempotency_scope: String,
    idempotency_key: String,
    declared_by_user_id: String,
    declared_at: String,
    created_at: String,
}

impl StoredUsageDeclaration {
    pub(super) fn into_receipt(
        self,
        replayed: bool,
    ) -> Result<ComputeAttemptUsageDeclarationReceipt> {
        audit::audit_declaration(&self)?;
        Ok(ComputeAttemptUsageDeclarationReceipt {
            schema: COMPUTE_ATTEMPT_USAGE_DECLARATION_SCHEMA,
            snapshot_id: self.snapshot_id,
            lease_id: self.lease_id,
            provider_id: self.provider_id,
            consumer_account_id: self.consumer_account_id,
            sequence_no: self.sequence_no,
            source_lease_revision: self.source_lease_revision,
            source_lease_digest: self.source_lease_digest,
            fencing_generation: self.fencing_generation,
            job_id: self.job_id,
            job_revision: self.job_revision,
            job_digest: self.job_digest,
            reservation_id: self.reservation_id,
            reservation_revision: self.reservation_revision,
            reservation_digest: self.reservation_digest,
            capacity_claim_id: self.capacity_claim_id,
            capacity_claim_revision: self.capacity_claim_revision,
            capacity_claim_digest: self.capacity_claim_digest,
            executor_usage_ref: self.executor_usage_ref,
            cumulative_declared_usage: self.cumulative_usage,
            cumulative_usage_digest: self.cumulative_usage_digest,
            reserved_contract: self.reserved_contract,
            reserved_contract_digest: self.reserved_contract_digest,
            overage_meters: self.overage_meters,
            request_digest: self.request_digest,
            event_digest: self.event_digest,
            declared_by_user_id: self.declared_by_user_id,
            declared_at: self.declared_at,
            verification_status: "unverified_provider_declaration",
            execution_effect: "evidence_only",
            capacity_effect: "unchanged",
            reservation_effect: "unchanged",
            money_effect: "preauthorization_unchanged",
            replayed,
        })
    }
}

pub(super) fn normalize_declaration(
    input: &DeclareComputeAttemptUsageRequest,
) -> Result<DeclareComputeAttemptUsageRequest> {
    for (label, value, max_len) in [
        ("Attempt Lease ID", input.lease_id.as_str(), 200),
        ("Provider ID", input.provider_id.as_str(), 160),
        ("执行器用量引用", input.executor_usage_ref.as_str(), 500),
        ("用量声明幂等键", input.idempotency_key.as_str(), 160),
        ("用量声明执行人", input.declared_by_user_id.as_str(), 160),
    ] {
        validate_exact(label, value, max_len)?;
    }
    if input.expected_lease_revision <= 0
        || input.expected_fencing_generation <= 0
        || !(1..=JSON_SAFE_SEQUENCE_MAX).contains(&input.sequence_no)
    {
        bail!("Lease 修订号、fencing 代次和用量序号必须是 JSON 安全正整数");
    }
    validate_digest("预期 Lease 摘要", &input.expected_lease_digest)?;
    if input.cumulative_declared_usage.is_empty() || input.cumulative_declared_usage.len() > 64 {
        bail!("累计用量声明必须包含 1 至 64 个 meter");
    }
    let mut normalized = input.clone();
    normalized
        .cumulative_declared_usage
        .sort_by(|left, right| left.meter.cmp(&right.meter));
    let mut seen = BTreeSet::new();
    for reading in &normalized.cumulative_declared_usage {
        validate_exact("用量 meter", &reading.meter, 120)?;
        if reading.cumulative_quantity < 0 {
            bail!("累计用量不能为负数");
        }
        if !seen.insert(reading.meter.clone()) {
            bail!("累计用量声明不能包含重复 meter");
        }
    }
    Ok(normalized)
}

pub(super) fn build_contract(
    lines: &[ComputeCapacityClaimLine],
) -> Result<Vec<ComputeDeclaredUsageContractLine>> {
    let mut contract = Vec::with_capacity(lines.len());
    let mut seen = BTreeSet::new();
    for line in lines {
        if line.quantity_units < 0 || !seen.insert(line.bucket.meter.clone()) {
            bail!("Capacity Claim meter 合同无效或重复");
        }
        contract.push(ComputeDeclaredUsageContractLine {
            meter: line.bucket.meter.clone(),
            reserved_quantity: line.quantity_units,
        });
    }
    contract.sort_by(|left, right| left.meter.cmp(&right.meter));
    Ok(contract)
}

pub(super) fn ensure_exact_meter_contract(
    readings: &[ComputeDeclaredUsageInput],
    contract: &[ComputeDeclaredUsageContractLine],
) -> Result<()> {
    if readings.len() != contract.len()
        || readings
            .iter()
            .zip(contract)
            .any(|(reading, line)| reading.meter != line.meter)
    {
        bail!("每份累计用量快照必须精确覆盖 Capacity Claim 的全部 meter");
    }
    Ok(())
}

pub(super) fn ensure_monotonic(
    input: &DeclareComputeAttemptUsageRequest,
    previous: &StoredUsageDeclaration,
) -> Result<()> {
    if input.sequence_no <= previous.sequence_no {
        bail!("用量声明序号必须严格递增");
    }
    let previous = previous
        .cumulative_usage
        .iter()
        .map(|reading| (reading.meter.as_str(), reading.quantity))
        .collect::<BTreeMap<_, _>>();
    if input.cumulative_declared_usage.iter().any(|reading| {
        previous
            .get(reading.meter.as_str())
            .is_some_and(|quantity| reading.cumulative_quantity < *quantity)
    }) {
        bail!("同一 Attempt 的累计 meter 读数不能回退");
    }
    Ok(())
}

pub(super) fn build_readings(
    input: &DeclareComputeAttemptUsageRequest,
    declared_at: &str,
) -> Result<Vec<ComputeMeterReading>> {
    input
        .cumulative_declared_usage
        .iter()
        .map(|reading| {
            let reading_digest = digest_json(&serde_json::json!({
                "purpose":"compute_attempt_declared_meter_reading",
                "lease_id":input.lease_id,
                "sequence_no":input.sequence_no,
                "meter":reading.meter,
                "quantity":reading.cumulative_quantity,
                "source_kind":"provider_declared",
                "source_id":input.executor_usage_ref,
                "observed_at":declared_at,
            }))?;
            Ok(ComputeMeterReading {
                meter: reading.meter.clone(),
                quantity: reading.cumulative_quantity,
                source_kind: "provider_declared".to_string(),
                source_id: input.executor_usage_ref.clone(),
                reading_digest,
                observed_at: declared_at.to_string(),
            })
        })
        .collect()
}

pub(super) fn overage_meters(
    readings: &[ComputeDeclaredUsageInput],
    contract: &[ComputeDeclaredUsageContractLine],
) -> Result<Vec<String>> {
    ensure_exact_meter_contract(readings, contract)?;
    Ok(readings
        .iter()
        .zip(contract)
        .filter(|(reading, line)| reading.cumulative_quantity > line.reserved_quantity)
        .map(|(reading, _)| reading.meter.clone())
        .collect())
}

pub(super) fn usage_digest(readings: &[ComputeMeterReading]) -> Result<String> {
    digest_json(&serde_json::json!({
        "purpose":"compute_attempt_cumulative_declared_usage",
        "readings":readings,
    }))
}

pub(super) fn contract_digest(contract: &[ComputeDeclaredUsageContractLine]) -> Result<String> {
    digest_json(&serde_json::json!({
        "purpose":"compute_attempt_reserved_usage_contract",
        "meters":contract,
    }))
}

pub(super) fn declaration_request_digest(
    input: &DeclareComputeAttemptUsageRequest,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "purpose":"compute_attempt_usage_declaration",
        "lease_id":input.lease_id,
        "provider_id":input.provider_id,
        "expected_lease_revision":input.expected_lease_revision,
        "expected_lease_digest":input.expected_lease_digest,
        "expected_fencing_generation":input.expected_fencing_generation,
        "sequence_no":input.sequence_no,
        "executor_usage_ref":input.executor_usage_ref,
        "cumulative_declared_usage":input.cumulative_declared_usage,
        "idempotency_key":input.idempotency_key,
        "declared_by_user_id":input.declared_by_user_id,
    }))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn declaration_event_digest(
    snapshot_id: &str,
    input: &DeclareComputeAttemptUsageRequest,
    current: &crate::store::compute_attempt_leases::StoredLeaseState,
    job: &crate::store::compute_job_registry::ComputeJobRegistrationReceipt,
    reservation: &crate::store::compute_reservation_registry::ComputeReservationRegistrationReceipt,
    claim: &ComputeCapacityClaim,
    usage_digest: &str,
    contract_digest: &str,
    overage_meters: &[String],
    request_digest: &str,
    declared_at: &str,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema":COMPUTE_ATTEMPT_USAGE_DECLARATION_SCHEMA,
        "snapshot_id":snapshot_id,
        "lease_id":input.lease_id,
        "provider_id":input.provider_id,
        "consumer_account_id":current.consumer_account_id,
        "sequence_no":input.sequence_no,
        "source_lease_revision":current.lease_revision,
        "source_lease_digest":current.lease_digest,
        "fencing_generation":input.expected_fencing_generation,
        "job_id":job.job.job_id,
        "job_revision":job.revision,
        "job_digest":job.job_digest,
        "reservation_id":reservation.reservation.reservation_id,
        "reservation_revision":reservation.revision,
        "reservation_digest":reservation.reservation_digest,
        "capacity_claim_id":claim.claim_id,
        "capacity_claim_revision":claim.revision,
        "capacity_claim_digest":claim.claim_digest,
        "executor_usage_ref":input.executor_usage_ref,
        "cumulative_usage_digest":usage_digest,
        "reserved_contract_digest":contract_digest,
        "overage_meters":overage_meters,
        "request_digest":request_digest,
        "declared_by_user_id":input.declared_by_user_id,
        "declared_at":declared_at,
    }))
}

pub(super) fn declaration_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredUsageDeclaration>> {
    conn.query_row(
        &format!(
            "{} WHERE idempotency_scope=?1 AND idempotency_key=?2",
            select_sql()
        ),
        params![scope, key],
        stored_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn declaration_by_sequence_on(
    conn: &Connection,
    lease_id: &str,
    sequence_no: i64,
) -> Result<Option<StoredUsageDeclaration>> {
    conn.query_row(
        &format!("{} WHERE lease_id=?1 AND sequence_no=?2", select_sql()),
        params![lease_id, sequence_no],
        stored_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn latest_declaration_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredUsageDeclaration>> {
    conn.query_row(
        &format!(
            "{} WHERE lease_id=?1 ORDER BY sequence_no DESC LIMIT 1",
            select_sql()
        ),
        params![lease_id],
        stored_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn select_sql() -> &'static str {
    "SELECT snapshot_id, lease_id, provider_id, consumer_account_id,
            sequence_no, source_lease_revision, source_lease_digest,
            source_lease_status, fencing_generation,
            job_id, job_revision, job_digest,
            reservation_id, reservation_revision, reservation_digest,
            capacity_claim_id, capacity_claim_revision, capacity_claim_digest,
            executor_usage_ref, cumulative_usage_json, cumulative_usage_digest,
            reserved_contract_json, reserved_contract_digest, overage_meters_json,
            request_digest, event_digest, idempotency_scope, idempotency_key,
            declared_by_user_id, declared_at, created_at
       FROM compute_attempt_usage_declarations"
}

fn stored_from_row(row: &Row<'_>) -> rusqlite::Result<StoredUsageDeclaration> {
    Ok(StoredUsageDeclaration {
        snapshot_id: row.get(0)?,
        lease_id: row.get(1)?,
        provider_id: row.get(2)?,
        consumer_account_id: row.get(3)?,
        sequence_no: row.get(4)?,
        source_lease_revision: row.get(5)?,
        source_lease_digest: row.get(6)?,
        source_lease_status: row.get(7)?,
        fencing_generation: row.get(8)?,
        job_id: row.get(9)?,
        job_revision: row.get(10)?,
        job_digest: row.get(11)?,
        reservation_id: row.get(12)?,
        reservation_revision: row.get(13)?,
        reservation_digest: row.get(14)?,
        capacity_claim_id: row.get(15)?,
        capacity_claim_revision: row.get(16)?,
        capacity_claim_digest: row.get(17)?,
        executor_usage_ref: row.get(18)?,
        cumulative_usage: parse_json(row, 19)?,
        cumulative_usage_digest: row.get(20)?,
        reserved_contract: parse_json(row, 21)?,
        reserved_contract_digest: row.get(22)?,
        overage_meters: parse_json(row, 23)?,
        request_digest: row.get(24)?,
        event_digest: row.get(25)?,
        idempotency_scope: row.get(26)?,
        idempotency_key: row.get(27)?,
        declared_by_user_id: row.get(28)?,
        declared_at: row.get(29)?,
        created_at: row.get(30)?,
    })
}

fn parse_json<T: serde::de::DeserializeOwned>(row: &Row<'_>, index: usize) -> rusqlite::Result<T> {
    let value: String = row.get(index)?;
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

pub(super) fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max_len
        || value.chars().any(char::is_control)
    {
        bail!("{label}为空、过长或包含无效字符");
    }
    Ok(())
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || value != value.to_ascii_lowercase()
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("{label}必须是 64 位小写十六进制 SHA-256");
    }
    Ok(())
}

fn digest_json(value: &impl Serialize) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(value)?)))
}
