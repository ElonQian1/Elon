use anyhow::{anyhow, bail, Context, Result};
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::{
    capacity::ComputeCapacityBucketStatus,
    offer::{
        ComputeOffer, OFFER_STATUS_ACTIVE, OFFER_STATUS_DRAFT, OFFER_STATUS_DRAINING,
        OFFER_STATUS_EXPIRED, OFFER_STATUS_REVOKED,
    },
    provider::{
        PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_DRAINING, PROVIDER_STATUS_QUARANTINED,
        PROVIDER_STATUS_REGISTERING,
    },
};

use super::{
    compute_capacity_rows::stored_bucket_on,
    compute_offer_contract_validation::validate_offer_contract,
    compute_provider_registry::{current_registered_provider_on, registered_provider_version_on},
    now, Store,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeOfferRegistrationReceipt {
    pub offer: ComputeOffer,
    pub provider_policy_revision: i64,
    pub provider_digest: String,
    pub replayed: bool,
}

#[derive(Debug)]
struct CurrentOfferProjection {
    offer_id: String,
    provider_id: String,
    provider_kind: String,
    sku_id: String,
    sku_digest: String,
    capacity_pool_id: String,
    current_offer_version: i64,
    current_offer_digest: String,
    current_provider_policy_revision: i64,
    current_provider_digest: String,
    status: String,
    valid_from: String,
    valid_until: String,
    first_created_at: String,
    current_version_created_at: String,
}

#[derive(Debug)]
struct StoredOfferVersion {
    offer_id: String,
    offer_version: i64,
    offer_digest: String,
    provider_id: String,
    provider_policy_revision: i64,
    provider_digest: String,
    sku_id: String,
    sku_digest: String,
    capacity_pool_id: String,
    capacity_epoch: i64,
    pool_revision: i64,
    pool_digest: String,
    status: String,
    valid_from: String,
    valid_until: String,
    offer_json: String,
}

impl Store {
    pub(crate) fn register_compute_offer(
        &self,
        offer: &ComputeOffer,
    ) -> Result<ComputeOfferRegistrationReceipt> {
        if offer.offer_id.trim().is_empty() || offer.offer_version <= 0 {
            bail!("算力 Offer ID 或版本无效");
        }
        let offer_json = serde_json::to_string(offer)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = current_offer_projection_on(&tx, offer.offer_id.trim())?;

        if let Some(current) = current {
            let current_version =
                offer_version_on(&tx, offer.offer_id.trim(), current.current_offer_version)?
                    .ok_or_else(|| anyhow!("算力 Offer 当前历史版本缺失，拒绝继续写入"))?;
            audited_offer_on(&tx, Some(&current), &current_version)?;

            if offer.offer_version <= current.current_offer_version {
                let stored = offer_version_on(&tx, offer.offer_id.trim(), offer.offer_version)?
                    .ok_or_else(|| anyhow!("算力 Offer 历史版本缺失，拒绝覆盖"))?;
                let stored_offer = audited_offer_on(&tx, None, &stored)?;
                if stored.offer_json != offer_json || stored.offer_digest != offer.offer_digest {
                    bail!("相同算力 Offer 版本不能绑定不同合同");
                }
                tx.commit()?;
                return Ok(ComputeOfferRegistrationReceipt {
                    offer: stored_offer,
                    provider_policy_revision: stored.provider_policy_revision,
                    provider_digest: stored.provider_digest,
                    replayed: true,
                });
            }

            let provider = if matches!(
                offer.status.as_str(),
                OFFER_STATUS_EXPIRED | OFFER_STATUS_REVOKED
            ) {
                registered_provider_version_on(
                    &tx,
                    &current_version.provider_id,
                    current_version.provider_policy_revision,
                )?
                .ok_or_else(|| anyhow!("算力 Offer 当前版本的 Provider 历史版本不存在"))?
            } else {
                current_registered_provider_on(&tx, offer.provider_id.trim())?
                    .ok_or_else(|| anyhow!("算力 Offer Provider 不存在"))?
            };
            validate_offer_contract(offer, &provider.provider)?;
            validate_provider_state_for_publish(offer, &provider.provider.status)?;
            ensure_offer_capacity_references_on(&tx, offer, true)?;
            ensure_stable_offer_identity(offer, &current)?;
            if offer.offer_version != current.current_offer_version + 1 {
                bail!(
                    "算力 Offer 版本必须连续递增，当前版本为 {}",
                    current.current_offer_version
                );
            }
            if !offer_status_transition_allowed(&current.status, &offer.status) {
                bail!(
                    "算力 Offer 状态不允许从 {} 变更为 {}",
                    current.status,
                    offer.status
                );
            }
            ensure_version_time_monotonic(&current.current_version_created_at, &offer.created_at)?;

            insert_offer_version(&tx, offer, &offer_json, &provider)?;
            let updated = tx.execute(
                "UPDATE compute_offers
                    SET current_offer_version=?1, current_offer_digest=?2,
                        current_provider_policy_revision=?3, current_provider_digest=?4,
                        status=?5, valid_from=?6, valid_until=?7,
                        current_version_created_at=?8, recorded_at=?9
                  WHERE offer_id=?10 AND current_offer_version=?11
                    AND current_offer_digest=?12",
                params![
                    offer.offer_version,
                    offer.offer_digest,
                    provider.provider.policy_revision,
                    provider.provider_digest,
                    offer.status,
                    offer.valid_from,
                    offer.valid_until,
                    offer.created_at,
                    now(),
                    offer.offer_id,
                    current.current_offer_version,
                    current.current_offer_digest,
                ],
            )?;
            if updated != 1 {
                bail!("算力 Offer 当前投影已变化，请基于最新版本重试");
            }
            tx.commit()?;
            return Ok(ComputeOfferRegistrationReceipt {
                offer: offer.clone(),
                provider_policy_revision: provider.provider.policy_revision,
                provider_digest: provider.provider_digest,
                replayed: false,
            });
        }

        let provider = current_registered_provider_on(&tx, offer.provider_id.trim())?
            .ok_or_else(|| anyhow!("算力 Offer Provider 不存在"))?;
        validate_offer_contract(offer, &provider.provider)?;
        validate_provider_state_for_publish(offer, &provider.provider.status)?;
        ensure_offer_capacity_references_on(&tx, offer, true)?;
        if offer.offer_version != 1 || offer.status != OFFER_STATUS_DRAFT {
            bail!("新算力 Offer 必须以 draft 状态和版本 1 创建");
        }
        tx.execute(
            "INSERT INTO compute_offers (
                offer_id, provider_id, provider_kind, sku_id, sku_digest,
                capacity_pool_id, current_offer_version, current_offer_digest,
                current_provider_policy_revision, current_provider_digest,
                status, valid_from, valid_until, first_created_at,
                current_version_created_at, recorded_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                ?9, ?10, ?11, ?12, ?13, ?14, ?14, ?15
             )",
            params![
                offer.offer_id,
                offer.provider_id,
                offer.provider_kind,
                offer.sku.sku_id,
                offer.sku.sku_digest,
                offer.capacity_pool.pool_id,
                offer.offer_version,
                offer.offer_digest,
                provider.provider.policy_revision,
                provider.provider_digest,
                offer.status,
                offer.valid_from,
                offer.valid_until,
                offer.created_at,
                now(),
            ],
        )?;
        insert_offer_version(&tx, offer, &offer_json, &provider)?;
        tx.commit()?;
        Ok(ComputeOfferRegistrationReceipt {
            offer: offer.clone(),
            provider_policy_revision: provider.provider.policy_revision,
            provider_digest: provider.provider_digest,
            replayed: false,
        })
    }

    pub(crate) fn compute_offer(&self, offer_id: &str) -> Result<ComputeOfferRegistrationReceipt> {
        if offer_id.trim().is_empty() {
            bail!("算力 Offer ID 不能为空");
        }
        let conn = self.conn()?;
        current_registered_offer_on(&conn, offer_id.trim())?
            .ok_or_else(|| anyhow!("算力 Offer 不存在"))
    }
}

