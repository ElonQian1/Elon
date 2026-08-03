use anyhow::{bail, Context, Result};
use chrono::{DateTime, FixedOffset};
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    market::{
        ComputePriceComponent, ComputePriceSnapshot, ComputePriceSource,
        COMPUTE_PRICE_SNAPSHOT_SCHEMA, PRICE_SOURCE_FALLBACK_CURVE, PRICE_SOURCE_INDEX,
        PRICE_SOURCE_MARK, PRICE_SOURCE_TRADE,
    },
    offer::{ComputeOffer, OFFER_STATUS_ACTIVE},
};

pub(super) fn validate_price_snapshot_contract(
    snapshot: &ComputePriceSnapshot,
    offer: &ComputeOffer,
) -> Result<String> {
    validate_identity(snapshot, offer)?;
    validate_locked_terms(snapshot, offer)?;
    validate_price_source(&snapshot.price_source, &snapshot.quoted_at)?;
    validate_amount_limits(snapshot)?;
    validate_times(snapshot, offer)?;

    let computed_digest = compute_price_snapshot_digest(snapshot)?;
    if snapshot.snapshot_digest != computed_digest {
        bail!("算力价格快照摘要与规范合同内容不一致");
    }
    Ok(computed_digest)
}

pub(super) fn compute_price_snapshot_digest(snapshot: &ComputePriceSnapshot) -> Result<String> {
    let mut canonical = snapshot.clone();
    canonical.snapshot_digest.clear();
    let encoded = serde_json::to_vec(&canonical)?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

fn validate_identity(snapshot: &ComputePriceSnapshot, offer: &ComputeOffer) -> Result<()> {
    if snapshot.schema != COMPUTE_PRICE_SNAPSHOT_SCHEMA {
        bail!("算力价格快照 schema 不受支持");
    }
    for (label, value) in [
        ("价格快照 ID", snapshot.snapshot_id.as_str()),
        ("价格快照摘要", snapshot.snapshot_digest.as_str()),
        ("报价 ID", snapshot.quote_id.as_str()),
        ("Provider ID", snapshot.provider_id.as_str()),
        ("Offer ID", snapshot.offer_id.as_str()),
        ("Offer 摘要", snapshot.offer_digest.as_str()),
        ("价格币种", snapshot.currency.as_str()),
        ("舍入模式", snapshot.rounding_mode.as_str()),
    ] {
        validate_exact_value(label, value)?;
    }
    if offer.status != OFFER_STATUS_ACTIVE {
        bail!("价格快照只能绑定 active Offer");
    }
    if snapshot.provider_id != offer.provider_id
        || snapshot.offer_id != offer.offer_id
        || snapshot.offer_version != offer.offer_version
        || snapshot.offer_digest != offer.offer_digest
        || snapshot.sku != offer.sku
    {
        bail!("价格快照与 Offer、Provider 或 SKU 绑定不一致");
    }
    if !offer
        .delivery_windows
        .iter()
        .any(|window| window == &snapshot.delivery_window)
    {
        bail!("价格快照交付窗口不属于 Offer");
    }
    if !matches!(
        snapshot.rounding_mode.as_str(),
        "half_up" | "half_even" | "floor" | "ceil"
    ) {
        bail!("价格快照舍入模式不受支持");
    }
    Ok(())
}

fn validate_locked_terms(snapshot: &ComputePriceSnapshot, offer: &ComputeOffer) -> Result<()> {
    let terms = &offer.price_terms;
    if snapshot.pricing_mode != terms.pricing_mode
        || snapshot.currency != terms.currency
        || snapshot.components != terms.components
        || snapshot.fee_rules != terms.fee_rules
        || snapshot.instrument_id != terms.instrument_id
    {
        bail!("V1 价格快照必须精确冻结 Offer 的价格组件、费用和工具引用");
    }
    validate_optional_value("成交 ID", snapshot.trade_id.as_deref())?;
    validate_optional_value("容量工具 ID", snapshot.instrument_id.as_deref())?;
    if snapshot.price_source.source_kind == PRICE_SOURCE_TRADE && snapshot.trade_id.is_none() {
        bail!("trade 来源的价格快照必须绑定成交 ID");
    }
    if snapshot.price_source.source_kind != PRICE_SOURCE_TRADE && snapshot.trade_id.is_some() {
        bail!("非 trade 来源的价格快照不能绑定成交 ID");
    }
    Ok(())
}

fn validate_price_source(source: &ComputePriceSource, quoted_at: &str) -> Result<()> {
    if !matches!(
        source.source_kind.as_str(),
        PRICE_SOURCE_TRADE | PRICE_SOURCE_INDEX | PRICE_SOURCE_MARK | PRICE_SOURCE_FALLBACK_CURVE
    ) {
        bail!("算力价格来源类型不受支持");
    }
    for (label, value) in [
        ("价格来源 ID", source.source_id.as_str()),
        ("价格来源摘要", source.source_digest.as_str()),
    ] {
        validate_exact_value(label, value)?;
    }
    if source.source_version <= 0 || source.sample_count < 0 {
        bail!("价格来源版本或样本数无效");
    }
    if source.source_kind == PRICE_SOURCE_TRADE && source.sample_count != 1 {
        bail!("trade 价格来源必须且只能对应一个成交样本");
    }
    let observation_start = parse_utc("价格观察窗口开始时间", &source.observation_window_start)?;
    let observation_end = parse_utc("价格观察窗口结束时间", &source.observation_window_end)?;
    let quoted_at = parse_utc("价格快照报价时间", quoted_at)?;
    if observation_start >= observation_end || observation_end > quoted_at {
        bail!("价格观察窗口必须在报价前完成且开始时间早于结束时间");
    }
    Ok(())
}

fn validate_amount_limits(snapshot: &ComputePriceSnapshot) -> Result<()> {
    if snapshot.consumer_max_amount_micros < 0
        || snapshot.provider_max_amount_micros < 0
        || snapshot.provider_max_amount_micros > snapshot.consumer_max_amount_micros
    {
        bail!("价格快照消费者或 Provider 最大整数微单位金额无效");
    }
    let consumer_base = maximum_component_amount(&snapshot.components, true)?;
    let provider_base = maximum_component_amount(&snapshot.components, false)?;
    if i128::from(snapshot.consumer_max_amount_micros) < consumer_base
        || i128::from(snapshot.provider_max_amount_micros) < provider_base
    {
        bail!("价格快照最大金额不能低于价格组件的最大基础金额");
    }
    Ok(())
}

fn maximum_component_amount(components: &[ComputePriceComponent], consumer: bool) -> Result<i128> {
    let mut total = 0_i128;
    for component in components {
        if component.unit_size <= 0
            || component.max_units <= 0
            || component.max_units % component.unit_size != 0
        {
            bail!("价格快照组件最大单位必须按计价粒度整除");
        }
        let quanta = i128::from(component.max_units / component.unit_size);
        let unit_price = if consumer {
            component.consumer_unit_price_micros
        } else {
            component.provider_unit_price_micros
        };
        if unit_price < 0 {
            bail!("价格快照组件单价不能为负数");
        }
        let amount = quanta
            .checked_mul(i128::from(unit_price))
            .context("价格快照组件金额溢出")?;
        total = total
            .checked_add(amount)
            .context("价格快照组件总金额溢出")?;
    }
    Ok(total)
}

fn validate_times(snapshot: &ComputePriceSnapshot, offer: &ComputeOffer) -> Result<()> {
    let quoted_at = parse_utc("价格快照报价时间", &snapshot.quoted_at)?;
    let expires_at = parse_utc("价格快照失效时间", &snapshot.expires_at)?;
    let offer_valid_from = parse_utc("Offer 生效时间", &offer.valid_from)?;
    let offer_valid_until = parse_utc("Offer 失效时间", &offer.valid_until)?;
    let terms_valid_until = parse_utc("Offer 价格条款失效时间", &offer.price_terms.valid_until)?;
    if quoted_at < offer_valid_from || quoted_at >= offer_valid_until || quoted_at >= expires_at {
        bail!("价格快照报价时间必须位于 Offer 有效期内并早于快照失效时间");
    }
    if expires_at > offer_valid_until || expires_at > terms_valid_until {
        bail!("价格快照不能晚于 Offer 或价格条款失效时间");
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
