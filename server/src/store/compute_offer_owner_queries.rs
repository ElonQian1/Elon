use anyhow::{bail, Result};
use rusqlite::params;

use super::{
    compute_offer_registry::{
        current_registered_offer_on, registered_offer_version_on, ComputeOfferRegistrationReceipt,
    },
    Store,
};

impl Store {
    pub(crate) fn compute_offer_if_exists(
        &self,
        offer_id: &str,
    ) -> Result<Option<ComputeOfferRegistrationReceipt>> {
        validate_offer_id(offer_id)?;
        current_registered_offer_on(&*self.conn()?, offer_id.trim())
    }

    pub(crate) fn compute_offer_version_if_exists(
        &self,
        offer_id: &str,
        offer_version: i64,
    ) -> Result<Option<ComputeOfferRegistrationReceipt>> {
        validate_offer_id(offer_id)?;
        if offer_version <= 0 {
            bail!("算力 Offer 版本必须为正整数");
        }
        registered_offer_version_on(&*self.conn()?, offer_id.trim(), offer_version)
    }

    pub(crate) fn list_compute_offers_for_provider(
        &self,
        provider_id: &str,
        pool_id: &str,
        limit: usize,
    ) -> Result<Vec<ComputeOfferRegistrationReceipt>> {
        validate_bounded("算力 Provider ID", provider_id, 160)?;
        validate_bounded("容量池 ID", pool_id, 160)?;
        let conn = self.conn()?;
        let offer_ids = {
            let mut stmt = conn.prepare(
                "SELECT offer_id FROM compute_offers
                  WHERE provider_id=?1 AND capacity_pool_id=?2
                  ORDER BY recorded_at DESC, offer_id ASC
                  LIMIT ?3",
            )?;
            let rows = stmt.query_map(
                params![
                    provider_id.trim(),
                    pool_id.trim(),
                    limit.clamp(1, 100) as i64
                ],
                |row| row.get::<_, String>(0),
            )?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        offer_ids
            .into_iter()
            .map(|offer_id| {
                current_registered_offer_on(&conn, &offer_id)?
                    .ok_or_else(|| anyhow::anyhow!("算力 Offer 在列表读取期间消失"))
            })
            .collect()
    }

    pub(crate) fn list_compute_offer_drafts_for_review(
        &self,
        limit: usize,
    ) -> Result<Vec<ComputeOfferRegistrationReceipt>> {
        let conn = self.conn()?;
        let offer_ids = {
            let mut stmt = conn.prepare(
                "SELECT offer_id FROM compute_offers
                  WHERE status='draft'
                  ORDER BY recorded_at ASC, offer_id ASC
                  LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit.clamp(1, 100) as i64], |row| {
                row.get::<_, String>(0)
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        offer_ids
            .into_iter()
            .map(|offer_id| {
                current_registered_offer_on(&conn, &offer_id)?
                    .ok_or_else(|| anyhow::anyhow!("算力 Offer 在审核队列读取期间消失"))
            })
            .collect()
    }
}

fn validate_offer_id(offer_id: &str) -> Result<()> {
    validate_bounded("算力 Offer ID", offer_id, 200)
}

fn validate_bounded(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max_len
        || value.chars().any(char::is_control)
    {
        bail!("{label}为空、过长或包含无效字符");
    }
    Ok(())
}
