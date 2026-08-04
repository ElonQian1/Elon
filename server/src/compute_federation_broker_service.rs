use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

use crate::{
    compute_federation::{
        execution::{
            ComputeJob, ComputeOfferBinding, ComputeProviderScope, ComputeReservedCapacity,
            COMPUTE_JOB_SCHEMA, JOB_STATUS_QUOTED, JOB_STATUS_SUBMITTED,
        },
        workload::ComputeWorkloadSpec,
    },
    store::{
        ComputeBrokerFinishAction, ComputeBrokerFinishReceipt, ComputeBrokerReservationReceipt,
        ComputeJobQuoteCandidatePage, ComputeJobRegistrationReceipt,
        ComputeReservationRegistrationReceipt, FinishComputeBrokerRequest,
        ReserveComputeBrokerRequest, Store,
    },
};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ReserveMyComputeRequest {
    pub reservation_id: String,
    pub idempotency_key: String,
    pub job_id: String,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub reserved_capacity: Vec<ComputeReservedCapacity>,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct FinishMyComputeRequest {
    pub idempotency_key: String,
    pub expected_reservation_revision: i64,
    pub expected_reservation_digest: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateMyComputeJobRequest {
    pub job_id: String,
    pub idempotency_key: String,
    pub merchant_id: Option<String>,
    pub workload: ComputeWorkloadSpec,
    pub provider_scope: ComputeProviderScope,
    pub max_consumer_charge_micros: i64,
    pub currency: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QuoteMyComputeJobRequest {
    pub offer_id: String,
    pub price_snapshot_id: String,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
}

pub(crate) fn create_job_for_project(
    store: &Store,
    user_id: &str,
    project_id: &str,
    request: CreateMyComputeJobRequest,
) -> Result<ComputeJobRegistrationReceipt> {
    if let Some(mut existing) =
        store.compute_job_for_consumer_idempotency(user_id, &request.idempotency_key)?
    {
        let job = &existing.job;
        if job.job_id != request.job_id
            || job.idempotency_key != request.idempotency_key
            || job.project_id.as_deref() != Some(project_id)
            || job.merchant_id != request.merchant_id
            || job.workload != request.workload
            || job.provider_scope != request.provider_scope
            || job.max_consumer_charge_micros != request.max_consumer_charge_micros
            || job.currency != request.currency
        {
            bail!("算力 Job 幂等键已绑定不同的需求合同");
        }
        existing.replayed = true;
        return Ok(existing);
    }
    if let Some(merchant_id) = request.merchant_id.as_deref() {
        store.open_commerce_merchant_for_project(project_id, merchant_id)?;
    }
    let now = Utc::now().to_rfc3339();
    store.register_compute_job(
        &ComputeJob {
            schema: COMPUTE_JOB_SCHEMA.to_string(),
            job_id: request.job_id,
            project_id: Some(project_id.to_string()),
            merchant_id: request.merchant_id,
            consumer_account_id: user_id.to_string(),
            idempotency_key: request.idempotency_key,
            workload: request.workload,
            provider_scope: request.provider_scope,
            status: JOB_STATUS_SUBMITTED.to_string(),
            selected_offer: None,
            price_snapshot_id: None,
            max_consumer_charge_micros: request.max_consumer_charge_micros,
            currency: request.currency,
            submitted_at: now.clone(),
            updated_at: now,
        },
        0,
    )
}

pub(crate) fn quote_job_for_project(
    store: &Store,
    user_id: &str,
    project_id: &str,
    job_id: &str,
    request: QuoteMyComputeJobRequest,
) -> Result<ComputeJobRegistrationReceipt> {
    let source = store.compute_job_version(job_id, request.expected_job_revision)?;
    ensure_job_receipt_scope(&source, user_id, Some(project_id))?;
    if source.job_digest != request.expected_job_digest {
        bail!("算力 Job 历史 digest 不匹配");
    }
    let offer = store.compute_offer(&request.offer_id)?;
    let snapshot = store.compute_price_snapshot(&request.price_snapshot_id)?;
    let mut quoted = source.job;
    quoted.status = JOB_STATUS_QUOTED.to_string();
    quoted.selected_offer = Some(ComputeOfferBinding {
        provider_id: offer.offer.provider_id.clone(),
        offer_id: offer.offer.offer_id.clone(),
        offer_version: offer.offer.offer_version,
        offer_digest: offer.offer.offer_digest.clone(),
    });
    quoted.price_snapshot_id = Some(snapshot.snapshot.snapshot_id);
    quoted.updated_at =
        immutable_timestamp_after(&quoted.updated_at, &snapshot.snapshot.quoted_at)?;
    store.register_compute_job(&quoted, source.revision)
}

pub(crate) fn list_quote_candidates_for_project(
    store: &Store,
    user_id: &str,
    project_id: &str,
    job_id: &str,
    limit: usize,
) -> Result<ComputeJobQuoteCandidatePage> {
    let job = store.compute_job(job_id)?;
    ensure_job_receipt_scope(&job, user_id, Some(project_id))?;
    store.list_compute_job_quote_candidates(&job, limit)
}

pub(crate) fn reserve_for_user(
    store: &Store,
    user_id: &str,
    expected_project_id: Option<&str>,
    request: ReserveMyComputeRequest,
) -> Result<ComputeBrokerReservationReceipt> {
    ensure_job_scope(store, user_id, expected_project_id, &request.job_id)?;
    store.reserve_compute_broker(&ReserveComputeBrokerRequest {
        reservation_id: request.reservation_id,
        consumer_account_id: user_id.to_string(),
        idempotency_key: request.idempotency_key,
        job_id: request.job_id,
        expected_job_revision: request.expected_job_revision,
        expected_job_digest: request.expected_job_digest,
        reserved_capacity: request.reserved_capacity,
        expires_at: request.expires_at,
    })
}

pub(crate) fn finish_for_user(
    store: &Store,
    user_id: &str,
    expected_project_id: Option<&str>,
    reservation_id: String,
    action: ComputeBrokerFinishAction,
    request: FinishMyComputeRequest,
) -> Result<ComputeBrokerFinishReceipt> {
    let reservation = store.compute_reservation(&reservation_id)?;
    ensure_job_scope(
        store,
        user_id,
        expected_project_id,
        &reservation.reservation.job.job_id,
    )?;
    store.finish_compute_broker(&FinishComputeBrokerRequest {
        reservation_id,
        consumer_account_id: user_id.to_string(),
        idempotency_key: request.idempotency_key,
        expected_reservation_revision: request.expected_reservation_revision,
        expected_reservation_digest: request.expected_reservation_digest,
        action,
        occurred_at: Utc::now().to_rfc3339(),
    })
}

pub(crate) fn get_job_for_user(
    store: &Store,
    user_id: &str,
    expected_project_id: Option<&str>,
    job_id: &str,
) -> Result<ComputeJobRegistrationReceipt> {
    let job = store.compute_job(job_id)?;
    ensure_job_receipt_scope(&job, user_id, expected_project_id)?;
    Ok(job)
}

pub(crate) fn list_jobs_for_user(
    store: &Store,
    user_id: &str,
    expected_project_id: Option<&str>,
    limit: usize,
) -> Result<Vec<ComputeJobRegistrationReceipt>> {
    store.list_compute_jobs_for_consumer(user_id, expected_project_id, limit)
}

pub(crate) fn get_reservation_for_user(
    store: &Store,
    user_id: &str,
    expected_project_id: Option<&str>,
    reservation_id: &str,
) -> Result<ComputeReservationRegistrationReceipt> {
    let reservation = store.compute_reservation(reservation_id)?;
    ensure_job_scope(
        store,
        user_id,
        expected_project_id,
        &reservation.reservation.job.job_id,
    )?;
    Ok(reservation)
}

pub(crate) fn list_reservations_for_user(
    store: &Store,
    user_id: &str,
    expected_project_id: Option<&str>,
    limit: usize,
) -> Result<Vec<ComputeReservationRegistrationReceipt>> {
    store.list_compute_reservations_for_consumer(user_id, expected_project_id, limit)
}

fn ensure_job_scope(
    store: &Store,
    user_id: &str,
    expected_project_id: Option<&str>,
    job_id: &str,
) -> Result<()> {
    let job = store.compute_job(job_id)?;
    ensure_job_receipt_scope(&job, user_id, expected_project_id)
}

fn ensure_job_receipt_scope(
    job: &ComputeJobRegistrationReceipt,
    user_id: &str,
    expected_project_id: Option<&str>,
) -> Result<()> {
    if job.job.consumer_account_id != user_id {
        bail!("只能操作当前登录用户自己的算力 Job");
    }
    if let Some(project_id) = expected_project_id {
        if job.job.project_id.as_deref() != Some(project_id) {
            bail!("算力 Job 不属于当前 MCP 项目");
        }
    }
    Ok(())
}

fn immutable_timestamp_after(value: &str, candidate: &str) -> Result<String> {
    let minimum = DateTime::parse_from_rfc3339(value)?
        .with_timezone(&Utc)
        .checked_add_signed(Duration::nanoseconds(1))
        .ok_or_else(|| anyhow::anyhow!("算力 Job 更新时间溢出"))?;
    let candidate = DateTime::parse_from_rfc3339(candidate)?.with_timezone(&Utc);
    Ok(std::cmp::max(candidate, minimum).to_rfc3339())
}
