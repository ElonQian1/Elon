use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, FixedOffset};
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    market::{
        ComputeDeliveryWindow, ComputeFeeRule, ComputePriceComponent, ComputePriceTerms,
        ComputeSku, COMPUTE_PRICE_TERMS_SCHEMA, COMPUTE_SKU_SCHEMA, PRICING_MODE_CAPACITY_FORWARD,
        PRICING_MODE_CAPACITY_FUTURE, PRICING_MODE_INDEX_LOCKED, PRICING_MODE_SPOT,
    },
    offer::{
        ComputeOffer, ComputeOfferAuthorization, ComputeOfferCapacity, ComputeOfferExecutionLimits,
        ComputeOfferResourceProfile, COMPUTE_OFFER_SCHEMA, OFFER_STATUS_ACTIVE, OFFER_STATUS_DRAFT,
        OFFER_STATUS_DRAINING, OFFER_STATUS_EXPIRED, OFFER_STATUS_REVOKED,
    },
    provider::{ComputeProvider, PROVIDER_STATUS_ACTIVE},
    workload::{ComputeModelRef, ComputeRuntimeRef},
};

pub(super) fn validate_offer_contract(
    offer: &ComputeOffer,
    provider: &ComputeProvider,
) -> Result<String> {
    validate_offer_identity(offer, provider)?;
    validate_sku(&offer.sku)?;
    validate_model(&offer.sku, offer.model.as_ref())?;
    validate_runtime(&offer.sku, &offer.runtime)?;
    validate_resource_profile(&offer.resource_profile)?;
    validate_provider_compatibility(offer, provider)?;
    validate_execution_limits(&offer.execution_limits)?;
    validate_authorization(&offer.authorization, provider)?;
    let windows = validate_delivery_windows(&offer.delivery_windows)?;
    validate_capacity(offer, &windows)?;
    validate_price_terms(&offer.price_terms, &offer.sku, &offer.capacity)?;
    validate_offer_times(offer, &windows)?;

    let computed_digest = compute_offer_digest(offer)?;
    if offer.offer_digest != computed_digest {
        bail!("算力 Offer 摘要与规范合同内容不一致");
    }
    Ok(computed_digest)
}