pub(super) fn current_registered_offer_on(
    conn: &Connection,
    offer_id: &str,
) -> Result<Option<ComputeOfferRegistrationReceipt>> {
    let Some(projection) = current_offer_projection_on(conn, offer_id)? else {
        return Ok(None);
    };
    let stored = offer_version_on(conn, offer_id, projection.current_offer_version)?
        .ok_or_else(|| anyhow!("算力 Offer 当前历史版本缺失"))?;
    let offer = audited_offer_on(conn, Some(&projection), &stored)?;
    Ok(Some(ComputeOfferRegistrationReceipt {
        offer,
        provider_policy_revision: stored.provider_policy_revision,
        provider_digest: stored.provider_digest,
        replayed: false,
    }))
}

pub(super) fn registered_offer_version_on(
    conn: &Connection,
    offer_id: &str,
    offer_version: i64,
) -> Result<Option<ComputeOfferRegistrationReceipt>> {
    let Some(stored) = offer_version_on(conn, offer_id, offer_version)? else {
        return Ok(None);
    };
    let offer = audited_offer_on(conn, None, &stored)?;
    Ok(Some(ComputeOfferRegistrationReceipt {
        offer,
        provider_policy_revision: stored.provider_policy_revision,
        provider_digest: stored.provider_digest,
        replayed: false,
    }))
}

