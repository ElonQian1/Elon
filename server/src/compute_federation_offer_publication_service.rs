use anyhow::{bail, Result};

use crate::{
    compute_federation_offer_publication_model::{
        ComputeOfferPublicationReceipt, PublishComputeOfferDraftRequest,
    },
    compute_federation_offer_service,
    store::{PublishComputeOfferDraft, Store},
};

pub(crate) fn publish_for_review(
    store: &Store,
    actor_user_id: &str,
    offer_id: &str,
    request: PublishComputeOfferDraftRequest,
) -> Result<ComputeOfferPublicationReceipt> {
    if !request.confirm_publish {
        bail!("发布 active Offer 前必须显式确认");
    }
    store.publish_compute_offer_draft(PublishComputeOfferDraft {
        offer_id: offer_id.to_string(),
        expected_offer_version: request.expected_offer_version,
        expected_offer_digest: request.expected_offer_digest,
        idempotency_scope: format!("compute_offer_publish:{offer_id}"),
        idempotency_key: request.idempotency_key,
        approved_by_user_id: actor_user_id.to_string(),
    })
}

pub(crate) fn get_for_review(
    store: &Store,
    offer_id: &str,
) -> Result<ComputeOfferPublicationReceipt> {
    store
        .compute_offer_publication(offer_id)?
        .ok_or_else(|| anyhow::anyhow!("Offer 尚无发布回执"))
}

pub(crate) fn get_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    offer_id: &str,
) -> Result<ComputeOfferPublicationReceipt> {
    compute_federation_offer_service::get_for_user(store, user_id, provider_id, pool_id, offer_id)?;
    let receipt = get_for_review(store, offer_id)?;
    if receipt.provider_id != provider_id || receipt.pool_id != pool_id {
        bail!("Offer 发布回执不属于指定 Provider/CapacityPool");
    }
    Ok(receipt)
}
