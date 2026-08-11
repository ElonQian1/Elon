//! Owner and administrator orchestration for Provider Capacity Commitments.

#[cfg(test)]
#[path = "capacity_commitment_test_support.rs"]
pub(crate) mod test_support;
#[cfg(test)]
#[path = "capacity_commitment_store_tests.rs"]
mod tests;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::{
    compute_federation::{
        capacity::ComputeCapacityPoolBinding,
        capacity_commitment::ComputeCapacityCommitmentQuantity,
        market::ComputeDeliveryWindowBinding,
    },
    compute_federation_price_snapshot_service,
    store::{
        CancelComputeCapacityCommitment, ComputeCapacityCommitmentCreateReceipt,
        ComputeCapacityCommitmentDetail, ComputeCapacityCommitmentExpiryReport,
        ComputeCapacityCommitmentTerminalWriteReceipt,
        ComputePlatformReferencePriceCurveSnapshotBindingReceipt, CreateComputeCapacityCommitment,
        ExpireDueComputeCapacityCommitments, Store,
        COMPUTE_CAPACITY_COMMITMENT_CANCEL_CONFIRMATION,
        COMPUTE_CAPACITY_COMMITMENT_CREATE_CONFIRMATION,
        COMPUTE_CAPACITY_COMMITMENT_EXPIRE_DUE_CONFIRMATION,
    },
};

