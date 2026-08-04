use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        market::{
            ComputePriceSnapshot, ComputePriceSource, COMPUTE_PRICE_SNAPSHOT_SCHEMA,
            PRICE_SOURCE_FALLBACK_CURVE,
        },
        offer::OFFER_STATUS_ACTIVE,
    },
    compute_federation_offer_service,
    compute_federation_price_snapshot_model::PublishMyComputePriceSnapshotRequest,
    store::{compute_price_snapshot_digest, ComputePriceSnapshotRegistrationReceipt, Store},
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MyComputePriceSnapshotView {
    pub snapshot: ComputePriceSnapshot,
    pub replayed: bool,
    pub market_effect: &'static str,
    pub reservation_effect: &'static str,
    pub capacity_effect: &'static str,
    pub funds_effect: &'static str,
}

pub(crate) fn publish_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    offer_id: &str,
    request: PublishMyComputePriceSnapshotRequest,
) -> Result<MyComputePriceSnapshotView> {
    validate_request(&request)?;
    let offer_view = compute_federation_offer_service::get_for_user(
        store,
        user_id,
        provider_id,
        pool_id,
        offer_id,
    )?;
    let offer = offer_view.offer;
    if offer.status != OFFER_STATUS_ACTIVE
        || offer.offer_version != request.expected_offer_version
        || offer.offer_digest != request.expected_offer_digest
    {
        bail!("只有当前版本和摘要精确匹配的 active Offer 可以发布价格快照");
    }
    let window = offer
        .delivery_windows
        .iter()
        .find(|window| window.binding.window_id == request.delivery_window_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("交付窗口不属于当前 Offer"))?;
    let snapshot_id = deterministic_id(
        "price_snapshot",
        user_id,
        provider_id,
        pool_id,
        offer_id,
        &request.idempotency_key,
    )?;
    let quote_id = deterministic_id(
        "quote",
        user_id,
        provider_id,
        pool_id,
        offer_id,
        &request.idempotency_key,
    )?;

    if let Some(existing) = store.compute_price_snapshot_if_exists(&snapshot_id)? {
        validate_replay(&existing.snapshot, &offer, &window, &quote_id, &request)?;
        return Ok(snapshot_view(existing, true));
    }

    let quoted_at = Utc::now();
    let expires_at = capped_expiry(&offer, &quoted_at, request.ttl_seconds)?;
    let source = fallback_source(&offer, &window, &request, &quoted_at)?;
    let mut snapshot = ComputePriceSnapshot {
        schema: COMPUTE_PRICE_SNAPSHOT_SCHEMA.to_string(),
        snapshot_id,
        snapshot_digest: String::new(),
        quote_id,
        pricing_mode: offer.price_terms.pricing_mode.clone(),
        sku: offer.sku.clone(),
        provider_id: offer.provider_id.clone(),
        offer_id: offer.offer_id.clone(),
        offer_version: offer.offer_version,
        offer_digest: offer.offer_digest.clone(),
        delivery_window: window,
        currency: offer.price_terms.currency.clone(),
        components: offer.price_terms.components.clone(),
        fee_rules: offer.price_terms.fee_rules.clone(),
        consumer_max_amount_micros: request.consumer_max_amount_micros,
        provider_max_amount_micros: request.provider_max_amount_micros,
        price_source: source,
        trade_id: None,
        instrument_id: offer.price_terms.instrument_id.clone(),
        rounding_mode: request.rounding_mode,
        quoted_at: quoted_at.to_rfc3339(),
        expires_at: expires_at.to_rfc3339(),
    };
    snapshot.snapshot_digest = compute_price_snapshot_digest(&snapshot)?;
    Ok(snapshot_view(
        store.register_compute_price_snapshot(&snapshot)?,
        false,
    ))
}

pub(crate) fn get_for_user(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    offer_id: &str,
    snapshot_id: &str,
) -> Result<MyComputePriceSnapshotView> {
    compute_federation_offer_service::get_for_user(store, user_id, provider_id, pool_id, offer_id)?;
    validate_exact("价格快照 ID", snapshot_id, 200)?;
    let receipt = store.compute_price_snapshot(snapshot_id)?;
    if receipt.snapshot.provider_id != provider_id || receipt.snapshot.offer_id != offer_id {
        bail!("价格快照不属于指定 Provider/Offer");
    }
    Ok(snapshot_view(receipt, false))
}

fn validate_replay(
    snapshot: &ComputePriceSnapshot,
    offer: &crate::compute_federation::offer::ComputeOffer,
    window: &crate::compute_federation::market::ComputeDeliveryWindow,
    quote_id: &str,
    request: &PublishMyComputePriceSnapshotRequest,
) -> Result<()> {
    let quoted_at = parse_utc("价格快照报价时间", &snapshot.quoted_at)?;
    let expected_expiry = capped_expiry(offer, &quoted_at, request.ttl_seconds)?;
    let expected_source = fallback_source(offer, window, request, &quoted_at)?;
    if snapshot.quote_id != quote_id
        || snapshot.offer_version != request.expected_offer_version
        || snapshot.offer_digest != request.expected_offer_digest
        || snapshot.delivery_window != *window
        || snapshot.consumer_max_amount_micros != request.consumer_max_amount_micros
        || snapshot.provider_max_amount_micros != request.provider_max_amount_micros
        || snapshot.rounding_mode != request.rounding_mode
        || snapshot.expires_at != expected_expiry.to_rfc3339()
        || snapshot.price_source != expected_source
    {
        bail!("价格快照幂等键已绑定不同报价合同");
    }
    Ok(())
}

