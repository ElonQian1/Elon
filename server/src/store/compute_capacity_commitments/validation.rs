use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, FixedOffset};
use rusqlite::{params, Connection};

use crate::compute_federation::{
    capacity::{
        ComputeCapacityMeterPolicy, ComputeCapacityOfferBinding, ComputeCapacityPoolStatus,
    },
    market::{
        ComputeDeliveryWindow, ComputePriceSnapshot, PRICE_SOURCE_FALLBACK_CURVE,
        PRICING_MODE_CAPACITY_FUTURE,
    },
    offer::{ComputeOffer, OFFER_STATUS_ACTIVE},
    provider::{PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_ACTIVE},
};

use super::{
    super::{
        compute_capacity_claims::HoldComputeCapacityClaimLine,
        compute_capacity_instruments::require_current_capacity_instrument_adoption_on,
        compute_capacity_pool_queries::current_capacity_pool_on,
        compute_offer_registry::current_registered_offer_on,
        compute_platform_reference_price_curve::audited_platform_reference_snapshot_binding_on,
        compute_price_snapshot_registry::registered_price_snapshot_on,
        compute_provider_registry::current_registered_provider_on,
    },
    types::{
        CancelComputeCapacityCommitment, CreateComputeCapacityCommitment,
        ExpireDueComputeCapacityCommitments, COMPUTE_CAPACITY_COMMITMENT_CANCEL_CONFIRMATION,
        COMPUTE_CAPACITY_COMMITMENT_CREATE_CONFIRMATION,
        COMPUTE_CAPACITY_COMMITMENT_EXPIRE_DUE_CONFIRMATION,
    },
};

pub(super) struct ValidatedCreate {
    pub offer: ComputeOffer,
    pub snapshot: ComputePriceSnapshot,
    pub delivery_window: ComputeDeliveryWindow,
    pub claim_lines: Vec<HoldComputeCapacityClaimLine>,
}

pub(super) fn validate_create_input(input: &CreateComputeCapacityCommitment) -> Result<()> {
    for (label, value, max) in [
        ("owner account ID", input.owner_account_id.as_str(), 200),
        ("Provider ID", input.provider_id.as_str(), 160),
        ("Offer ID", input.offer_id.as_str(), 200),
        ("Pool ID", input.pool.pool_id.as_str(), 200),
        (
            "delivery window ID",
            input.delivery_window.window_id.as_str(),
            200,
        ),
        ("Price Snapshot ID", input.price_snapshot_id.as_str(), 200),
        (
            "v223 reference binding ID",
            input.reference_binding_id.as_str(),
            200,
        ),
        ("instrument ID", input.instrument_id.as_str(), 200),
        ("idempotency scope", input.idempotency_scope.as_str(), 240),
        ("idempotency key", input.idempotency_key.as_str(), 200),
    ] {
        validate_exact(label, value, max)?;
    }
    for (label, value) in [
        ("Provider digest", input.provider_digest.as_str()),
        ("Offer digest", input.offer_digest.as_str()),
        ("Pool digest", input.pool.pool_digest.as_str()),
        (
            "delivery window digest",
            input.delivery_window.window_digest.as_str(),
        ),
        (
            "Price Snapshot digest",
            input.price_snapshot_digest.as_str(),
        ),
        (
            "v223 reference binding digest",
            input.reference_binding_digest.as_str(),
        ),
    ] {
        validate_digest(label, value)?;
    }
    if input.provider_policy_revision <= 0
        || input.offer_version <= 0
        || input.pool.capacity_epoch <= 0
        || input.pool.pool_revision <= 0
    {
        bail!("容量承诺 exact revision/epoch 必须为正整数");
    }
    if input.confirmation != COMPUTE_CAPACITY_COMMITMENT_CREATE_CONFIRMATION {
        bail!("容量承诺 Create 缺少固定确认短语");
    }
    validate_quantities(&input.quantities)
}

