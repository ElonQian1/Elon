use anyhow::{bail, Result};

use crate::{
    compute_federation_offer_lifecycle_model::{
        ComputeOfferLifecycleReceipt, DrainComputeOfferRequest,
    },
    compute_federation_offer_service,
    store::{DrainComputeOffer, Store},
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
