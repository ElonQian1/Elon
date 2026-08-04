use anyhow::{bail, Result};
use chrono::Utc;
use serde::Deserialize;

use crate::{
    compute_federation::execution::ComputeReservedCapacity,
    store::{
        ComputeBrokerFinishAction, ComputeBrokerFinishReceipt, ComputeBrokerReservationReceipt,
        FinishComputeBrokerRequest, ReserveComputeBrokerRequest, Store,
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

fn ensure_job_scope(
    store: &Store,
    user_id: &str,
    expected_project_id: Option<&str>,
    job_id: &str,
) -> Result<()> {
    let job = store.compute_job(job_id)?;
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
