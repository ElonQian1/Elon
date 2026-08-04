use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::compute_federation::{
    capacity::ComputeCapacityClaimState,
    execution::{ATTEMPT_STATUS_RUNNING, JOB_STATUS_RUNNING, RESERVATION_STATUS_ACTIVE},
    provider::{PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_DRAINING},
    receipts::ComputeMeterReading,
};

use super::{
    compute_attempt_activations::compute_attempt_activation_on,
    compute_attempt_leases::{current_lease_state_on, StoredLeaseState},
    compute_capacity_claim_rows::stored_claim_on,
    compute_job_registry::current_registered_job_on,
    compute_provider_registry::current_registered_provider_on,
    compute_reservation_registry::current_registered_reservation_on,
    new_id, Store,
};

mod support;
mod template;

pub(crate) use template::ComputeAttemptUsageTemplateReceipt;

use support::{
    build_contract, build_readings, declaration_by_idempotency_on, declaration_by_sequence_on,
    declaration_event_digest, declaration_request_digest, ensure_exact_meter_contract,
    ensure_monotonic, latest_declaration_on, normalize_declaration, overage_meters, usage_digest,
};

pub(crate) const COMPUTE_ATTEMPT_USAGE_DECLARATION_SCHEMA: &str =
    "compute_federation.attempt_usage_declaration.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeDeclaredUsageInput {
    pub meter: String,
    pub cumulative_quantity: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct DeclareComputeAttemptUsageRequest {
    pub lease_id: String,
    pub provider_id: String,
    pub expected_lease_revision: i64,
    pub expected_lease_digest: String,
    pub expected_fencing_generation: i64,
    pub sequence_no: i64,
    pub executor_usage_ref: String,
    pub cumulative_declared_usage: Vec<ComputeDeclaredUsageInput>,
    pub idempotency_key: String,
    pub declared_by_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ComputeDeclaredUsageContractLine {
    pub meter: String,
    pub reserved_quantity: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptUsageDeclarationReceipt {
    pub schema: &'static str,
    pub snapshot_id: String,
    pub lease_id: String,
    pub provider_id: String,
    pub consumer_account_id: String,
    pub sequence_no: i64,
    pub source_lease_revision: i64,
    pub source_lease_digest: String,
    pub fencing_generation: i64,
    pub job_id: String,
    pub job_revision: i64,
    pub job_digest: String,
    pub reservation_id: String,
    pub reservation_revision: i64,
    pub reservation_digest: String,
    pub capacity_claim_id: String,
    pub capacity_claim_revision: i64,
    pub capacity_claim_digest: String,
    pub executor_usage_ref: String,
    pub cumulative_declared_usage: Vec<ComputeMeterReading>,
    pub cumulative_usage_digest: String,
    pub reserved_contract: Vec<ComputeDeclaredUsageContractLine>,
    pub reserved_contract_digest: String,
    pub overage_meters: Vec<String>,
    pub request_digest: String,
    pub event_digest: String,
    pub declared_by_user_id: String,
    pub declared_at: String,
    pub verification_status: &'static str,
    pub execution_effect: &'static str,
    pub capacity_effect: &'static str,
    pub reservation_effect: &'static str,
    pub money_effect: &'static str,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn declare_compute_attempt_usage(
        &self,
        input: &DeclareComputeAttemptUsageRequest,
    ) -> Result<ComputeAttemptUsageDeclarationReceipt> {
        let input = normalize_declaration(input)?;
        let request_digest = declaration_request_digest(&input)?;
        let idempotency_scope = format!("compute_attempt_usage:{}", input.provider_id);
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            declaration_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同 Attempt 用量声明幂等键不能用于不同请求");
            }
            let receipt = stored.into_receipt(true)?;
            tx.commit()?;
            return Ok(receipt);
        }
        if let Some(stored) = declaration_by_sequence_on(&tx, &input.lease_id, input.sequence_no)? {
            if stored.request_digest != request_digest {
                bail!("同一 Attempt 用量序号已绑定不同声明");
            }
            let receipt = stored.into_receipt(true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let current = current_lease_state_on(&tx, &input.lease_id)?
            .ok_or_else(|| anyhow!("Attempt Lease 当前状态不存在"))?;
        ensure_live_running_lease(&tx, &input, &current)?;
        let activation = compute_attempt_activation_on(&tx, &input.lease_id)?;
        let job = current_registered_job_on(&tx, &current.lease.job_id)?
            .ok_or_else(|| anyhow!("Attempt 对应 Job 不存在"))?;
        let reservation = current_registered_reservation_on(&tx, &current.lease.reservation_id)?
            .ok_or_else(|| anyhow!("Attempt 对应 Reservation 不存在"))?;
        let claim = stored_claim_on(&tx, &activation.active_claim.claim_id)?
            .ok_or_else(|| anyhow!("Attempt 对应 Capacity Claim 不存在"))?;
        ensure_active_bindings(&current, &activation, &job, &reservation, &claim)?;

        let contract = build_contract(&claim.lines)?;
        ensure_exact_meter_contract(&input.cumulative_declared_usage, &contract)?;
        if let Some(previous) = latest_declaration_on(&tx, &input.lease_id)? {
            ensure_monotonic(&input, &previous)?;
        }

        let declared_at = Utc::now().to_rfc3339();
        let readings = build_readings(&input, &declared_at)?;
        let cumulative_usage_digest = usage_digest(&readings)?;
        let reserved_contract_digest = support::contract_digest(&contract)?;
        let overage_meters = overage_meters(&input.cumulative_declared_usage, &contract)?;
        let snapshot_id = new_id("compute_attempt_usage");
        let event_digest = declaration_event_digest(
            &snapshot_id,
            &input,
            &current,
            &job,
            &reservation,
            &claim,
            &cumulative_usage_digest,
            &reserved_contract_digest,
            &overage_meters,
            &request_digest,
            &declared_at,
        )?;

        tx.execute(
            "INSERT INTO compute_attempt_usage_declarations (
                snapshot_id, lease_id, provider_id, consumer_account_id,
                sequence_no, source_lease_revision, source_lease_digest,
                source_lease_status, fencing_generation,
                job_id, job_revision, job_digest,
                reservation_id, reservation_revision, reservation_digest,
                capacity_claim_id, capacity_claim_revision, capacity_claim_digest,
                executor_usage_ref, cumulative_usage_json, cumulative_usage_digest,
                reserved_contract_json, reserved_contract_digest, overage_meters_json,
                request_digest, event_digest, idempotency_scope, idempotency_key,
                declared_by_user_id, declared_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'running', ?8, ?9, ?10,
                       ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                       ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?29)",
            params![
                snapshot_id,
                input.lease_id,
                input.provider_id,
                current.consumer_account_id,
                input.sequence_no,
                current.lease_revision,
                current.lease_digest,
                input.expected_fencing_generation,
                job.job.job_id,
                job.revision,
                job.job_digest,
                reservation.reservation.reservation_id,
                reservation.revision,
                reservation.reservation_digest,
                claim.claim_id,
                claim.revision,
                claim.claim_digest,
                input.executor_usage_ref,
                serde_json::to_string(&readings)?,
                cumulative_usage_digest,
                serde_json::to_string(&contract)?,
                reserved_contract_digest,
                serde_json::to_string(&overage_meters)?,
                request_digest,
                event_digest,
                idempotency_scope,
                input.idempotency_key,
                input.declared_by_user_id,
                declared_at,
            ],
        )?;
        let stored =
            declaration_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
                .ok_or_else(|| anyhow!("Attempt 用量声明写入后不可见"))?;
        let receipt = stored.into_receipt(false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn latest_compute_attempt_usage_declaration(
        &self,
        lease_id: &str,
    ) -> Result<ComputeAttemptUsageDeclarationReceipt> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        latest_declaration_on(&*self.conn()?, lease_id)?
            .ok_or_else(|| anyhow!("Attempt 尚无累计用量声明"))?
            .into_receipt(false)
    }

    pub(crate) fn compute_attempt_usage_declaration(
        &self,
        lease_id: &str,
        sequence_no: i64,
    ) -> Result<ComputeAttemptUsageDeclarationReceipt> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        if sequence_no <= 0 {
            bail!("Attempt 用量声明序号必须为正整数");
        }
        compute_attempt_usage_declaration_on(&*self.conn()?, lease_id, sequence_no)?
            .ok_or_else(|| anyhow!("Attempt 指定序号的用量声明不存在"))
    }
}