pub(super) fn validate_cancel_input(input: &CancelComputeCapacityCommitment) -> Result<()> {
    for (label, value, max) in [
        ("owner account ID", input.owner_account_id.as_str(), 200),
        ("Provider ID", input.provider_id.as_str(), 160),
        ("Pool ID", input.pool_id.as_str(), 200),
        ("Commitment ID", input.commitment_id.as_str(), 200),
        ("idempotency scope", input.idempotency_scope.as_str(), 240),
        ("idempotency key", input.idempotency_key.as_str(), 200),
    ] {
        validate_exact(label, value, max)?;
    }
    validate_digest(
        "expected Commitment digest",
        &input.expected_commitment_digest,
    )?;
    if input.expected_commitment_revision != 1 {
        bail!("容量承诺 Cancel 只接受 expected revision 1");
    }
    if !input.reason.is_empty() {
        validate_exact("capacity commitment cancel reason", &input.reason, 1000)?;
    }
    if input.confirmation != COMPUTE_CAPACITY_COMMITMENT_CANCEL_CONFIRMATION {
        bail!("容量承诺 Cancel 缺少固定确认短语");
    }
    Ok(())
}

pub(super) fn validate_expire_input(input: &ExpireDueComputeCapacityCommitments) -> Result<()> {
    validate_exact("platform admin user ID", &input.admin_user_id, 200)?;
    if !(1..=100).contains(&input.limit) {
        bail!("容量承诺到期恢复 limit 必须在 1 到 100 之间");
    }
    if input.confirmation != COMPUTE_CAPACITY_COMMITMENT_EXPIRE_DUE_CONFIRMATION {
        bail!("容量承诺 Expire Due 缺少固定确认短语");
    }
    Ok(())
}

