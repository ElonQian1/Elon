use anyhow::Result;
use chrono::Utc;
use serde::Deserialize;

use crate::{
    compute_federation::execution::ComputeReservedCapacity,
    store::{
        ComputeBrokerFinishAction, ComputeBrokerFinishReceipt, ComputeBrokerReservationReceipt,
        FinishComputeBrokerRequest, ReserveComputeBrokerRequest,
    },
    types::AppState,
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
    state: &AppState,
    user_id: &str,
    request: ReserveMyComputeRequest,
) -> Result<ComputeBrokerReservationReceipt> {
    state
        .store
        .reserve_compute_broker(&ReserveComputeBrokerRequest {
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
    state: &AppState,
    user_id: &str,
    reservation_id: String,
    action: ComputeBrokerFinishAction,
    request: FinishMyComputeRequest,
) -> Result<ComputeBrokerFinishReceipt> {
    state
        .store
        .finish_compute_broker(&FinishComputeBrokerRequest {
            reservation_id,
            consumer_account_id: user_id.to_string(),
            idempotency_key: request.idempotency_key,
            expected_reservation_revision: request.expected_reservation_revision,
            expected_reservation_digest: request.expected_reservation_digest,
            action,
            occurred_at: Utc::now().to_rfc3339(),
        })
}