#[derive(Clone, Serialize)]
pub(crate) struct CapacityCommitmentSourceView {
    pub snapshot: crate::compute_federation::market::ComputePriceSnapshot,
    pub reference_binding: ComputePlatformReferencePriceCurveSnapshotBindingReceipt,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateCapacityCommitmentBody {
    pub idempotency_key: String,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
    pub capacity_epoch: i64,
    pub pool_revision: i64,
    pub pool_digest: String,
    pub delivery_window_id: String,
    pub delivery_window_digest: String,
    pub price_snapshot_id: String,
    pub price_snapshot_digest: String,
    pub reference_binding_id: String,
    pub reference_binding_digest: String,
    pub instrument_id: String,
    pub quantities: Vec<ComputeCapacityCommitmentQuantity>,
    pub confirm_commitment: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CancelCapacityCommitmentBody {
    pub idempotency_key: String,
    pub expected_commitment_revision: i64,
    pub expected_commitment_digest: String,
    #[serde(default)]
    pub reason: String,
    pub confirm_cancel: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExpireDueCapacityCommitmentsBody {
    pub limit: usize,
    pub confirm_expire_due: bool,
}

pub(crate) fn create_for_owner(
    store: &Store,
    owner_account_id: &str,
    provider_id: &str,
    pool_id: &str,
    body: CreateCapacityCommitmentBody,
) -> Result<ComputeCapacityCommitmentCreateReceipt> {
    if !body.confirm_commitment {
        bail!("创建容量承诺前必须显式确认");
    }
    store.create_compute_capacity_commitment(CreateComputeCapacityCommitment {
        owner_account_id: owner_account_id.to_string(),
        provider_id: provider_id.to_string(),
        provider_policy_revision: body.provider_policy_revision,
        provider_digest: body.provider_digest,
        offer_id: body.offer_id,
        offer_version: body.offer_version,
        offer_digest: body.offer_digest,
        pool: ComputeCapacityPoolBinding {
            pool_id: pool_id.to_string(),
            capacity_epoch: body.capacity_epoch,
            pool_revision: body.pool_revision,
            pool_digest: body.pool_digest,
        },
        delivery_window: ComputeDeliveryWindowBinding {
            window_id: body.delivery_window_id,
            window_digest: body.delivery_window_digest,
        },
        price_snapshot_id: body.price_snapshot_id,
        price_snapshot_digest: body.price_snapshot_digest,
        reference_binding_id: body.reference_binding_id,
        reference_binding_digest: body.reference_binding_digest,
        instrument_id: body.instrument_id,
        quantities: body.quantities,
        idempotency_scope: operation_scope("create", owner_account_id),
        idempotency_key: body.idempotency_key,
        confirmation: COMPUTE_CAPACITY_COMMITMENT_CREATE_CONFIRMATION.to_string(),
    })
}

pub(crate) fn source_for_owner(
    store: &Store,
    owner_account_id: &str,
    provider_id: &str,
    pool_id: &str,
    offer_id: &str,
    snapshot_id: &str,
) -> Result<CapacityCommitmentSourceView> {
    let snapshot = compute_federation_price_snapshot_service::get_for_user(
        store,
        owner_account_id,
        provider_id,
        pool_id,
        offer_id,
        snapshot_id,
    )?
    .snapshot;
    let reference_binding = store
        .platform_reference_snapshot_binding(snapshot_id)?
        .ok_or_else(|| anyhow::anyhow!("价格快照尚未获得平台参考价格审核绑定"))?;
    if reference_binding.snapshot_id != snapshot.snapshot_id
        || reference_binding.snapshot_digest != snapshot.snapshot_digest
    {
        bail!("平台参考价格绑定与价格快照不一致");
    }
    Ok(CapacityCommitmentSourceView {
        snapshot,
        reference_binding,
    })
}

pub(crate) fn get_for_owner(
    store: &Store,
    owner_account_id: &str,
    provider_id: &str,
    pool_id: &str,
    commitment_id: &str,
) -> Result<ComputeCapacityCommitmentDetail> {
    store
        .compute_capacity_commitment_for_owner(
            owner_account_id,
            provider_id,
            pool_id,
            commitment_id,
        )?
        .ok_or_else(|| anyhow::anyhow!("容量承诺不存在"))
}

pub(crate) fn list_for_owner(
    store: &Store,
    owner_account_id: &str,
    provider_id: &str,
    pool_id: &str,
    status: Option<&str>,
    limit: usize,
) -> Result<Vec<ComputeCapacityCommitmentDetail>> {
    store.list_compute_capacity_commitments_for_owner(
        owner_account_id,
        provider_id,
        pool_id,
        status,
        limit,
    )
}

pub(crate) fn cancel_for_owner(
    store: &Store,
    owner_account_id: &str,
    provider_id: &str,
    pool_id: &str,
    commitment_id: &str,
    body: CancelCapacityCommitmentBody,
) -> Result<ComputeCapacityCommitmentTerminalWriteReceipt> {
    if !body.confirm_cancel {
        bail!("取消容量承诺前必须显式确认");
    }
    store.cancel_compute_capacity_commitment(CancelComputeCapacityCommitment {
        owner_account_id: owner_account_id.to_string(),
        provider_id: provider_id.to_string(),
        pool_id: pool_id.to_string(),
        commitment_id: commitment_id.to_string(),
        expected_commitment_revision: body.expected_commitment_revision,
        expected_commitment_digest: body.expected_commitment_digest,
        reason: body.reason,
        idempotency_scope: operation_scope("cancel", owner_account_id),
        idempotency_key: body.idempotency_key,
        confirmation: COMPUTE_CAPACITY_COMMITMENT_CANCEL_CONFIRMATION.to_string(),
    })
}

pub(crate) fn expire_due_for_admin(
    store: &Store,
    admin_user_id: &str,
    body: ExpireDueCapacityCommitmentsBody,
) -> Result<ComputeCapacityCommitmentExpiryReport> {
    if !body.confirm_expire_due {
        bail!("执行容量承诺到期恢复前必须显式确认");
    }
    store.expire_due_compute_capacity_commitments(ExpireDueComputeCapacityCommitments {
        admin_user_id: admin_user_id.to_string(),
        limit: body.limit,
        confirmation: COMPUTE_CAPACITY_COMMITMENT_EXPIRE_DUE_CONFIRMATION.to_string(),
    })
}

fn operation_scope(operation: &str, actor_id: &str) -> String {
    format!("capacity-commitment:{operation}:{actor_id}")
}