pub(super) fn validate_create_dependencies_on(
    conn: &Connection,
    input: &CreateComputeCapacityCommitment,
    recorded_at: &str,
) -> Result<ValidatedCreate> {
    let now = parse_utc("Store time", recorded_at)?;
    let provider = current_registered_provider_on(conn, &input.provider_id)?
        .ok_or_else(|| anyhow!("容量承诺 Provider 不存在"))?;
    if provider.provider.owner_account_id != input.owner_account_id
        || provider.provider.policy_revision != input.provider_policy_revision
        || provider.provider_digest != input.provider_digest
        || provider.provider.status != PROVIDER_STATUS_ACTIVE
        || provider.provider.provider_kind == PROVIDER_KIND_EXTERNAL_POOL
    {
        bail!("容量承诺只能绑定本人 current active local Provider exact revision/digest");
    }

    let offer_receipt = current_registered_offer_on(conn, &input.offer_id)?
        .ok_or_else(|| anyhow!("容量承诺 Offer 不存在"))?;
    if offer_receipt.provider_policy_revision != provider.provider.policy_revision
        || offer_receipt.provider_digest != provider.provider_digest
    {
        bail!("容量承诺 current Offer 未绑定 current Provider exact policy/digest");
    }
    let offer = offer_receipt.offer;
    if offer.offer_version != input.offer_version
        || offer.offer_digest != input.offer_digest
        || offer.status != OFFER_STATUS_ACTIVE
        || offer.provider_id != provider.provider.provider_id
        || offer.provider_kind != provider.provider.provider_kind
        || offer.capacity_pool != input.pool
        || offer.price_terms.pricing_mode != PRICING_MODE_CAPACITY_FUTURE
        || offer.price_terms.instrument_id.as_deref() != Some(input.instrument_id.as_str())
    {
        bail!("容量承诺只能绑定 current active Offer 的 exact Provider/Pool/future instrument");
    }
    if now < parse_utc("Offer valid_from", &offer.valid_from)?
        || now >= parse_utc("Offer valid_until", &offer.valid_until)?
    {
        bail!("容量承诺 Offer 当前不在有效期内");
    }

    let pool = current_capacity_pool_on(conn, &input.pool.pool_id)?
        .ok_or_else(|| anyhow!("容量承诺 CapacityPool 不存在"))?;
    if pool.binding != input.pool
        || pool.status != ComputeCapacityPoolStatus::Active
        || pool.provider_id != input.provider_id
        || pool.resource_profile_digest != offer.resource_profile.declared_profile_digest
        || pool.region_or_data_zone != offer.sku.region_or_data_zone
    {
        bail!("容量承诺 Pool 必须是 current active exact revision/digest 并匹配 Offer");
    }
    validate_pool_meter_policies(&pool.meter_policies, &offer)?;

    let snapshot = registered_price_snapshot_on(conn, &input.price_snapshot_id)?
        .ok_or_else(|| anyhow!("容量承诺 v171 Price Snapshot 不存在"))?;
    let delivery_window = offer
        .delivery_windows
        .iter()
        .find(|window| window.binding == input.delivery_window)
        .cloned()
        .ok_or_else(|| anyhow!("容量承诺 Offer 不包含 exact 交付窗口"))?;
    if now >= parse_utc("delivery window start", &delivery_window.starts_at_utc)? {
        bail!("容量承诺只能在交付窗口开始前创建");
    }
    validate_snapshot(input, &offer, &snapshot, &delivery_window, &now)?;

    let reference = audited_platform_reference_snapshot_binding_on(
        conn,
        &input.price_snapshot_id,
        &input.reference_binding_id,
        &input.reference_binding_digest,
    )?
    .ok_or_else(|| anyhow!("容量承诺 Price Snapshot 缺少 exact v223 binding"))?;
    if reference.snapshot_digest != input.price_snapshot_digest
        || reference.source_kind != PRICE_SOURCE_FALLBACK_CURVE
        || reference.status != "snapshot_registered"
    {
        bail!("容量承诺 v223 binding 与 Snapshot/source/status 不一致");
    }

    let instrument_authority = require_current_capacity_instrument_adoption_on(
        conn,
        &offer,
        snapshot.instrument_id.as_deref(),
    )?
    .ok_or_else(|| anyhow!("容量承诺 future Offer 缺少 CapacityInstrument adoption authority"))?;
    validate_contract_multiple(
        &input.quantities,
        &instrument_authority.instrument.contract_units,
    )?;

    let claim_lines = validate_meter_window_and_offer_cap_on(conn, input, &offer, &snapshot)?;
    Ok(ValidatedCreate {
        offer,
        snapshot,
        delivery_window,
        claim_lines,
    })
}

fn validate_contract_multiple(
    quantities: &[crate::compute_federation::capacity_commitment::ComputeCapacityCommitmentQuantity],
    contract_units: &[crate::compute_federation::capacity_instrument::ComputeCapacityInstrumentContractUnit],
) -> Result<()> {
    let requested = quantities
        .iter()
        .map(|value| (value.meter.as_str(), value.quantity_units))
        .collect::<BTreeMap<_, _>>();
    let contract = contract_units
        .iter()
        .map(|value| (value.meter.as_str(), value.quantity_units))
        .collect::<BTreeMap<_, _>>();
    if requested.len() != quantities.len()
        || contract.len() != contract_units.len()
        || requested.keys().copied().collect::<BTreeSet<_>>()
            != contract.keys().copied().collect::<BTreeSet<_>>()
    {
        bail!("容量承诺必须覆盖 CapacityInstrument 的完整 contract unit meter 集合");
    }
    let mut multiplier = None;
    for (meter, contract_quantity) in contract {
        let quantity = requested[&meter];
        if contract_quantity <= 0 || quantity <= 0 || quantity % contract_quantity != 0 {
            bail!("容量承诺 meter {meter} 必须是 CapacityInstrument 合约数量的整数倍");
        }
        let current = quantity / contract_quantity;
        if multiplier
            .replace(current)
            .is_some_and(|expected| expected != current)
        {
            bail!("容量承诺所有 meter 必须采用同一个整份 CapacityInstrument 合约倍数");
        }
    }
    Ok(())
}