fn insert_offer_version(
    conn: &Connection,
    offer: &ComputeOffer,
    offer_json: &str,
    provider: &super::compute_provider_registry::ComputeProviderRegistrationReceipt,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_offer_versions (
            offer_id, offer_version, offer_digest, provider_id,
            provider_policy_revision, provider_digest, sku_id, sku_digest,
            capacity_pool_id, capacity_epoch, pool_revision, pool_digest,
            status, valid_from, valid_until, offer_json, created_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
            ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
         )",
        params![
            offer.offer_id,
            offer.offer_version,
            offer.offer_digest,
            offer.provider_id,
            provider.provider.policy_revision,
            provider.provider_digest,
            offer.sku.sku_id,
            offer.sku.sku_digest,
            offer.capacity_pool.pool_id,
            offer.capacity_pool.capacity_epoch,
            offer.capacity_pool.pool_revision,
            offer.capacity_pool.pool_digest,
            offer.status,
            offer.valid_from,
            offer.valid_until,
            offer_json,
            now(),
        ],
    )?;
    Ok(())
}

fn audited_offer_on(
    conn: &Connection,
    projection: Option<&CurrentOfferProjection>,
    stored: &StoredOfferVersion,
) -> Result<ComputeOffer> {
    let provider =
        registered_provider_version_on(conn, &stored.provider_id, stored.provider_policy_revision)?
            .ok_or_else(|| anyhow!("算力 Offer 绑定的 Provider 历史版本不存在"))?;
    if provider.provider_digest != stored.provider_digest {
        bail!("算力 Offer 绑定的 Provider 摘要不一致");
    }
    let offer: ComputeOffer =
        serde_json::from_str(&stored.offer_json).context("算力 Offer 历史版本 JSON 无效")?;
    let computed_digest = validate_offer_contract(&offer, &provider.provider)?;
    if computed_digest != stored.offer_digest
        || offer.offer_id != stored.offer_id
        || offer.offer_version != stored.offer_version
        || offer.provider_id != stored.provider_id
        || offer.sku.sku_id != stored.sku_id
        || offer.sku.sku_digest != stored.sku_digest
        || offer.capacity_pool.pool_id != stored.capacity_pool_id
        || offer.capacity_pool.capacity_epoch != stored.capacity_epoch
        || offer.capacity_pool.pool_revision != stored.pool_revision
        || offer.capacity_pool.pool_digest != stored.pool_digest
        || offer.status != stored.status
        || offer.valid_from != stored.valid_from
        || offer.valid_until != stored.valid_until
    {
        bail!("算力 Offer 历史版本身份、摘要或投影字段审计失败");
    }
    ensure_offer_capacity_references_on(conn, &offer, false)?;
    if let Some(projection) = projection {
        if stored.offer_version == 1 {
            if offer.created_at != projection.first_created_at {
                bail!("算力 Offer 首版本创建时间与当前投影不一致");
            }
        } else {
            let first_stored = offer_version_on(conn, &projection.offer_id, 1)?
                .ok_or_else(|| anyhow!("算力 Offer 首个历史版本缺失"))?;
            let first_offer = audited_offer_on(conn, None, &first_stored)?;
            if first_offer.created_at != projection.first_created_at {
                bail!("算力 Offer 首版本创建时间与当前投影不一致");
            }
        }
        ensure_current_offer_projection(&offer, stored, projection)?;
    }
    Ok(offer)
}

