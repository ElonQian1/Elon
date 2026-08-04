use std::collections::BTreeSet;

use anyhow::{bail, Result};
use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        capacity::ComputeCapacityPoolStatus,
        offer::{ComputeOffer, OFFER_STATUS_DRAFT},
        provider::PROVIDER_STATUS_ACTIVE,
    },
    compute_federation_capacity_pool_service,
    compute_federation_offer_draft_builder::{build_offer_draft, ResolvedDraftCapacity},
    compute_federation_offer_draft_model::CreateMyComputeOfferDraftRequest,
    store::{ComputeOfferRegistrationReceipt, Store},
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MyComputeOfferView {
    pub offer: ComputeOffer,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub replayed: bool,
    pub market_effect: &'static str,
}

pub(crate) fn create_draft_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    request: CreateMyComputeOfferDraftRequest,
) -> Result<MyComputeOfferView> {
    if !request.confirm_create {
        bail!("创建 draft Offer 前必须显式确认");
    }
    validate_exact("draft Offer 幂等键", &request.idempotency_key, 160)?;
    if request.capacity.is_empty() || request.capacity.len() > 256 {
        bail!("draft Offer 容量行数量必须在 1 到 256 之间");
    }
    let provider = owned_active_provider(store, user_id, provider_id)?;
    let pool = compute_federation_capacity_pool_service::owned_pool_for_user(
        store,
        user_id,
        provider_id,
        pool_id,
    )?;
    if pool.status != ComputeCapacityPoolStatus::Active {
        bail!("只有 active CapacityPool 可以创建 draft Offer");
    }

    let offer_id = deterministic_offer_id(
        user_id,
        provider_id,
        pool_id,
        request.idempotency_key.as_str(),
    )?;
    let existing = store.compute_offer_if_exists(&offer_id)?;
    let created_at = existing
        .as_ref()
        .map(|receipt| receipt.offer.created_at.clone())
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let resolved_capacity = resolve_capacity(store, &request)?;
    let candidate = build_offer_draft(
        offer_id,
        created_at,
        &provider.provider,
        &pool,
        &request,
        resolved_capacity,
    )?;

    if let Some(existing) = existing {
        ensure_offer_scope(&existing.offer, provider_id, pool_id)?;
        if existing.offer.status != OFFER_STATUS_DRAFT || existing.offer != candidate {
            bail!("draft Offer 幂等键已绑定不同合同");
        }
        return Ok(offer_view(existing, true));
    }
    let receipt = store.register_compute_offer(&candidate)?;
    Ok(offer_view(receipt, false))
}

pub(crate) fn get_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    offer_id: &str,
) -> Result<MyComputeOfferView> {
    owned_active_or_historical_provider(store, user_id, provider_id)?;
    validate_exact("算力 Offer ID", offer_id, 200)?;
    let receipt = store
        .compute_offer_if_exists(offer_id)?
        .ok_or_else(|| anyhow::anyhow!("算力 Offer 不存在"))?;
    ensure_offer_scope(&receipt.offer, provider_id, pool_id)?;
    Ok(offer_view(receipt, false))
}

pub(crate) fn list_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    limit: usize,
) -> Result<Vec<MyComputeOfferView>> {
    owned_active_or_historical_provider(store, user_id, provider_id)?;
    store
        .list_compute_offers_for_provider(provider_id, pool_id, limit)?
        .into_iter()
        .map(|receipt| {
            ensure_offer_scope(&receipt.offer, provider_id, pool_id)?;
            Ok(offer_view(receipt, false))
        })
        .collect()
}

fn resolve_capacity<'a>(
    store: &Store,
    request: &'a CreateMyComputeOfferDraftRequest,
) -> Result<Vec<ResolvedDraftCapacity<'a>>> {
    let mut bucket_ids = BTreeSet::new();
    request
        .capacity
        .iter()
        .map(|input| {
            validate_exact("容量 bucket ID", &input.bucket_id, 160)?;
            if !bucket_ids.insert(input.bucket_id.as_str()) {
                bail!("draft Offer 容量 bucket 不能重复");
            }
            Ok(ResolvedDraftCapacity {
                input,
                bucket: store.compute_capacity_bucket(&input.bucket_id)?,
            })
        })
        .collect()
}

fn owned_active_provider(
    store: &Store,
    user_id: &str,
    provider_id: &str,
) -> Result<crate::store::ComputeProviderRegistrationReceipt> {
    let provider = owned_active_or_historical_provider(store, user_id, provider_id)?;
    if provider.provider.status != PROVIDER_STATUS_ACTIVE
        || (provider.provider.endpoint.is_none() && provider.provider.adapter.is_none())
        || provider
            .provider
            .evidence_profile
            .verified_hardware_digest
            .is_none()
        || provider
            .provider
            .evidence_profile
            .last_verified_at
            .is_none()
        || provider.provider.trust_tier == "self_declared"
    {
        bail!("只有具备路由和 verified 证据的 active Provider 可以创建 draft Offer");
    }
    Ok(provider)
}

fn owned_active_or_historical_provider(
    store: &Store,
    user_id: &str,
    provider_id: &str,
) -> Result<crate::store::ComputeProviderRegistrationReceipt> {
    validate_exact("算力 Provider ID", provider_id, 160)?;
    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    Ok(provider)
}

fn deterministic_offer_id(
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    idempotency_key: &str,
) -> Result<String> {
    for (label, value) in [
        ("当前用户 ID", user_id),
        ("算力 Provider ID", provider_id),
        ("容量池 ID", pool_id),
        ("draft Offer 幂等键", idempotency_key),
    ] {
        validate_exact(label, value, 160)?;
    }
    let scope = serde_json::json!({
        "purpose":"compute_offer_draft_create",
        "user_id":user_id,
        "provider_id":provider_id,
        "pool_id":pool_id,
        "idempotency_key":idempotency_key,
    });
    Ok(format!(
        "offer_draft_{}",
        hex::encode(Sha256::digest(serde_json::to_vec(&scope)?))
    ))
}

fn ensure_offer_scope(offer: &ComputeOffer, provider_id: &str, pool_id: &str) -> Result<()> {
    if offer.provider_id != provider_id || offer.capacity_pool.pool_id != pool_id {
        bail!("算力 Offer 不属于指定 Provider/CapacityPool");
    }
    Ok(())
}

fn offer_view(mut receipt: ComputeOfferRegistrationReceipt, replayed: bool) -> MyComputeOfferView {
    receipt.replayed |= replayed;
    MyComputeOfferView {
        offer: receipt.offer,
        provider_policy_revision: receipt.provider_policy_revision,
        provider_digest: receipt.provider_digest,
        replayed: receipt.replayed,
        market_effect: "none",
    }
}

fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max_len
        || value.chars().any(char::is_control)
    {
        bail!("{label}为空、过长或包含无效字符");
    }
    Ok(())
}