fn fallback_source(
    offer: &crate::compute_federation::offer::ComputeOffer,
    window: &crate::compute_federation::market::ComputeDeliveryWindow,
    request: &PublishMyComputePriceSnapshotRequest,
    quoted_at: &DateTime<Utc>,
) -> Result<ComputePriceSource> {
    let observation_start = quoted_at
        .clone()
        .checked_sub_signed(Duration::seconds(1))
        .context("价格观察窗口时间溢出")?;
    let source_id = format!("offer_fallback_curve:{}", offer.offer_id);
    let digest_value = serde_json::json!({
        "purpose":"compute_offer_fallback_curve_quote_v1",
        "offer_id":offer.offer_id,
        "offer_version":offer.offer_version,
        "offer_digest":offer.offer_digest,
        "window_id":window.binding.window_id,
        "window_digest":window.binding.window_digest,
        "consumer_max_amount_micros":request.consumer_max_amount_micros,
        "provider_max_amount_micros":request.provider_max_amount_micros,
        "ttl_seconds":request.ttl_seconds,
        "rounding_mode":request.rounding_mode,
        "idempotency_key":request.idempotency_key,
        "quoted_at":quoted_at.to_rfc3339(),
    });
    Ok(ComputePriceSource {
        source_kind: PRICE_SOURCE_FALLBACK_CURVE.to_string(),
        source_id,
        source_version: offer.offer_version,
        observation_window_start: observation_start.to_rfc3339(),
        observation_window_end: quoted_at.to_rfc3339(),
        sample_count: 0,
        source_digest: hex::encode(Sha256::digest(serde_json::to_vec(&digest_value)?)),
    })
}

fn capped_expiry(
    offer: &crate::compute_federation::offer::ComputeOffer,
    quoted_at: &DateTime<Utc>,
    ttl_seconds: i64,
) -> Result<DateTime<Utc>> {
    let requested = quoted_at
        .clone()
        .checked_add_signed(Duration::seconds(ttl_seconds))
        .context("价格快照 TTL 溢出")?;
    let offer_expiry = parse_utc("Offer 失效时间", &offer.valid_until)?;
    let terms_expiry = parse_utc("价格条款失效时间", &offer.price_terms.valid_until)?;
    let expiry = requested.min(offer_expiry).min(terms_expiry);
    let valid_from = parse_utc("Offer 生效时间", &offer.valid_from)?;
    if quoted_at < &valid_from || expiry <= quoted_at.clone() {
        bail!("Offer 尚未生效或剩余有效期不足，不能发布价格快照");
    }
    Ok(expiry)
}

fn deterministic_id(
    prefix: &str,
    user_id: &str,
    provider_id: &str,
    pool_id: &str,
    offer_id: &str,
    idempotency_key: &str,
) -> Result<String> {
    for (label, value, max_len) in [
        ("ID 前缀", prefix, 40),
        ("当前用户 ID", user_id, 160),
        ("Provider ID", provider_id, 160),
        ("CapacityPool ID", pool_id, 160),
        ("Offer ID", offer_id, 200),
        ("价格快照幂等键", idempotency_key, 160),
    ] {
        validate_exact(label, value, max_len)?;
    }
    let value = serde_json::json!({
        "purpose":prefix,
        "user_id":user_id,
        "provider_id":provider_id,
        "pool_id":pool_id,
        "offer_id":offer_id,
        "idempotency_key":idempotency_key,
    });
    Ok(format!(
        "{prefix}_{}",
        hex::encode(Sha256::digest(serde_json::to_vec(&value)?))
    ))
}

fn snapshot_view(
    mut receipt: ComputePriceSnapshotRegistrationReceipt,
    replayed: bool,
) -> MyComputePriceSnapshotView {
    receipt.replayed |= replayed;
    MyComputePriceSnapshotView {
        snapshot: receipt.snapshot,
        replayed: receipt.replayed,
        market_effect: "quote_candidate_enabled",
        reservation_effect: "none",
        capacity_effect: "none",
        funds_effect: "none",
    }
}

fn validate_request(request: &PublishMyComputePriceSnapshotRequest) -> Result<()> {
    if !request.confirm_publish {
        bail!("发布价格快照前必须显式确认");
    }
    if request.expected_offer_version <= 0
        || request.consumer_max_amount_micros < 0
        || request.provider_max_amount_micros < 0
        || request.provider_max_amount_micros > request.consumer_max_amount_micros
        || !(30..=3600).contains(&request.ttl_seconds)
    {
        bail!("价格快照版本、金额上限或 TTL 无效");
    }
    for (label, value, max_len) in [
        (
            "预期 Offer 摘要",
            request.expected_offer_digest.as_str(),
            256,
        ),
        ("交付窗口 ID", request.delivery_window_id.as_str(), 160),
        ("舍入模式", request.rounding_mode.as_str(), 40),
        ("价格快照幂等键", request.idempotency_key.as_str(), 160),
    ] {
        validate_exact(label, value, max_len)?;
    }
    if !matches!(
        request.rounding_mode.as_str(),
        "half_up" | "half_even" | "floor" | "ceil"
    ) {
        bail!("价格快照舍入模式不受支持");
    }
    Ok(())
}

fn parse_utc(label: &str, value: &str) -> Result<DateTime<Utc>> {
    let parsed =
        DateTime::parse_from_rfc3339(value).with_context(|| format!("{label}不是 RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("{label}必须使用 UTC 时区");
    }
    Ok(parsed.with_timezone(&Utc))
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
