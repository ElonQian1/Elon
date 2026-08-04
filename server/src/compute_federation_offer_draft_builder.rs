use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};

use crate::{
    compute_federation::{
        capacity::{ComputeCapacityBucketStatus, ComputeCapacityPool},
        market::{
            ComputeDeliveryWindow, ComputePriceTerms, ComputeSku, COMPUTE_PRICE_TERMS_SCHEMA,
            COMPUTE_SKU_SCHEMA,
        },
        offer::{
            ComputeOffer, ComputeOfferAuthorization, ComputeOfferCapacity,
            ComputeOfferResourceProfile, COMPUTE_OFFER_SCHEMA, OFFER_STATUS_DRAFT,
        },
        provider::ComputeProvider,
    },
    compute_federation_offer_draft_model::{
        ComputeOfferDraftCapacityInput, CreateMyComputeOfferDraftRequest,
    },
    store::{compute_offer_digest, compute_sku_digest, ComputeCapacityBucketRead},
};

pub(crate) struct ResolvedDraftCapacity<'a> {
    pub input: &'a ComputeOfferDraftCapacityInput,
    pub bucket: ComputeCapacityBucketRead,
}

pub(crate) fn build_offer_draft(
    offer_id: String,
    created_at: String,
    provider: &ComputeProvider,
    pool: &ComputeCapacityPool,
    request: &CreateMyComputeOfferDraftRequest,
    resolved_capacity: Vec<ResolvedDraftCapacity<'_>>,
) -> Result<ComputeOffer> {
    validate_request_bounds(request)?;
    let valid_from = canonical_utc("Offer 生效时间", &request.valid_from)?;
    let valid_until = canonical_utc("Offer 失效时间", &request.valid_until)?;
    let created_at = canonical_utc("Offer 创建时间", &created_at)?;
    let mut windows = BTreeMap::new();
    let mut metering_units = BTreeSet::new();
    let mut capacity = Vec::with_capacity(resolved_capacity.len());
    for resolved in resolved_capacity {
        if resolved.bucket.balance.status != ComputeCapacityBucketStatus::Open
            || resolved.bucket.balance.binding.pool != pool.binding
        {
            bail!("draft Offer 只能引用当前 active Pool 的 open Bucket");
        }
        metering_units.insert(resolved.bucket.balance.binding.meter.clone());
        let window = ComputeDeliveryWindow {
            binding: resolved.bucket.balance.binding.delivery_window.clone(),
            starts_at_utc: resolved.bucket.starts_at_utc,
            ends_at_utc: resolved.bucket.ends_at_utc,
        };
        if windows
            .insert(window.binding.window_id.clone(), window.clone())
            .is_some_and(|existing| existing != window)
        {
            bail!("同一交付窗口 ID 不能绑定不同时间或摘要");
        }
        capacity.push(ComputeOfferCapacity {
            bucket: resolved.bucket.balance.binding,
            total_units: resolved.input.total_units,
            reservable_units: resolved.input.reservable_units,
        });
    }
    capacity.sort_by(|left, right| left.bucket.bucket_id.cmp(&right.bucket.bucket_id));

    let model_family = request
        .model
        .as_ref()
        .map(|model| model.model_family.clone());
    let model_digest = request
        .model
        .as_ref()
        .map(|model| model.model_digest.clone());
    let tokenizer_digest = request
        .model
        .as_ref()
        .and_then(|model| model.tokenizer_digest.clone());
    let mut sku = ComputeSku {
        schema: COMPUTE_SKU_SCHEMA.to_string(),
        sku_id: request.sku.sku_id.clone(),
        task_kind: request.sku.task_kind.clone(),
        model_family,
        model_digest,
        tokenizer_digest,
        runtime_family: request.runtime.runtime_family.clone(),
        precision: request.runtime.precision.clone(),
        context_or_shape_bucket: request.sku.context_or_shape_bucket.clone(),
        verification_tier: request.sku.verification_tier.clone(),
        sla_tier: request.sku.sla_tier.clone(),
        region_or_data_zone: pool.region_or_data_zone.clone(),
        delivery_window_class: request.sku.delivery_window_class.clone(),
        metering_units: metering_units.into_iter().collect(),
        sku_digest: String::new(),
    };
    sku.sku_digest = compute_sku_digest(&sku)?;

    let mut offer = ComputeOffer {
        schema: COMPUTE_OFFER_SCHEMA.to_string(),
        offer_id,
        offer_version: 1,
        offer_digest: String::new(),
        provider_id: provider.provider_id.clone(),
        provider_kind: provider.provider_kind.clone(),
        status: OFFER_STATUS_DRAFT.to_string(),
        sku,
        model: request.model.clone(),
        runtime: request.runtime.clone(),
        resource_profile: ComputeOfferResourceProfile {
            declared_profile_digest: pool.resource_profile_digest.clone(),
            observed_profile_digest: provider.evidence_profile.observed_hardware_digest.clone(),
            verified_profile_digest: provider.evidence_profile.verified_hardware_digest.clone(),
            accelerator_kind: request.resource_profile.accelerator_kind.clone(),
            accelerator_count: request.resource_profile.accelerator_count,
            vram_bytes: request.resource_profile.vram_bytes,
            ram_bytes: request.resource_profile.ram_bytes,
        },
        capacity_pool: pool.binding.clone(),
        capacity,
        execution_limits: request.execution_limits.clone(),
        authorization: ComputeOfferAuthorization {
            public: request.authorization.public,
            allowed_account_ids: request.authorization.allowed_account_ids.clone(),
            allowed_project_ids: request.authorization.allowed_project_ids.clone(),
            allowed_data_classes: request.authorization.allowed_data_classes.clone(),
            policy_revision: 1,
        },
        delivery_windows: windows.into_values().collect(),
        price_terms: ComputePriceTerms {
            schema: COMPUTE_PRICE_TERMS_SCHEMA.to_string(),
            pricing_mode: request.price_terms.pricing_mode.clone(),
            currency: request.price_terms.currency.clone(),
            curve_id: request.price_terms.curve_id.clone(),
            curve_version: request.price_terms.curve_version,
            instrument_id: request.price_terms.instrument_id.clone(),
            components: request.price_terms.components.clone(),
            fee_rules: request.price_terms.fee_rules.clone(),
            valid_until: valid_until.clone(),
        },
        valid_from,
        valid_until,
        created_at,
    };
    offer.offer_digest = compute_offer_digest(&offer)?;
    Ok(offer)
}

fn validate_request_bounds(request: &CreateMyComputeOfferDraftRequest) -> Result<()> {
    if serde_json::to_vec(request)?.len() > 256 * 1024 {
        bail!("draft Offer 请求不能超过 256 KiB");
    }
    if request.capacity.is_empty() || request.capacity.len() > 256 {
        bail!("draft Offer 容量行数量必须在 1 到 256 之间");
    }
    if request.price_terms.components.is_empty() || request.price_terms.components.len() > 64 {
        bail!("draft Offer 价格组件数量必须在 1 到 64 之间");
    }
    if request.price_terms.fee_rules.len() > 64
        || request.authorization.allowed_account_ids.len() > 256
        || request.authorization.allowed_project_ids.len() > 256
        || request.authorization.allowed_data_classes.len() > 16
    {
        bail!("draft Offer 授权或费用规则数量超过上限");
    }
    Ok(())
}

fn canonical_utc(label: &str, value: &str) -> Result<String> {
    let parsed = DateTime::parse_from_rfc3339(value.trim())
        .with_context(|| format!("{label}不是 RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("{label}必须使用 UTC 时区");
    }
    Ok(parsed.with_timezone(&Utc).to_rfc3339())
}