fn ensure_current_offer_projection(
    offer: &ComputeOffer,
    stored: &StoredOfferVersion,
    projection: &CurrentOfferProjection,
) -> Result<()> {
    if offer.offer_id != projection.offer_id
        || offer.provider_id != projection.provider_id
        || offer.provider_kind != projection.provider_kind
        || offer.sku.sku_id != projection.sku_id
        || offer.sku.sku_digest != projection.sku_digest
        || offer.capacity_pool.pool_id != projection.capacity_pool_id
        || offer.offer_version != projection.current_offer_version
        || offer.offer_digest != projection.current_offer_digest
        || stored.provider_policy_revision != projection.current_provider_policy_revision
        || stored.provider_digest != projection.current_provider_digest
        || offer.status != projection.status
        || offer.valid_from != projection.valid_from
        || offer.valid_until != projection.valid_until
        || offer.created_at != projection.current_version_created_at
    {
        bail!("算力 Offer 当前投影与不可变版本不一致");
    }
    Ok(())
}

fn ensure_stable_offer_identity(
    offer: &ComputeOffer,
    current: &CurrentOfferProjection,
) -> Result<()> {
    if offer.provider_id != current.provider_id
        || offer.provider_kind != current.provider_kind
        || offer.sku.sku_id != current.sku_id
        || offer.sku.sku_digest != current.sku_digest
        || offer.capacity_pool.pool_id != current.capacity_pool_id
    {
        bail!("算力 Offer 的 Provider、SKU 和容量池稳定身份不能原地改变");
    }
    Ok(())
}

fn validate_provider_state_for_publish(offer: &ComputeOffer, provider_status: &str) -> Result<()> {
    let allowed = match offer.status.as_str() {
        OFFER_STATUS_DRAFT => {
            matches!(
                provider_status,
                PROVIDER_STATUS_REGISTERING | PROVIDER_STATUS_ACTIVE
            )
        }
        OFFER_STATUS_ACTIVE => provider_status == PROVIDER_STATUS_ACTIVE,
        OFFER_STATUS_DRAINING => {
            matches!(
                provider_status,
                PROVIDER_STATUS_ACTIVE | PROVIDER_STATUS_DRAINING | PROVIDER_STATUS_QUARANTINED
            )
        }
        OFFER_STATUS_EXPIRED | OFFER_STATUS_REVOKED => true,
        _ => false,
    };
    if !allowed {
        bail!(
            "Provider 当前状态 {provider_status} 不允许发布 {} Offer",
            offer.status
        );
    }
    if offer.status == OFFER_STATUS_ACTIVE {
        let valid_until = DateTime::parse_from_rfc3339(&offer.valid_until)
            .context("active Offer 失效时间不是 RFC3339")?;
        if valid_until <= chrono::Utc::now() {
            bail!("不能发布已经失效的 active Offer");
        }
    }
    Ok(())
}

fn offer_status_transition_allowed(current: &str, next: &str) -> bool {
    match current {
        OFFER_STATUS_DRAFT => matches!(
            next,
            OFFER_STATUS_DRAFT | OFFER_STATUS_ACTIVE | OFFER_STATUS_REVOKED
        ),
        OFFER_STATUS_ACTIVE => matches!(
            next,
            OFFER_STATUS_ACTIVE
                | OFFER_STATUS_DRAINING
                | OFFER_STATUS_EXPIRED
                | OFFER_STATUS_REVOKED
        ),
        OFFER_STATUS_DRAINING => matches!(
            next,
            OFFER_STATUS_DRAINING | OFFER_STATUS_EXPIRED | OFFER_STATUS_REVOKED
        ),
        OFFER_STATUS_EXPIRED | OFFER_STATUS_REVOKED => false,
        _ => false,
    }
}

fn ensure_version_time_monotonic(previous: &str, next: &str) -> Result<()> {
    let previous = DateTime::parse_from_rfc3339(previous)
        .context("算力 Offer 当前版本创建时间不是 RFC3339")?;
    let next =
        DateTime::parse_from_rfc3339(next).context("算力 Offer 新版本创建时间不是 RFC3339")?;
    if next < previous {
        bail!("算力 Offer 新版本创建时间不能早于当前版本");
    }
    Ok(())
}

