use std::collections::BTreeMap;

use anyhow::{bail, Result};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaim, ComputeCapacityClaimLine},
    execution::{ComputeReservation, ComputeReservedCapacity},
    market::{ComputePriceComponent, ComputePriceSnapshot},
    offer::{ComputeOffer, ComputeOfferCapacity},
    workload::ComputeUsageLimit,
};

use super::validate_exact_value;

pub(super) fn validate_reserved_capacity(
    reservation: &ComputeReservation,
    job_limits: &[ComputeUsageLimit],
    offer: &ComputeOffer,
    snapshot: &ComputePriceSnapshot,
    claim: &ComputeCapacityClaim,
) -> Result<()> {
    if reservation.reserved_capacity.is_empty() {
        bail!("算力 Reservation 至少需要一个容量 meter");
    }
    let reserved = unique_reserved_capacity(&reservation.reserved_capacity)?;
    let limits = unique_job_limits(job_limits)?;
    let prices = unique_price_components(&snapshot.components)?;
    let offered = unique_offer_capacity(&offer.capacity)?;
    let claimed = unique_claim_lines(claim)?;
    if reserved.len() != prices.len()
        || reserved.len() != limits.len()
        || reserved.len() != claimed.len()
    {
        bail!("Reservation、Job、Price Snapshot 与 Capacity Claim 的 meter 集合不一致");
    }

    for (meter, quantity) in reserved {
        let limit = limits
            .get(meter)
            .ok_or_else(|| anyhow::anyhow!("Reservation meter 不在 Job 使用量上限中"))?;
        let component = prices
            .get(meter)
            .ok_or_else(|| anyhow::anyhow!("Reservation meter 不在 Price Snapshot 中"))?;
        let offer_capacity = offered
            .get(meter)
            .ok_or_else(|| anyhow::anyhow!("Reservation meter 不在 Offer 容量中"))?;
        let claim_line = claimed
            .get(meter)
            .ok_or_else(|| anyhow::anyhow!("Reservation meter 缺少 Capacity Claim 行"))?;
        if component.unit_size <= 0
            || quantity > **limit
            || quantity > component.max_units
            || quantity % component.unit_size != 0
        {
            bail!("Reservation 容量超过 Job/Price Snapshot 上限或不符合计价粒度");
        }
        if quantity > offer_capacity.reservable_units
            || quantity > offer_capacity.total_units
            || claim_line.quantity_units != quantity
            || claim_line.bucket != offer_capacity.bucket
        {
            bail!("Reservation 容量与 Offer 静态上限或 Capacity Claim bucket 不一致");
        }
    }
    Ok(())
}

fn unique_reserved_capacity(capacity: &[ComputeReservedCapacity]) -> Result<BTreeMap<&str, i64>> {
    let mut result = BTreeMap::new();
    for item in capacity {
        validate_exact_value("Reservation capacity meter", &item.meter)?;
        if item.quantity <= 0 || result.insert(item.meter.as_str(), item.quantity).is_some() {
            bail!("Reservation capacity 数量无效或 meter 重复");
        }
    }
    Ok(result)
}

fn unique_job_limits(limits: &[ComputeUsageLimit]) -> Result<BTreeMap<&str, &i64>> {
    let mut result = BTreeMap::new();
    for limit in limits {
        if result
            .insert(limit.meter.as_str(), &limit.max_quantity)
            .is_some()
        {
            bail!("Job 使用量上限 meter 重复");
        }
    }
    Ok(result)
}

fn unique_price_components(
    components: &[ComputePriceComponent],
) -> Result<BTreeMap<&str, &ComputePriceComponent>> {
    let mut result = BTreeMap::new();
    for component in components {
        if result.insert(component.meter.as_str(), component).is_some() {
            bail!("Price Snapshot meter 重复");
        }
    }
    Ok(result)
}

fn unique_offer_capacity(
    capacity: &[ComputeOfferCapacity],
) -> Result<BTreeMap<&str, &ComputeOfferCapacity>> {
    let mut result = BTreeMap::new();
    for item in capacity {
        if result.insert(item.bucket.meter.as_str(), item).is_some() {
            bail!("Offer 容量 meter 重复");
        }
    }
    Ok(result)
}

fn unique_claim_lines(
    claim: &ComputeCapacityClaim,
) -> Result<BTreeMap<&str, &ComputeCapacityClaimLine>> {
    let mut result = BTreeMap::new();
    for line in &claim.lines {
        if result.insert(line.bucket.meter.as_str(), line).is_some() {
            bail!("Capacity Claim meter 重复");
        }
    }
    Ok(result)
}