fn validate_snapshot(
    input: &CreateComputeCapacityCommitment,
    offer: &ComputeOffer,
    snapshot: &ComputePriceSnapshot,
    window: &ComputeDeliveryWindow,
    now: &DateTime<FixedOffset>,
) -> Result<()> {
    if snapshot.snapshot_digest != input.price_snapshot_digest
        || snapshot.offer_id != offer.offer_id
        || snapshot.offer_version != offer.offer_version
        || snapshot.offer_digest != offer.offer_digest
        || snapshot.provider_id != offer.provider_id
        || snapshot.sku != offer.sku
        || snapshot.delivery_window != *window
        || snapshot.pricing_mode != PRICING_MODE_CAPACITY_FUTURE
        || snapshot.instrument_id.as_deref() != Some(input.instrument_id.as_str())
        || snapshot.currency != "CNY"
        || snapshot.components != offer.price_terms.components
        || snapshot.fee_rules != offer.price_terms.fee_rules
        || snapshot.trade_id.is_some()
        || snapshot.price_source.source_kind != PRICE_SOURCE_FALLBACK_CURVE
        || snapshot.price_source.sample_count != 0
    {
        bail!("容量承诺 v171 Snapshot 未与 Offer/SKU/window/future terms 精确绑定");
    }
    if *now >= parse_utc("Price Snapshot expires_at", &snapshot.expires_at)? {
        bail!("容量承诺 Price Snapshot 已过期");
    }
    Ok(())
}

fn validate_pool_meter_policies(
    policies: &[ComputeCapacityMeterPolicy],
    offer: &ComputeOffer,
) -> Result<()> {
    let by_meter = policies
        .iter()
        .map(|policy| (policy.meter.as_str(), policy))
        .collect::<BTreeMap<_, _>>();
    for line in &offer.capacity {
        let policy = by_meter
            .get(line.bucket.meter.as_str())
            .ok_or_else(|| anyhow!("Offer meter 不在 current Pool policy 中"))?;
        if policy.meter_mode != line.bucket.meter_mode
            || policy.quantum_units != line.bucket.quantum_units
            || policy.policy_digest != line.bucket.meter_policy_digest
        {
            bail!("Offer bucket 与 current Pool meter policy 不一致");
        }
    }
    Ok(())
}

fn validate_meter_window_and_offer_cap_on(
    conn: &Connection,
    input: &CreateComputeCapacityCommitment,
    offer: &ComputeOffer,
    snapshot: &ComputePriceSnapshot,
) -> Result<Vec<HoldComputeCapacityClaimLine>> {
    let window_rows = offer
        .capacity
        .iter()
        .filter(|line| line.bucket.delivery_window == input.delivery_window)
        .map(|line| (line.bucket.meter.as_str(), line))
        .collect::<BTreeMap<_, _>>();
    let components = snapshot
        .components
        .iter()
        .map(|component| (component.meter.as_str(), component))
        .collect::<BTreeMap<_, _>>();
    let quantities = input
        .quantities
        .iter()
        .map(|quantity| (quantity.meter.as_str(), quantity))
        .collect::<BTreeMap<_, _>>();
    let sku_meters = offer
        .sku
        .metering_units
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let exact_window_row_count = offer
        .capacity
        .iter()
        .filter(|line| line.bucket.delivery_window == input.delivery_window)
        .count();
    if window_rows.len() != exact_window_row_count
        || components.len() != snapshot.components.len()
        || quantities.len() != input.quantities.len()
        || window_rows.keys().copied().collect::<BTreeSet<_>>() != sku_meters
        || components.keys().copied().collect::<BTreeSet<_>>() != sku_meters
        || quantities.keys().copied().collect::<BTreeSet<_>>() != sku_meters
    {
        bail!("容量承诺必须覆盖 SKU/Offer exact window/Snapshot 的完整 meter 集合");
    }

    let mut lines = Vec::with_capacity(sku_meters.len());
    for meter in sku_meters {
        let offer_line = window_rows[&meter];
        let component = components[&meter];
        let quantity = quantities[&meter].quantity_units;
        if quantity <= 0
            || quantity % offer_line.bucket.quantum_units != 0
            || quantity % component.unit_size != 0
            || quantity > component.max_units
            || quantity > offer_line.reservable_units
        {
            bail!("容量承诺 meter {meter} 的数量、粒度或上限无效");
        }
        let live = live_offer_bucket_units_on(conn, &offer.offer_id, &offer_line.bucket.bucket_id)?;
        let next = live
            .checked_add(i128::from(quantity))
            .context("容量承诺 Offer live quantity 溢出")?;
        if next > i128::from(offer_line.reservable_units) {
            bail!("容量承诺会超过 current Offer meter {meter} 的 reservable_units");
        }
        lines.push(HoldComputeCapacityClaimLine {
            bucket_id: offer_line.bucket.bucket_id.clone(),
            quantity_units: quantity,
        });
    }
    Ok(lines)
}