fn ensure_offer_capacity_references_on(
    conn: &Connection,
    offer: &ComputeOffer,
    publishing: bool,
) -> Result<()> {
    let pool = conn
        .query_row(
            "SELECT p.provider_id, p.status, p.current_capacity_epoch,
                    pv.pool_digest, pv.resource_profile_json, COALESCE(pv.region, '')
               FROM compute_capacity_pools p
               JOIN compute_capacity_pool_versions pv ON pv.pool_id=p.pool_id
              WHERE pv.pool_id=?1 AND pv.capacity_epoch=?2 AND pv.pool_revision=?3",
            params![
                offer.capacity_pool.pool_id,
                offer.capacity_pool.capacity_epoch,
                offer.capacity_pool.pool_revision,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("算力 Offer 容量池版本不存在"))?;
    let resource_profile: serde_json::Value =
        serde_json::from_str(&pool.4).context("容量池资源档案 JSON 无效")?;
    let profile_digest = resource_profile
        .get("digest")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("容量池资源档案缺少摘要"))?;
    if pool.0 != offer.provider_id
        || pool.3 != offer.capacity_pool.pool_digest
        || pool.5 != offer.sku.region_or_data_zone
        || profile_digest != offer.resource_profile.declared_profile_digest
    {
        bail!("算力 Offer 与容量池 Provider、摘要、区域或资源档案不一致");
    }
    if publishing && !pool_state_allows_offer(&pool.1, pool.2, offer)? {
        bail!("容量池当前状态或 epoch 不允许发布该状态的 Offer");
    }

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

fn current_offer_projection_on(
    conn: &Connection,
    offer_id: &str,
) -> Result<Option<CurrentOfferProjection>> {
    conn.query_row(
        "SELECT offer_id, provider_id, provider_kind, sku_id, sku_digest,
                capacity_pool_id, current_offer_version, current_offer_digest,
                current_provider_policy_revision, current_provider_digest,
                status, valid_from, valid_until, first_created_at,
                current_version_created_at
           FROM compute_offers WHERE offer_id=?1",
        params![offer_id],
        |row| {
            Ok(CurrentOfferProjection {
                offer_id: row.get(0)?,
                provider_id: row.get(1)?,
                provider_kind: row.get(2)?,
                sku_id: row.get(3)?,
                sku_digest: row.get(4)?,
                capacity_pool_id: row.get(5)?,
                current_offer_version: row.get(6)?,
                current_offer_digest: row.get(7)?,
                current_provider_policy_revision: row.get(8)?,
                current_provider_digest: row.get(9)?,
                status: row.get(10)?,
                valid_from: row.get(11)?,
                valid_until: row.get(12)?,
                first_created_at: row.get(13)?,
                current_version_created_at: row.get(14)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn offer_version_on(
    conn: &Connection,
    offer_id: &str,
    offer_version: i64,
) -> Result<Option<StoredOfferVersion>> {
    conn.query_row(
        "SELECT offer_id, offer_version, offer_digest, provider_id,
                provider_policy_revision, provider_digest, sku_id, sku_digest,
                capacity_pool_id, capacity_epoch, pool_revision, pool_digest,
                status, valid_from, valid_until, offer_json
           FROM compute_offer_versions
          WHERE offer_id=?1 AND offer_version=?2",
        params![offer_id, offer_version],
        |row| {
            Ok(StoredOfferVersion {
                offer_id: row.get(0)?,
                offer_version: row.get(1)?,
                offer_digest: row.get(2)?,
                provider_id: row.get(3)?,
                provider_policy_revision: row.get(4)?,
                provider_digest: row.get(5)?,
                sku_id: row.get(6)?,
                sku_digest: row.get(7)?,
                capacity_pool_id: row.get(8)?,
                capacity_epoch: row.get(9)?,
                pool_revision: row.get(10)?,
                pool_digest: row.get(11)?,
                status: row.get(12)?,
                valid_from: row.get(13)?,
                valid_until: row.get(14)?,
                offer_json: row.get(15)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}
