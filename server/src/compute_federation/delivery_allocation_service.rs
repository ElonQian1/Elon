//! Provider, consumer, and administrator orchestration for v228 DeliveryAllocation.

#[cfg(test)]
#[path = "delivery_allocation_store_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "delivery_allocation_reservation_expiry_store_tests.rs"]
mod reservation_expiry_store_tests;

use anyhow::{bail, Result};
use serde::Deserialize;

use crate::store::{
    ComputeDeliveryAllocationDetail, ComputeDeliveryAllocationExerciseWriteReceipt,
    ComputeDeliveryAllocationExpiryReport, ComputeDeliveryAllocationGrantWriteReceipt,
    ComputeDeliveryAllocationReservationExpiryReport,
    ComputeDeliveryAllocationTerminalWriteReceipt, CreateComputeDeliveryAllocationGrant,
    DeclineComputeDeliveryAllocationGrant, ExerciseComputeDeliveryAllocationGrant,
    ExpireDueComputeDeliveryAllocationGrants, ExpireDueComputeDeliveryAllocationReservations,
    Store, COMPUTE_DELIVERY_ALLOCATION_DECLINE_CONFIRMATION,
    COMPUTE_DELIVERY_ALLOCATION_EXERCISE_CONFIRMATION,
    COMPUTE_DELIVERY_ALLOCATION_EXPIRE_DUE_CONFIRMATION,
    COMPUTE_DELIVERY_ALLOCATION_GRANT_CONFIRMATION,
    COMPUTE_DELIVERY_ALLOCATION_RESERVATION_EXPIRE_DUE_CONFIRMATION,
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateDeliveryAllocationGrantBody {
    pub idempotency_key: String,
    pub expected_commitment_revision: i64,
    pub expected_commitment_digest: String,
    pub consumer_account_id: String,
    pub job_id: String,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub confirm_grant: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExerciseDeliveryAllocationGrantBody {
    pub reservation_id: String,
    pub idempotency_key: String,
    pub expected_grant_revision: i64,
    pub expected_grant_digest: String,
    pub confirm_financial_action: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclineDeliveryAllocationGrantBody {
    pub idempotency_key: String,
    pub expected_grant_revision: i64,
    pub expected_grant_digest: String,
    pub confirm_decline: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpireDueDeliveryAllocationGrantsBody {
    pub limit: usize,
    pub confirm_expire_due: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpireDueDeliveryAllocationReservationsBody {
    pub limit: usize,
    pub confirm_expire_due: bool,
}

pub(crate) fn create_for_provider_owner(
    store: &Store,
    provider_owner_account_id: &str,
    provider_id: &str,
    pool_id: &str,
    commitment_id: &str,
    body: CreateDeliveryAllocationGrantBody,
) -> Result<ComputeDeliveryAllocationGrantWriteReceipt> {
    if !body.confirm_grant {
        bail!("创建交付授权前必须显式确认");
    }
    store.create_compute_delivery_allocation_grant(CreateComputeDeliveryAllocationGrant {
        provider_owner_account_id: provider_owner_account_id.to_string(),
        provider_id: provider_id.to_string(),
        pool_id: pool_id.to_string(),
        commitment_id: commitment_id.to_string(),
        expected_commitment_revision: body.expected_commitment_revision,
        expected_commitment_digest: body.expected_commitment_digest,
        consumer_account_id: body.consumer_account_id,
        job_id: body.job_id,
        expected_job_revision: body.expected_job_revision,
        expected_job_digest: body.expected_job_digest,
        idempotency_scope: operation_scope("grant", provider_owner_account_id),
        idempotency_key: body.idempotency_key,
        confirmation: COMPUTE_DELIVERY_ALLOCATION_GRANT_CONFIRMATION.to_string(),
    })
}

pub(crate) fn get_for_provider_owner(
    store: &Store,
    provider_owner_account_id: &str,
    provider_id: &str,
    pool_id: &str,
    commitment_id: &str,
) -> Result<ComputeDeliveryAllocationDetail> {
    store
        .delivery_allocation_grant_for_provider(
            provider_owner_account_id,
            provider_id,
            pool_id,
            commitment_id,
        )?
        .ok_or_else(|| anyhow::anyhow!("交付授权不存在"))
}

pub(crate) fn get_for_consumer(
    store: &Store,
    consumer_account_id: &str,
    grant_id: &str,
) -> Result<ComputeDeliveryAllocationDetail> {
    store
        .delivery_allocation_grant_for_consumer(consumer_account_id, grant_id)?
        .ok_or_else(|| anyhow::anyhow!("交付授权不存在"))
}

pub(crate) fn list_for_consumer(
    store: &Store,
    consumer_account_id: &str,
    status: Option<&str>,
    limit: usize,
) -> Result<Vec<ComputeDeliveryAllocationDetail>> {
    store.list_compute_delivery_allocation_grants_for_consumer(consumer_account_id, status, limit)
}

pub(crate) fn exercise_for_consumer(
    store: &Store,
    consumer_account_id: &str,
    grant_id: &str,
    body: ExerciseDeliveryAllocationGrantBody,
) -> Result<ComputeDeliveryAllocationExerciseWriteReceipt> {
    if !body.confirm_financial_action {
        bail!("行权并冻结预算前必须显式确认");
    }
    store.exercise_compute_delivery_allocation_grant(ExerciseComputeDeliveryAllocationGrant {
        consumer_account_id: consumer_account_id.to_string(),
        grant_id: grant_id.to_string(),
        reservation_id: body.reservation_id,
        expected_grant_revision: body.expected_grant_revision,
        expected_grant_digest: body.expected_grant_digest,
        idempotency_scope: operation_scope("exercise", consumer_account_id),
        idempotency_key: body.idempotency_key,
        confirmation: COMPUTE_DELIVERY_ALLOCATION_EXERCISE_CONFIRMATION.to_string(),
    })
}

pub(crate) fn decline_for_consumer(
    store: &Store,
    consumer_account_id: &str,
    grant_id: &str,
    body: DeclineDeliveryAllocationGrantBody,
) -> Result<ComputeDeliveryAllocationTerminalWriteReceipt> {
    if !body.confirm_decline {
        bail!("拒绝交付授权前必须显式确认");
    }
    store.decline_compute_delivery_allocation_grant(DeclineComputeDeliveryAllocationGrant {
        consumer_account_id: consumer_account_id.to_string(),
        grant_id: grant_id.to_string(),
        expected_grant_revision: body.expected_grant_revision,
        expected_grant_digest: body.expected_grant_digest,
        idempotency_scope: operation_scope("decline", consumer_account_id),
        idempotency_key: body.idempotency_key,
        confirmation: COMPUTE_DELIVERY_ALLOCATION_DECLINE_CONFIRMATION.to_string(),
    })
}

pub(crate) fn expire_due_for_admin(
    store: &Store,
    admin_user_id: &str,
    body: ExpireDueDeliveryAllocationGrantsBody,
) -> Result<ComputeDeliveryAllocationExpiryReport> {
    if !body.confirm_expire_due {
        bail!("执行交付授权到期恢复前必须显式确认");
    }
    store.expire_due_compute_delivery_allocation_grants(ExpireDueComputeDeliveryAllocationGrants {
        admin_user_id: admin_user_id.to_string(),
        limit: body.limit,
        confirmation: COMPUTE_DELIVERY_ALLOCATION_EXPIRE_DUE_CONFIRMATION.to_string(),
    })
}

pub(crate) fn expire_due_reservations_for_admin(
    store: &Store,
    _admin_user_id: &str,
    body: ExpireDueDeliveryAllocationReservationsBody,
) -> Result<ComputeDeliveryAllocationReservationExpiryReport> {
    if !body.confirm_expire_due {
        bail!("执行已行权交付分配的 Reservation 到期恢复前必须显式确认");
    }
    store.expire_due_compute_delivery_allocation_reservations(
        ExpireDueComputeDeliveryAllocationReservations {
            limit: body.limit,
            confirmation: COMPUTE_DELIVERY_ALLOCATION_RESERVATION_EXPIRE_DUE_CONFIRMATION
                .to_string(),
        },
    )
}

fn operation_scope(operation: &str, actor_id: &str) -> String {
    format!("delivery-allocation:{operation}:{actor_id}")
}
