use anyhow::{bail, Result};

use crate::{
    compute_federation::offer::{OFFER_STATUS_EXPIRED, OFFER_STATUS_REVOKED},
    compute_federation_offer_lifecycle_model::{
        ComputeOfferLifecycleReceipt, DrainComputeOfferRequest, TerminateComputeOfferRequest,
    },
    compute_federation_offer_service,
    store::{DrainComputeOffer, Store, TerminateComputeOffer},
};

pub(crate) fn drain_for_review(
    store: &Store,
    actor_user_id: &str,
    offer_id: &str,
    request: DrainComputeOfferRequest,
) -> Result<ComputeOfferLifecycleReceipt> {
    if !request.confirm_drain {
        bail!("将 active Offer 转为 draining 前必须显式确认");
    }
    store.drain_compute_offer(DrainComputeOffer {
        offer_id: offer_id.to_string(),
        expected_offer_version: request.expected_offer_version,
        expected_offer_digest: request.expected_offer_digest,
        reason: request.reason,
        idempotency_scope: format!("compute_offer_drain:{offer_id}"),
        idempotency_key: request.idempotency_key,
        changed_by_user_id: actor_user_id.to_string(),
    })
}

pub(crate) fn get_drain_for_review(
    store: &Store,
    offer_id: &str,
) -> Result<ComputeOfferLifecycleReceipt> {
    store
        .compute_offer_drain_event(offer_id)?
        .ok_or_else(|| anyhow::anyhow!("Offer 尚无 draining 回执"))
}

pub(crate) fn get_drain_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    offer_id: &str,
) -> Result<ComputeOfferLifecycleReceipt> {
    compute_federation_offer_service::get_for_user(store, user_id, provider_id, pool_id, offer_id)?;
    let receipt = get_drain_for_review(store, offer_id)?;
    if receipt.provider_id != provider_id || receipt.pool_id != pool_id {
        bail!("Offer draining 回执不属于指定 Provider/CapacityPool");
    }
    Ok(receipt)
}

pub(crate) fn expire_for_review(
    store: &Store,
    actor_user_id: &str,
    offer_id: &str,
    request: TerminateComputeOfferRequest,
) -> Result<ComputeOfferLifecycleReceipt> {
    terminate_for_review(
        store,
        actor_user_id,
        offer_id,
        OFFER_STATUS_EXPIRED,
        request,
    )
}

pub(crate) fn revoke_for_review(
    store: &Store,
    actor_user_id: &str,
    offer_id: &str,
    request: TerminateComputeOfferRequest,
) -> Result<ComputeOfferLifecycleReceipt> {
    terminate_for_review(
        store,
        actor_user_id,
        offer_id,
        OFFER_STATUS_REVOKED,
        request,
    )
}

pub(crate) fn get_terminal_for_review(
    store: &Store,
    offer_id: &str,
    target_status: &str,
) -> Result<ComputeOfferLifecycleReceipt> {
    store
        .compute_offer_terminal_event(offer_id, target_status)?
        .ok_or_else(|| anyhow::anyhow!("Offer 尚无 {target_status} 回执"))
}

pub(crate) fn get_terminal_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    offer_id: &str,
    target_status: &str,
) -> Result<ComputeOfferLifecycleReceipt> {
    compute_federation_offer_service::get_for_user(store, user_id, provider_id, pool_id, offer_id)?;
    let receipt = get_terminal_for_review(store, offer_id, target_status)?;
    if receipt.provider_id != provider_id || receipt.pool_id != pool_id {
        bail!("Offer 终态回执不属于指定 Provider/CapacityPool");
    }
    Ok(receipt)
}

fn terminate_for_review(
    store: &Store,
    actor_user_id: &str,
    offer_id: &str,
    target_status: &str,
    request: TerminateComputeOfferRequest,
) -> Result<ComputeOfferLifecycleReceipt> {
    if !request.confirm_terminal {
        bail!("将 draining Offer 转为 {target_status} 前必须显式确认");
    }
    store.terminate_compute_offer(TerminateComputeOffer {
        offer_id: offer_id.to_string(),
        target_status: target_status.to_string(),
        expected_offer_version: request.expected_offer_version,
        expected_offer_digest: request.expected_offer_digest,
        reason: request.reason,
        idempotency_scope: format!("compute_offer_{target_status}:{offer_id}"),
        idempotency_key: request.idempotency_key,
        changed_by_user_id: actor_user_id.to_string(),
    })
}