fn live_offer_bucket_units_on(conn: &Connection, offer_id: &str, bucket_id: &str) -> Result<i128> {
    let mut statement = conn.prepare(
        "SELECT lines.quantity_units
           FROM compute_capacity_claims claims
           JOIN compute_capacity_claim_lines lines ON lines.claim_id=claims.claim_id
          WHERE claims.status IN ('held','active')
            AND claims.claim_kind IN ('quote_hold','reservation','capacity_commitment')
            AND lines.bucket_id=?1
            AND EXISTS (
                SELECT 1 FROM compute_capacity_ledger_transactions held
                 WHERE held.claim_id=claims.claim_id
                   AND held.claim_effect='held' AND held.offer_id=?2
            )
          ORDER BY claims.claim_id",
    )?;
    let values = statement.query_map(params![bucket_id, offer_id], |row| row.get::<_, i64>(0))?;
    let mut total = 0_i128;
    for value in values {
        total = total
            .checked_add(i128::from(value?))
            .context("容量承诺 live Offer quantity 汇总溢出")?;
    }
    Ok(total)
}

fn validate_quantities(
    values: &[crate::compute_federation::capacity_commitment::ComputeCapacityCommitmentQuantity],
) -> Result<()> {
    if values.is_empty() || values.len() > 64 {
        bail!("容量承诺 quantities 数量必须在 1 到 64 之间");
    }
    let mut meters = BTreeSet::new();
    for value in values {
        validate_exact("capacity commitment meter", &value.meter, 160)?;
        if value.quantity_units <= 0 || !meters.insert(value.meter.as_str()) {
            bail!("容量承诺 meter 必须唯一且 quantity_units 为正整数");
        }
    }
    Ok(())
}

pub(super) fn parse_utc(label: &str, value: &str) -> Result<DateTime<FixedOffset>> {
    let value = DateTime::parse_from_rfc3339(value).map_err(|_| anyhow!("{label} 不是 RFC3339"))?;
    if value.offset().local_minus_utc() != 0 {
        bail!("{label} 必须使用 UTC 时区");
    }
    Ok(value)
}

pub(super) fn validate_exact(label: &str, value: &str, max: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("{label} 为空、过长、未规范化或包含控制字符");
    }
    Ok(())
}

pub(super) fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
    {
        bail!("{label} 必须是 64 位小写十六进制 SHA-256");
    }
    Ok(())
}

pub(super) fn offer_binding(offer: &ComputeOffer) -> ComputeCapacityOfferBinding {
    ComputeCapacityOfferBinding {
        offer_id: offer.offer_id.clone(),
        offer_version: offer.offer_version,
        offer_digest: offer.offer_digest.clone(),
    }
}