pub(super) fn compute_offer_digest(offer: &ComputeOffer) -> Result<String> {
    let mut canonical = offer.clone();
    canonical.offer_digest.clear();
    let encoded = serde_json::to_vec(&canonical)?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

fn validate_offer_identity(offer: &ComputeOffer, provider: &ComputeProvider) -> Result<()> {
    if offer.schema != COMPUTE_OFFER_SCHEMA {
        bail!("算力 Offer schema 不受支持");
    }
    for (label, value) in [
        ("Offer ID", offer.offer_id.as_str()),
        ("Offer 摘要", offer.offer_digest.as_str()),
        ("Provider ID", offer.provider_id.as_str()),
        ("Provider 类型", offer.provider_kind.as_str()),
    ] {
        validate_exact_value(label, value)?;
    }
    if offer.offer_version <= 0 {
        bail!("算力 Offer 版本必须为正整数");
    }
    if !matches!(
        offer.status.as_str(),
        OFFER_STATUS_DRAFT
            | OFFER_STATUS_ACTIVE
            | OFFER_STATUS_DRAINING
            | OFFER_STATUS_EXPIRED
            | OFFER_STATUS_REVOKED
    ) {
        bail!("算力 Offer 状态不受支持");
    }
    if offer.provider_id != provider.provider_id || offer.provider_kind != provider.provider_kind {
        bail!("算力 Offer 与 Provider 稳定身份不一致");
    }
    if offer.status == OFFER_STATUS_ACTIVE && provider.status != PROVIDER_STATUS_ACTIVE {
        bail!("active Offer 只能由 active Provider 发布");
    }
    Ok(())
}

fn validate_sku(sku: &ComputeSku) -> Result<()> {
    if sku.schema != COMPUTE_SKU_SCHEMA {
        bail!("算力 SKU schema 不受支持");
    }
    for (label, value) in [
        ("SKU ID", sku.sku_id.as_str()),
        ("SKU 任务类型", sku.task_kind.as_str()),
        ("SKU 运行时家族", sku.runtime_family.as_str()),
        ("SKU 精度", sku.precision.as_str()),
        ("SKU 上下文或形状档位", sku.context_or_shape_bucket.as_str()),
        ("SKU 验证等级", sku.verification_tier.as_str()),
        ("SKU SLA 等级", sku.sla_tier.as_str()),
        ("SKU 区域", sku.region_or_data_zone.as_str()),
        ("SKU 交付窗口等级", sku.delivery_window_class.as_str()),
        ("SKU 摘要", sku.sku_digest.as_str()),
    ] {
        validate_exact_value(label, value)?;
    }
    validate_optional_value("SKU 模型家族", sku.model_family.as_deref())?;
    validate_optional_value("SKU 模型摘要", sku.model_digest.as_deref())?;
    validate_optional_value("SKU Tokenizer 摘要", sku.tokenizer_digest.as_deref())?;
    validate_unique_values("SKU 计量单位", &sku.metering_units, true)?;
    Ok(())
}

fn validate_model(sku: &ComputeSku, model: Option<&ComputeModelRef>) -> Result<()> {
    let sku_has_model =
        sku.model_family.is_some() || sku.model_digest.is_some() || sku.tokenizer_digest.is_some();
    match model {
        Some(model) => {
            for (label, value) in [
                ("模型 ID", model.model_id.as_str()),
                ("模型家族", model.model_family.as_str()),
                ("模型摘要", model.model_digest.as_str()),
            ] {
                validate_exact_value(label, value)?;
            }
            validate_optional_value("模型 Tokenizer 摘要", model.tokenizer_digest.as_deref())?;
            validate_unique_values("模型 Adapter 摘要", &model.adapter_digests, false)?;
            if sku.model_family.as_deref() != Some(model.model_family.as_str())
                || sku.model_digest.as_deref() != Some(model.model_digest.as_str())
                || sku.tokenizer_digest != model.tokenizer_digest
            {
                bail!("算力 Offer 模型与 SKU 模型约束不一致");
            }
        }
        None if sku_has_model => bail!("带模型约束的 SKU 必须绑定模型"),
        None => {}
    }
    Ok(())
}

fn validate_runtime(sku: &ComputeSku, runtime: &ComputeRuntimeRef) -> Result<()> {
    for (label, value) in [
        ("运行时家族", runtime.runtime_family.as_str()),
        ("运行时版本", runtime.runtime_version.as_str()),
        ("运行时精度", runtime.precision.as_str()),
        ("Runner 摘要", runtime.runner_digest.as_str()),
    ] {
        validate_exact_value(label, value)?;
    }
    if runtime.runtime_family != sku.runtime_family || runtime.precision != sku.precision {
        bail!("算力 Offer 运行时与 SKU 不一致");
    }
    let plugin_parts = [
        runtime.plugin_id.as_deref(),
        runtime.plugin_version.as_deref(),
        runtime.plugin_digest.as_deref(),
    ];
    if plugin_parts.iter().any(|value| value.is_some())
        && plugin_parts.iter().any(|value| value.is_none())
    {
        bail!("算力 Offer 插件 ID、版本和摘要必须同时提供");
    }
    for (label, value) in [
        ("运行时插件 ID", runtime.plugin_id.as_deref()),
        ("运行时插件版本", runtime.plugin_version.as_deref()),
        ("运行时插件摘要", runtime.plugin_digest.as_deref()),
    ] {
        validate_optional_value(label, value)?;
    }
    Ok(())
}

fn validate_resource_profile(profile: &ComputeOfferResourceProfile) -> Result<()> {
    validate_exact_value("声明资源档案摘要", &profile.declared_profile_digest)?;
    validate_optional_value(
        "观测资源档案摘要",
        profile.observed_profile_digest.as_deref(),
    )?;
    validate_optional_value(
        "验证资源档案摘要",
        profile.verified_profile_digest.as_deref(),
    )?;
    validate_exact_value("加速器类型", &profile.accelerator_kind)?;
    if profile.accelerator_count <= 0 || profile.vram_bytes <= 0 || profile.ram_bytes <= 0 {
        bail!("算力 Offer 的加速器数量、显存和内存必须为正整数");
    }
    Ok(())
}

fn validate_provider_compatibility(offer: &ComputeOffer, provider: &ComputeProvider) -> Result<()> {
    if !provider
        .capabilities
        .task_kinds
        .iter()
        .any(|value| value == &offer.sku.task_kind)
    {
        bail!("Provider 不支持 Offer 的任务类型");
    }
    if !provider
        .capabilities
        .accelerator_kinds
        .iter()
        .any(|value| value == &offer.resource_profile.accelerator_kind)
    {
        bail!("Provider 不支持 Offer 的加速器类型");
    }
    if !provider
        .capabilities
        .regions
        .iter()
        .any(|value| value == &offer.sku.region_or_data_zone)
    {
        bail!("Provider 不支持 Offer 的区域");
    }
    Ok(())
}

fn validate_execution_limits(limits: &ComputeOfferExecutionLimits) -> Result<()> {
    if limits.max_concurrent_attempts <= 0 || limits.max_attempt_runtime_seconds <= 0 {
        bail!("算力 Offer 并发数和最长 Attempt 时间必须为正整数");
    }
    Ok(())
}

fn validate_authorization(
    authorization: &ComputeOfferAuthorization,
    provider: &ComputeProvider,
) -> Result<()> {
    if authorization.policy_revision <= 0 {
        bail!("算力 Offer 授权策略版本必须为正整数");
    }
    validate_unique_values("允许账户", &authorization.allowed_account_ids, false)?;
    validate_unique_values("允许项目", &authorization.allowed_project_ids, false)?;
    validate_unique_values("允许数据等级", &authorization.allowed_data_classes, false)?;
    if authorization.public
        && (!authorization.allowed_account_ids.is_empty()
            || !authorization.allowed_project_ids.is_empty())
    {
        bail!("公开 Offer 不能同时设置账户或项目白名单");
    }
    if !authorization.public
        && authorization.allowed_account_ids.is_empty()
        && authorization.allowed_project_ids.is_empty()
    {
        bail!("非公开 Offer 至少需要一个允许账户或项目");
    }
    for data_class in &authorization.allowed_data_classes {
        if !provider
            .capabilities
            .allowed_data_classes
            .iter()
            .any(|value| value == data_class)
        {
            bail!("Offer 授权的数据等级超出 Provider 能力范围");
        }
    }
    Ok(())
}

fn validate_delivery_windows(
    windows: &[ComputeDeliveryWindow],
) -> Result<BTreeMap<String, (String, DateTime<FixedOffset>, DateTime<FixedOffset>)>> {
    if windows.is_empty() {
        bail!("算力 Offer 至少需要一个交付窗口");
    }
    let mut result = BTreeMap::new();
    for window in windows {
        validate_exact_value("交付窗口 ID", &window.binding.window_id)?;
        validate_exact_value("交付窗口摘要", &window.binding.window_digest)?;
        let starts = parse_utc("交付窗口开始时间", &window.starts_at_utc)?;
        let ends = parse_utc("交付窗口结束时间", &window.ends_at_utc)?;
        if starts >= ends {
            bail!("交付窗口结束时间必须晚于开始时间");
        }
        if result
            .insert(
                window.binding.window_id.clone(),
                (window.binding.window_digest.clone(), starts, ends),
            )
            .is_some()
        {
            bail!("算力 Offer 交付窗口 ID 不能重复");
        }
    }
    Ok(result)
}

fn validate_capacity(
    offer: &ComputeOffer,
    windows: &BTreeMap<String, (String, DateTime<FixedOffset>, DateTime<FixedOffset>)>,
) -> Result<()> {
    if offer.capacity.is_empty() {
        bail!("算力 Offer 至少需要一条容量合同");
    }
    for (label, value) in [
        ("容量池 ID", offer.capacity_pool.pool_id.as_str()),
        ("容量池摘要", offer.capacity_pool.pool_digest.as_str()),
    ] {
        validate_exact_value(label, value)?;
    }
    if offer.capacity_pool.capacity_epoch <= 0 || offer.capacity_pool.pool_revision <= 0 {
        bail!("算力 Offer 容量池 epoch 和版本必须为正整数");
    }

    let meters = offer
        .sku
        .metering_units
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut bucket_ids = BTreeSet::new();
    let mut window_meters = BTreeSet::new();
    let mut meter_policies = BTreeMap::new();
    for capacity in &offer.capacity {
        validate_capacity_line(offer, capacity, windows, &meters)?;
        if !bucket_ids.insert(capacity.bucket.bucket_id.as_str()) {
            bail!("算力 Offer 容量 bucket 不能重复");
        }
        if !window_meters.insert((
            capacity.bucket.delivery_window.window_id.as_str(),
            capacity.bucket.meter.as_str(),
        )) {
            bail!("算力 Offer 同一交付窗口的 meter 容量不能重复");
        }
        let policy = (
            capacity.bucket.meter_mode,
            capacity.bucket.quantum_units,
            capacity.bucket.meter_policy_digest.as_str(),
        );
        if meter_policies
            .insert(capacity.bucket.meter.as_str(), policy)
            .is_some_and(|existing| existing != policy)
        {
            bail!("算力 Offer 同一 meter 的计量模式、粒度和策略摘要必须一致");
        }
    }
    let expected = windows
        .len()
        .checked_mul(meters.len())
        .context("算力 Offer 容量矩阵大小溢出")?;
    if window_meters.len() != expected {
        bail!("算力 Offer 必须覆盖每个交付窗口与 meter 的完整容量矩阵");
    }
    Ok(())
}

fn validate_capacity_line(
    offer: &ComputeOffer,
    capacity: &ComputeOfferCapacity,
    windows: &BTreeMap<String, (String, DateTime<FixedOffset>, DateTime<FixedOffset>)>,
    meters: &BTreeSet<&str>,
) -> Result<()> {
    if capacity.bucket.pool != offer.capacity_pool {
        bail!("算力 Offer 容量行必须绑定同一个容量池版本");
    }
    for (label, value) in [
        ("容量 bucket ID", capacity.bucket.bucket_id.as_str()),
        ("容量 bucket 摘要", capacity.bucket.bucket_digest.as_str()),
        ("容量 meter", capacity.bucket.meter.as_str()),
        (
            "容量计量策略摘要",
            capacity.bucket.meter_policy_digest.as_str(),
        ),
    ] {
        validate_exact_value(label, value)?;
    }
    if capacity.bucket.quantum_units <= 0
        || capacity.total_units <= 0
        || capacity.reservable_units < 0
        || capacity.reservable_units > capacity.total_units
        || capacity.total_units % capacity.bucket.quantum_units != 0
        || capacity.reservable_units % capacity.bucket.quantum_units != 0
    {
        bail!("算力 Offer 容量数量或计量粒度无效");
    }
    if offer.status == OFFER_STATUS_ACTIVE && capacity.reservable_units == 0 {
        bail!("active Offer 的每条容量必须允许至少一个单位被预留");
    }
    if !meters.contains(capacity.bucket.meter.as_str()) {
        bail!("算力 Offer 容量 meter 不属于 SKU");
    }
    let (window_digest, _, _) = windows
        .get(&capacity.bucket.delivery_window.window_id)
        .ok_or_else(|| anyhow::anyhow!("算力 Offer 容量引用了未声明的交付窗口"))?;
    if window_digest != &capacity.bucket.delivery_window.window_digest {
        bail!("算力 Offer 容量交付窗口摘要不一致");
    }
    Ok(())
}

fn validate_price_terms(
    terms: &ComputePriceTerms,
    sku: &ComputeSku,
    capacity: &[ComputeOfferCapacity],
) -> Result<()> {
    if terms.schema != COMPUTE_PRICE_TERMS_SCHEMA {
        bail!("算力价格条款 schema 不受支持");
    }
    validate_exact_value("价格币种", &terms.currency)?;
    validate_optional_value("价格曲线 ID", terms.curve_id.as_deref())?;
    validate_optional_value("价格工具 ID", terms.instrument_id.as_deref())?;
    match terms.pricing_mode.as_str() {
        PRICING_MODE_SPOT => {}
        PRICING_MODE_INDEX_LOCKED => {
            if terms.curve_id.is_none() || terms.curve_version.is_none() {
                bail!("index_locked 价格必须绑定曲线 ID 与版本");
            }
        }
        PRICING_MODE_CAPACITY_FORWARD | PRICING_MODE_CAPACITY_FUTURE => {
            if terms.instrument_id.is_none() {
                bail!("远期或期货容量价格必须绑定工具 ID");
            }
        }
        _ => bail!("算力 Offer 定价模式不受支持"),
    }
    if terms.curve_version.is_some_and(|version| version <= 0) {
        bail!("价格曲线版本必须为正整数");
    }
    let expected_meters = sku
        .metering_units
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let meter_quantums = capacity
        .iter()
        .map(|line| (line.bucket.meter.as_str(), line.bucket.quantum_units))
        .collect::<BTreeMap<_, _>>();
    let mut component_meters = BTreeSet::new();
    for component in &terms.components {
        validate_price_component(component)?;
        if meter_quantums.get(component.meter.as_str()) != Some(&component.unit_size) {
            bail!("算力 Offer 价格单位必须等于容量 meter 的计量粒度");
        }
        if !component_meters.insert(component.meter.as_str()) {
            bail!("算力 Offer 价格 meter 不能重复");
        }
    }
    if component_meters != expected_meters {
        bail!("算力 Offer 价格组件必须与 SKU meter 完全一致");
    }
    for rule in &terms.fee_rules {
        validate_fee_rule(rule)?;
    }
    parse_utc("价格条款有效期", &terms.valid_until)?;
    Ok(())
}

fn validate_price_component(component: &ComputePriceComponent) -> Result<()> {
    validate_exact_value("价格 meter", &component.meter)?;
    if component.unit_size <= 0
        || component.max_units <= 0
        || component.consumer_unit_price_micros < 0
        || component.provider_unit_price_micros < 0
    {
        bail!("算力 Offer 价格组件数量和整数微单位价格无效");
    }
    if component.provider_unit_price_micros > component.consumer_unit_price_micros {
        bail!("Provider 单价不能高于消费者单价");
    }
    Ok(())
}

fn validate_fee_rule(rule: &ComputeFeeRule) -> Result<()> {
    validate_exact_value("费用类型", &rule.fee_kind)?;
    validate_exact_value("费用承担方", &rule.charged_to)?;
    if rule.fixed_amount_micros < 0 || !(0..=10_000).contains(&rule.rate_basis_points) {
        bail!("算力 Offer 固定费用或费率无效");
    }
    if rule
        .maximum_amount_micros
        .is_some_and(|maximum| maximum < 0 || maximum < rule.fixed_amount_micros)
    {
        bail!("算力 Offer 费用上限不能为负或低于固定费用");
    }
    Ok(())
}

fn validate_offer_times(
    offer: &ComputeOffer,
    windows: &BTreeMap<String, (String, DateTime<FixedOffset>, DateTime<FixedOffset>)>,
) -> Result<()> {
    let created = parse_utc("Offer 创建时间", &offer.created_at)?;
    let valid_from = parse_utc("Offer 生效时间", &offer.valid_from)?;
    let valid_until = parse_utc("Offer 失效时间", &offer.valid_until)?;
    let price_valid_until = parse_utc("价格条款有效期", &offer.price_terms.valid_until)?;
    if created > valid_from || valid_from >= valid_until {
        bail!("算力 Offer 必须满足 created_at <= valid_from < valid_until");
    }
    if price_valid_until < valid_until {
        bail!("算力 Offer 价格条款必须覆盖完整有效期");
    }
    for (_, starts, ends) in windows.values() {
        if starts < &valid_from || ends > &valid_until {
            bail!("算力 Offer 交付窗口必须完全位于 Offer 有效期内");
        }
    }
    Ok(())
}

fn validate_unique_values(label: &str, values: &[String], required: bool) -> Result<()> {
    if required && values.is_empty() {
        bail!("{label}不能为空");
    }
    let mut unique = BTreeSet::new();
    for value in values {
        validate_exact_value(label, value)?;
        if !unique.insert(value.as_str()) {
            bail!("{label}不能重复");
        }
    }
    Ok(())
}

fn validate_exact_value(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label}不能为空");
    }
    if value != value.trim() {
        bail!("{label}不能包含首尾空白");
    }
    Ok(())
}

fn validate_optional_value(label: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = value {
        validate_exact_value(label, value)?;
    }
    Ok(())
}

fn parse_utc(label: &str, value: &str) -> Result<DateTime<FixedOffset>> {
    let parsed =
        DateTime::parse_from_rfc3339(value).with_context(|| format!("{label}不是 RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("{label}必须使用 UTC 时区");
    }
    Ok(parsed)
}