pub(super) fn latest_compute_attempt_usage_declaration_on(
    conn: &rusqlite::Connection,
    lease_id: &str,
) -> Result<Option<ComputeAttemptUsageDeclarationReceipt>> {
    latest_declaration_on(conn, lease_id)?
        .map(|stored| stored.into_receipt(false))
        .transpose()
}

pub(crate) fn compute_attempt_usage_declaration_on(
    conn: &rusqlite::Connection,
    lease_id: &str,
    sequence_no: i64,
) -> Result<Option<ComputeAttemptUsageDeclarationReceipt>> {
    support::validate_exact("Attempt Lease ID", lease_id, 200)?;
    if sequence_no <= 0 {
        bail!("Attempt 用量声明序号必须为正整数");
    }
    declaration_by_sequence_on(conn, lease_id, sequence_no)?
        .map(|stored| stored.into_receipt(false))
        .transpose()
}

fn ensure_live_running_lease(
    conn: &rusqlite::Connection,
    input: &DeclareComputeAttemptUsageRequest,
    current: &StoredLeaseState,
) -> Result<()> {
    ensure_live_running_lease_owner(
        conn,
        &input.provider_id,
        &input.declared_by_user_id,
        current,
    )?;
    if current.lease_revision != input.expected_lease_revision
        || current.lease_digest != input.expected_lease_digest
        || current.lease.fencing_generation != input.expected_fencing_generation
    {
        bail!("用量声明必须绑定当前 running Lease 的精确版本、摘要和 fencing 代次");
    }
    Ok(())
}

