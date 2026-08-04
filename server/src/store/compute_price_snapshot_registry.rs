use anyhow::{anyhow, bail, Context, Result};
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::market::ComputePriceSnapshot;

use super::{
    compute_offer_registry::{current_registered_offer_on, registered_offer_version_on},
    compute_price_snapshot_validation::validate_price_snapshot_contract,
    now, Store,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputePriceSnapshotRegistrationReceipt {
    pub snapshot: ComputePriceSnapshot,
    pub replayed: bool,
}

#[derive(Debug)]
struct StoredPriceSnapshot {
    snapshot_id: String,
    snapshot_digest: String,
    quote_id: String,
    pricing_mode: String,
    sku_id: String,
    sku_digest: String,
    provider_id: String,
    offer_id: String,
    offer_version: i64,
    offer_digest: String,
    delivery_window_id: String,
    delivery_window_digest: String,
    currency: String,
    consumer_max_amount_micros: i64,
    provider_max_amount_micros: i64,
    price_source_kind: String,
    price_source_id: String,
    price_source_version: i64,
    price_source_digest: String,
    trade_id: Option<String>,
    instrument_id: Option<String>,
    quoted_at: String,
    expires_at: String,
    snapshot_json: String,
}

impl Store {
    pub(crate) fn register_compute_price_snapshot(
        &self,
        snapshot: &ComputePriceSnapshot,
    ) -> Result<ComputePriceSnapshotRegistrationReceipt> {
        if snapshot.snapshot_id.trim().is_empty() || snapshot.quote_id.trim().is_empty() {
            bail!("算力价格快照 ID 和报价 ID 不能为空");
        }
        let snapshot_json = serde_json::to_string(snapshot)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = price_snapshot_on(&tx, snapshot.snapshot_id.trim())? {
            let stored_snapshot = audited_price_snapshot_on(&tx, &stored)?;
            if stored.snapshot_json != snapshot_json
                || stored.snapshot_digest != snapshot.snapshot_digest
            {
                bail!("相同算力价格快照 ID 不能绑定不同锁价合同");
            }
            tx.commit()?;
            return Ok(ComputePriceSnapshotRegistrationReceipt {
                snapshot: stored_snapshot,
                replayed: true,
            });
        }
        if let Some(existing_snapshot_id) = snapshot_id_for_quote_on(&tx, &snapshot.quote_id)? {
            bail!("报价 ID 已绑定价格快照 {existing_snapshot_id}");
        }

        let offer = current_registered_offer_on(&tx, snapshot.offer_id.trim())?
            .ok_or_else(|| anyhow!("算力价格快照 Offer 不存在"))?;
        validate_price_snapshot_contract(snapshot, &offer.offer)?;
        let expires_at = DateTime::parse_from_rfc3339(&snapshot.expires_at)
            .context("算力价格快照失效时间不是 RFC3339")?;
        let quoted_at = DateTime::parse_from_rfc3339(&snapshot.quoted_at)
            .context("算力价格快照报价时间不是 RFC3339")?;
        if quoted_at > chrono::Utc::now() {
            bail!("不能登记报价时间位于未来的算力价格快照");
        }
        if expires_at <= chrono::Utc::now() {
            bail!("不能登记已经失效的算力价格快照");
        }

        tx.execute(
            "INSERT INTO compute_price_snapshots (
                snapshot_id, snapshot_digest, quote_id, pricing_mode,
                sku_id, sku_digest, provider_id, offer_id, offer_version,
                offer_digest, delivery_window_id, delivery_window_digest,
                currency, consumer_max_amount_micros, provider_max_amount_micros,
                price_source_kind, price_source_id, price_source_version,
                price_source_digest, trade_id, instrument_id, quoted_at,
                expires_at, snapshot_json, created_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19,
                ?20, ?21, ?22, ?23, ?24, ?25
             )",
            params![
                snapshot.snapshot_id,
                snapshot.snapshot_digest,
                snapshot.quote_id,
                snapshot.pricing_mode,
                snapshot.sku.sku_id,
                snapshot.sku.sku_digest,
                snapshot.provider_id,
                snapshot.offer_id,
                snapshot.offer_version,
                snapshot.offer_digest,
                snapshot.delivery_window.binding.window_id,
                snapshot.delivery_window.binding.window_digest,
                snapshot.currency,
                snapshot.consumer_max_amount_micros,
                snapshot.provider_max_amount_micros,
                snapshot.price_source.source_kind,
                snapshot.price_source.source_id,
                snapshot.price_source.source_version,
                snapshot.price_source.source_digest,
                snapshot.trade_id,
                snapshot.instrument_id,
                snapshot.quoted_at,
                snapshot.expires_at,
                snapshot_json,
                now(),
            ],
        )?;
        tx.commit()?;
        Ok(ComputePriceSnapshotRegistrationReceipt {
            snapshot: snapshot.clone(),
            replayed: false,
        })
    }

    pub(crate) fn compute_price_snapshot(
        &self,
        snapshot_id: &str,
    ) -> Result<ComputePriceSnapshotRegistrationReceipt> {
        if snapshot_id.trim().is_empty() {
            bail!("算力价格快照 ID 不能为空");
        }
        let conn = self.conn()?;
        let snapshot = registered_price_snapshot_on(&conn, snapshot_id.trim())?
            .ok_or_else(|| anyhow!("算力价格快照不存在"))?;
        Ok(ComputePriceSnapshotRegistrationReceipt {
            snapshot,
            replayed: false,
        })
    }

    pub(crate) fn compute_price_snapshot_if_exists(
        &self,
        snapshot_id: &str,
    ) -> Result<Option<ComputePriceSnapshotRegistrationReceipt>> {
        if snapshot_id.trim().is_empty() {
            bail!("算力价格快照 ID 不能为空");
        }
        let conn = self.conn()?;
        registered_price_snapshot_on(&conn, snapshot_id.trim())?
            .map(|snapshot| {
                Ok(ComputePriceSnapshotRegistrationReceipt {
                    snapshot,
                    replayed: false,
                })
            })
            .transpose()
    }
}

