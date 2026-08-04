use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        capacity::ComputeCapacityPoolStatus,
        offer::{ComputeOffer, OFFER_STATUS_DRAFT, OFFER_STATUS_REVOKED},
        provider::PROVIDER_STATUS_ACTIVE,
    },
    compute_federation_capacity_pool_service,
    compute_federation_offer_draft_builder::{
        build_offer_draft, build_revised_offer_draft, ResolvedDraftCapacity,
    },
    compute_federation_offer_draft_model::{
        CreateMyComputeOfferDraftRequest, ReviseMyComputeOfferDraftRequest,
        RevokeMyComputeOfferDraftRequest,
    },
    store::{compute_offer_digest, ComputeOfferRegistrationReceipt, Store},
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

pub(crate) fn get_for_review(store: &Store, offer_id: &str) -> Result<MyComputeOfferView> {
    validate_exact("算力 Offer ID", offer_id, 200)?;
    store
        .compute_offer_if_exists(offer_id)?
        .map(|receipt| offer_view(receipt, false))
        .ok_or_else(|| anyhow::anyhow!("算力 Offer 不存在"))
}

pub(crate) fn list_drafts_for_review(
    store: &Store,
    limit: usize,
) -> Result<Vec<MyComputeOfferView>> {
    Ok(store
        .list_compute_offer_drafts_for_review(limit)?
        .into_iter()
        .map(|receipt| offer_view(receipt, false))
        .collect())
}

pub(crate) fn revoke_draft_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    offer_id: &str,
    request: RevokeMyComputeOfferDraftRequest,
) -> Result<MyComputeOfferView> {
    if !request.confirm_revoke {
        bail!("撤销 draft Offer 前必须显式确认");
    }
    if request.expected_offer_version <= 0 {
        bail!("预期 Offer 版本必须为正整数");
    }
    validate_exact("预期 Offer 摘要", &request.expected_offer_digest, 256)?;
    owned_active_or_historical_provider(store, user_id, provider_id)?;
    validate_exact("算力 Offer ID", offer_id, 200)?;
    let current = store
        .compute_offer_if_exists(offer_id)?
        .ok_or_else(|| anyhow::anyhow!("算力 Offer 不存在"))?;
    ensure_offer_scope(&current.offer, provider_id, pool_id)?;

    if current.offer.status == OFFER_STATUS_REVOKED {
        let replay_version = request
            .expected_offer_version
            .checked_add(1)
            .context("算力 Offer 版本溢出")?;
        if current.offer.offer_version != replay_version {
            bail!("draft Offer 已终结，且不是本请求的幂等结果");
        }
        let previous = store
            .compute_offer_version_if_exists(offer_id, request.expected_offer_version)?
            .ok_or_else(|| anyhow::anyhow!("draft Offer 撤销前历史版本缺失"))?;
        ensure_offer_scope(&previous.offer, provider_id, pool_id)?;
        if previous.offer.status != OFFER_STATUS_DRAFT
            || previous.offer.offer_digest != request.expected_offer_digest
        {
            bail!("draft Offer 撤销重放与历史合同不一致");
        }
        return Ok(offer_view(current, true));
    }

    if current.offer.status != OFFER_STATUS_DRAFT {
        bail!("本入口只能撤销 draft Offer");
    }
    if current.offer.offer_version != request.expected_offer_version
        || current.offer.offer_digest != request.expected_offer_digest
    {
        bail!("draft Offer 当前版本或摘要已变化");
    }

    let mut revoked = current.offer;
    revoked.offer_version = revoked
        .offer_version
        .checked_add(1)
        .context("算力 Offer 版本溢出")?;
    revoked.status = OFFER_STATUS_REVOKED.to_string();
    revoked.offer_digest.clear();
    revoked.offer_digest = compute_offer_digest(&revoked)?;
    Ok(offer_view(store.register_compute_offer(&revoked)?, false))
}

pub(crate) fn revise_draft_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    offer_id: &str,
    request: ReviseMyComputeOfferDraftRequest,
) -> Result<MyComputeOfferView> {
    if !request.confirm_revise {
        bail!("修订 draft Offer 前必须显式确认");
    }
    if request.expected_offer_version <= 0 {
        bail!("预期 Offer 版本必须为正整数");
    }
    validate_exact("预期 Offer 摘要", &request.expected_offer_digest, 256)?;
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
        bail!("只有 active CapacityPool 可以修订 draft Offer");
    }
    validate_exact("算力 Offer ID", offer_id, 200)?;
    let current = store
        .compute_offer_if_exists(offer_id)?
        .ok_or_else(|| anyhow::anyhow!("算力 Offer 不存在"))?;
    ensure_offer_scope(&current.offer, provider_id, pool_id)?;
    if current.offer.status != OFFER_STATUS_DRAFT {
        bail!("本入口只能修订 draft Offer");
    }

    let previous = store
        .compute_offer_version_if_exists(offer_id, request.expected_offer_version)?
        .ok_or_else(|| anyhow::anyhow!("draft Offer 修订前历史版本缺失"))?;
    if previous.offer.status != OFFER_STATUS_DRAFT
        || previous.offer.offer_digest != request.expected_offer_digest
    {
        bail!("draft Offer 预期历史版本与请求不一致");
    }

    let next_version = request
        .expected_offer_version
        .checked_add(1)
        .context("算力 Offer 版本溢出")?;
    let resolved_capacity = resolve_revision_capacity(store, &request)?;
    let mut candidate = build_revised_offer_draft(
        offer_id.to_string(),
        current.offer.created_at.clone(),
        &provider.provider,
        &pool,
        &request,
        resolved_capacity,
    )?;
    candidate.offer_version = next_version;
    candidate.authorization.policy_revision = previous
        .offer
        .authorization
        .policy_revision
        .checked_add(1)
        .context("Offer 授权策略修订号溢出")?;
    candidate.offer_digest.clear();
    candidate.offer_digest = compute_offer_digest(&candidate)?;

    if current.offer.offer_version == next_version {
        if current.offer != candidate {
            bail!("draft Offer 修订重放与历史合同不一致");
        }
        return Ok(offer_view(current, true));
    }
    if current.offer.offer_version != request.expected_offer_version
        || current.offer.offer_digest != request.expected_offer_digest
    {
        bail!("draft Offer 当前版本或摘要已变化");
    }
    Ok(offer_view(store.register_compute_offer(&candidate)?, false))
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

fn resolve_revision_capacity<'a>(
    store: &Store,
    request: &'a ReviseMyComputeOfferDraftRequest,
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
