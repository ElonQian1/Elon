use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::{
    capacity::ComputeCapacityBucketStatus,
    offer::{
        ComputeOffer, OFFER_STATUS_ACTIVE, OFFER_STATUS_DRAFT, OFFER_STATUS_DRAINING,
        OFFER_STATUS_EXPIRED, OFFER_STATUS_REVOKED,
    },
};

use super::super::{
    compute_capacity_pool_queries::audited_compute_capacity_pool_version_on,
    compute_capacity_rows::{stored_bucket_on, stored_bucket_reference_on},
};

pub(super) fn audit_offer_capacity_references_on(
    conn: &Connection,
    offer: &ComputeOffer,
    require_bucket_heads: bool,
) -> Result<()> {
    ensure_offer_pool_reference_on(conn, offer, false)?;
    if require_bucket_heads {
        ensure_offer_bucket_references_on(conn, offer, false)
    } else {
        ensure_offer_immutable_bucket_references_on(conn, offer)
    }
}

pub(super) fn ensure_offer_capacity_references_on(
    conn: &Connection,
    offer: &ComputeOffer,
    publishing: bool,
) -> Result<()> {
    ensure_offer_pool_reference_on(conn, offer, publishing)?;
    ensure_offer_bucket_references_on(conn, offer, publishing)
}

fn ensure_offer_pool_reference_on(
    conn: &Connection,
    offer: &ComputeOffer,
    publishing: bool,
) -> Result<()> {
    let pool = audited_compute_capacity_pool_version_on(
        conn,
        &offer.capacity_pool.pool_id,
        offer.capacity_pool.capacity_epoch,
        offer.capacity_pool.pool_revision,
    )?
    .ok_or_else(|| anyhow!("算力 Offer 容量池版本不存在"))?;
    if pool.provider_id != offer.provider_id
        || pool.binding != offer.capacity_pool
        || pool.region_or_data_zone != offer.sku.region_or_data_zone
        || pool.resource_profile_digest != offer.resource_profile.declared_profile_digest
    {
        bail!("算力 Offer 与容量池 Provider、摘要、区域或资源档案不一致");
    }
    if publishing {
        let current = conn
            .query_row(
                "SELECT status, current_capacity_epoch FROM compute_capacity_pools
                  WHERE pool_id=?1",
                params![offer.capacity_pool.pool_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("算力 Offer 当前容量池不存在"))?;
        if !pool_state_allows_offer(&current.0, current.1, offer) {
            bail!("容量池当前状态或 epoch 不允许发布该状态的 Offer");
        }
    }
    Ok(())
}

fn ensure_offer_bucket_references_on(
    conn: &Connection,
    offer: &ComputeOffer,
    publishing: bool,
) -> Result<()> {
    for capacity in &offer.capacity {
        let stored = stored_bucket_on(conn, &capacity.bucket.bucket_id)?
            .ok_or_else(|| anyhow!("算力 Offer 容量 bucket 不存在"))?;
        if stored.balance.binding != capacity.bucket {
            bail!("算力 Offer 容量 bucket 绑定与账本不一致");
        }
        let window = offer
            .delivery_windows
            .iter()
            .find(|window| window.binding == capacity.bucket.delivery_window)
            .ok_or_else(|| anyhow!("算力 Offer 容量 bucket 缺少对应交付窗口"))?;
        if stored.starts_at != window.starts_at_utc || stored.ends_at != window.ends_at_utc {
            bail!("算力 Offer 交付窗口与容量 bucket 时间不一致");
        }
        if publishing
            && matches!(
                offer.status.as_str(),
                OFFER_STATUS_DRAFT | OFFER_STATUS_ACTIVE
            )
            && stored.balance.status != ComputeCapacityBucketStatus::Open
        {
            bail!("draft 或 active Offer 只能引用 open 容量 bucket");
        }
        if publishing
            && offer.status == OFFER_STATUS_ACTIVE
            && stored.balance.issued_units < capacity.total_units
        {
            bail!("active Offer 的静态容量上限不能超过 bucket 已发行容量");
        }
    }
    Ok(())
}

fn ensure_offer_immutable_bucket_references_on(
    conn: &Connection,
    offer: &ComputeOffer,
) -> Result<()> {
    for capacity in &offer.capacity {
        let stored = stored_bucket_reference_on(conn, &capacity.bucket.bucket_id)?
            .ok_or_else(|| anyhow!("算力 Offer 容量 bucket 不存在"))?;
        if stored.binding != capacity.bucket {
            bail!("算力 Offer 容量 bucket immutable binding 不一致");
        }
        let window = offer
            .delivery_windows
            .iter()
            .find(|window| window.binding == capacity.bucket.delivery_window)
            .ok_or_else(|| anyhow!("算力 Offer 容量 bucket 缺少对应交付窗口"))?;
        if stored.starts_at != window.starts_at_utc || stored.ends_at != window.ends_at_utc {
            bail!("算力 Offer 交付窗口与容量 bucket immutable 时间不一致");
        }
    }
    Ok(())
}

fn pool_state_allows_offer(status: &str, current_epoch: i64, offer: &ComputeOffer) -> bool {
    if current_epoch != offer.capacity_pool.capacity_epoch {
        return matches!(
            offer.status.as_str(),
            OFFER_STATUS_EXPIRED | OFFER_STATUS_REVOKED
        );
    }
    match offer.status.as_str() {
        OFFER_STATUS_DRAFT => matches!(status, "registering" | "active"),
        OFFER_STATUS_ACTIVE => status == "active",
        OFFER_STATUS_DRAINING => matches!(status, "active" | "draining" | "quarantined"),
        OFFER_STATUS_EXPIRED | OFFER_STATUS_REVOKED => true,
        _ => false,
    }
}