pub(super) fn registered_price_snapshot_on(
    conn: &Connection,
    snapshot_id: &str,
) -> Result<Option<ComputePriceSnapshot>> {
    let Some(stored) = price_snapshot_on(conn, snapshot_id.trim())? else {
        return Ok(None);
    };
    audited_price_snapshot_on(conn, &stored).map(Some)
}

fn audited_price_snapshot_on(
    conn: &Connection,
    stored: &StoredPriceSnapshot,
) -> Result<ComputePriceSnapshot> {
    let offer = registered_offer_version_on(conn, &stored.offer_id, stored.offer_version)?
        .ok_or_else(|| anyhow!("算力价格快照绑定的 Offer 历史版本不存在"))?;
    let snapshot: ComputePriceSnapshot =
        serde_json::from_str(&stored.snapshot_json).context("算力价格快照历史 JSON 无效")?;
    let computed_digest = validate_price_snapshot_contract(&snapshot, &offer.offer)?;
    if computed_digest != stored.snapshot_digest
        || snapshot.snapshot_id != stored.snapshot_id
        || snapshot.quote_id != stored.quote_id
        || snapshot.pricing_mode != stored.pricing_mode
        || snapshot.sku.sku_id != stored.sku_id
        || snapshot.sku.sku_digest != stored.sku_digest
        || snapshot.provider_id != stored.provider_id
        || snapshot.offer_id != stored.offer_id
        || snapshot.offer_version != stored.offer_version
        || snapshot.offer_digest != stored.offer_digest
        || snapshot.delivery_window.binding.window_id != stored.delivery_window_id
        || snapshot.delivery_window.binding.window_digest != stored.delivery_window_digest
        || snapshot.currency != stored.currency
        || snapshot.consumer_max_amount_micros != stored.consumer_max_amount_micros
        || snapshot.provider_max_amount_micros != stored.provider_max_amount_micros
        || snapshot.price_source.source_kind != stored.price_source_kind
        || snapshot.price_source.source_id != stored.price_source_id
        || snapshot.price_source.source_version != stored.price_source_version
        || snapshot.price_source.source_digest != stored.price_source_digest
        || snapshot.trade_id != stored.trade_id
        || snapshot.instrument_id != stored.instrument_id
        || snapshot.quoted_at != stored.quoted_at
        || snapshot.expires_at != stored.expires_at
    {
        bail!("算力价格快照摘要、身份或索引字段审计失败");
    }
    Ok(snapshot)
}

fn snapshot_id_for_quote_on(conn: &Connection, quote_id: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT snapshot_id FROM compute_price_snapshots WHERE quote_id=?1",
        params![quote_id.trim()],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn price_snapshot_on(conn: &Connection, snapshot_id: &str) -> Result<Option<StoredPriceSnapshot>> {
    conn.query_row(
        "SELECT snapshot_id, snapshot_digest, quote_id, pricing_mode,
                sku_id, sku_digest, provider_id, offer_id, offer_version,
                offer_digest, delivery_window_id, delivery_window_digest,
                currency, consumer_max_amount_micros, provider_max_amount_micros,
                price_source_kind, price_source_id, price_source_version,
                price_source_digest, trade_id, instrument_id, quoted_at,
                expires_at, snapshot_json
           FROM compute_price_snapshots WHERE snapshot_id=?1",
        params![snapshot_id],
        |row| {
            Ok(StoredPriceSnapshot {
                snapshot_id: row.get(0)?,
                snapshot_digest: row.get(1)?,
                quote_id: row.get(2)?,
                pricing_mode: row.get(3)?,
                sku_id: row.get(4)?,
                sku_digest: row.get(5)?,
                provider_id: row.get(6)?,
                offer_id: row.get(7)?,
                offer_version: row.get(8)?,
                offer_digest: row.get(9)?,
                delivery_window_id: row.get(10)?,
                delivery_window_digest: row.get(11)?,
                currency: row.get(12)?,
                consumer_max_amount_micros: row.get(13)?,
                provider_max_amount_micros: row.get(14)?,
                price_source_kind: row.get(15)?,
                price_source_id: row.get(16)?,
                price_source_version: row.get(17)?,
                price_source_digest: row.get(18)?,
                trade_id: row.get(19)?,
                instrument_id: row.get(20)?,
                quoted_at: row.get(21)?,
                expires_at: row.get(22)?,
                snapshot_json: row.get(23)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}