fn ensure_live_running_lease_owner(
    conn: &rusqlite::Connection,
    provider_id: &str,
    user_id: &str,
    current: &StoredLeaseState,
) -> Result<()> {
    let provider = current_registered_provider_on(conn, provider_id)?
        .ok_or_else(|| anyhow!("Attempt Lease Provider 不存在"))?;
    if provider.provider.owner_account_id != user_id
        || current.provider_id != provider_id
        || !matches!(
            provider.provider.status.as_str(),
            PROVIDER_STATUS_ACTIVE | PROVIDER_STATUS_DRAINING
        )
    {
        bail!("只有当前 Provider 所有者可为 active/draining Provider 声明用量");
    }
    if current.lease.status != ATTEMPT_STATUS_RUNNING || current.lease.last_heartbeat_at.is_none() {
        bail!("只有已心跳的当前 running Lease 可以读取或追加用量声明");
    }
    let now = Utc::now();
    let expires_at = DateTime::parse_from_rfc3339(&current.lease.expires_at)?.with_timezone(&Utc);
    let hard_deadline =
        DateTime::parse_from_rfc3339(&current.lease.hard_deadline_at)?.with_timezone(&Utc);
    if now >= expires_at || now >= hard_deadline {
        bail!("已过期的 Attempt Lease 不能追加用量声明");
    }
    Ok(())
}

fn ensure_active_bindings(
    current: &StoredLeaseState,
    activation: &super::ComputeAttemptActivationReceipt,
    job: &super::compute_job_registry::ComputeJobRegistrationReceipt,
    reservation: &super::compute_reservation_registry::ComputeReservationRegistrationReceipt,
    claim: &crate::compute_federation::capacity::ComputeCapacityClaim,
) -> Result<()> {
    if activation.lease.lease_id != current.lease.lease_id
        || activation.lease.fencing_generation != current.lease.fencing_generation
        || activation.running_job.job_id != job.job.job_id
        || activation.running_job.job_revision != job.revision
        || activation.running_job.job_digest != job.job_digest
        || job.job.status != JOB_STATUS_RUNNING
        || activation.active_reservation_revision != reservation.revision
        || activation.active_reservation_digest != reservation.reservation_digest
        || reservation.reservation.status != RESERVATION_STATUS_ACTIVE
        || activation.active_claim.claim_id != claim.claim_id
        || activation.active_claim.claim_revision != claim.revision
        || activation.active_claim.claim_digest != claim.claim_digest
        || claim.state != ComputeCapacityClaimState::Active
        || reservation.reservation.capacity_claim.claim_id != claim.claim_id
        || reservation.reservation.capacity_claim.claim_revision != claim.revision
        || reservation.reservation.capacity_claim.claim_digest != claim.claim_digest
        || current.consumer_account_id != job.job.consumer_account_id
    {
        bail!("Attempt 用量声明引用的 Job、Reservation 或 Capacity Claim 已漂移");
    }
    Ok(())
}
