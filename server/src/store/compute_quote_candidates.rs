use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::compute_federation::{
    execution::{ComputeOfferBinding, JOB_STATUS_QUOTED, JOB_STATUS_SUBMITTED},
    market::ComputePriceSnapshot,
    offer::OFFER_STATUS_ACTIVE,
    provider::PROVIDER_STATUS_ACTIVE,
};

use super::{
    compute_job_contract_validation::validate_job_contract,
    compute_job_registry::ComputeJobRegistrationReceipt,
    compute_offer_registry::current_registered_offer_on,
    compute_price_snapshot_registry::registered_price_snapshot_on,
    compute_provider_registry::{current_registered_provider_on, registered_provider_version_on},
    Store,
};

const MAX_CANDIDATE_SCAN: usize = 1_000;
const CANDIDATE_BATCH_SIZE: usize = 64;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeQuoteCandidateProvider {
    pub provider_id: String,
    pub provider_kind: String,
    pub display_name: String,
    pub trust_tier: String,
    pub home_region: Option<String>,
    pub policy_revision: i64,
    pub provider_digest: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeJobQuoteCandidate {
    pub offer: ComputeOfferBinding,
    pub price_snapshot: ComputePriceSnapshot,
    pub provider: ComputeQuoteCandidateProvider,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeJobQuoteCandidatePage {
    pub job_id: String,
    pub job_revision: i64,
    pub job_digest: String,
    pub candidates: Vec<ComputeJobQuoteCandidate>,
    pub scanned_count: usize,
    pub scan_truncated: bool,
}

impl Store {
    pub(crate) fn list_compute_job_quote_candidates(
        &self,
        job: &ComputeJobRegistrationReceipt,
        limit: usize,
    ) -> Result<ComputeJobQuoteCandidatePage> {
        if !matches!(
            job.job.status.as_str(),
            JOB_STATUS_SUBMITTED | JOB_STATUS_QUOTED
        ) {
            bail!("只有 submitted 或 quoted Job 可以发现新的锁价候选");
        }
        let limit = limit.clamp(1, 100);
        let conn = self.conn()?;
        let now = Utc::now();
        let deadline = DateTime::parse_from_rfc3339(&job.job.workload.deadline_at)
            .context("算力 Job 截止时间不是 RFC3339")?
            .with_timezone(&Utc);
        if deadline <= now {
            bail!("已经超过截止时间的 Job 不能发现锁价候选");
        }
        let now_text = now.to_rfc3339();
        let mut candidates = Vec::new();
        let mut offset = 0usize;
        let mut scanned_count = 0usize;
        let mut source_exhausted = false;

        while candidates.len() < limit && offset < MAX_CANDIDATE_SCAN {
            let batch_size = CANDIDATE_BATCH_SIZE.min(MAX_CANDIDATE_SCAN - offset);
            let snapshot_ids = candidate_snapshot_ids_on(
                &conn,
                &job.job.currency,
                job.job.max_consumer_charge_micros,
                &now_text,
                batch_size,
                offset,
            )?;
            source_exhausted = snapshot_ids.len() < batch_size;
            offset += snapshot_ids.len();
            for snapshot_id in snapshot_ids {
                scanned_count += 1;
                if let Some(candidate) = quote_candidate_on(&conn, job, &snapshot_id, &now)? {
                    candidates.push(candidate);
                    if candidates.len() == limit {
                        break;
                    }
                }
            }
            if source_exhausted {
                break;
            }
        }

        Ok(ComputeJobQuoteCandidatePage {
            job_id: job.job.job_id.clone(),
            job_revision: job.revision,
            job_digest: job.job_digest.clone(),
            scan_truncated: candidates.len() == limit
                || (!source_exhausted && offset >= MAX_CANDIDATE_SCAN),
            candidates,
            scanned_count,
        })
    }
}

fn candidate_snapshot_ids_on(
    conn: &Connection,
    currency: &str,
    max_consumer_charge_micros: i64,
    now: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT snapshot.snapshot_id
           FROM compute_price_snapshots AS snapshot
           JOIN compute_offers AS offer
             ON offer.offer_id=snapshot.offer_id
            AND offer.current_offer_version=snapshot.offer_version
            AND offer.current_offer_digest=snapshot.offer_digest
           JOIN compute_providers AS provider
             ON provider.provider_id=snapshot.provider_id
          WHERE snapshot.currency=?1
            AND snapshot.consumer_max_amount_micros<=?2
            AND offer.status='active'
            AND provider.status='active'
            AND julianday(snapshot.expires_at)>julianday(?3)
            AND julianday(offer.valid_from)<=julianday(?3)
            AND julianday(offer.valid_until)>julianday(?3)
          ORDER BY snapshot.consumer_max_amount_micros ASC,
                   snapshot.quoted_at DESC,
                   snapshot.snapshot_id ASC
          LIMIT ?4 OFFSET ?5",
    )?;
    let rows = stmt.query_map(
        params![
            currency,
            max_consumer_charge_micros,
            now,
            limit as i64,
            offset as i64,
        ],
        |row| row.get(0),
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn quote_candidate_on(
    conn: &Connection,
    job: &ComputeJobRegistrationReceipt,
    snapshot_id: &str,
    now: &DateTime<Utc>,
) -> Result<Option<ComputeJobQuoteCandidate>> {
    let snapshot = registered_price_snapshot_on(conn, snapshot_id)?
        .ok_or_else(|| anyhow!("候选 Price Snapshot 在读取期间消失"))?;
    let offer = current_registered_offer_on(conn, &snapshot.offer_id)?
        .ok_or_else(|| anyhow!("候选 Price Snapshot 的当前 Offer 不存在"))?;
    if offer.offer.offer_version != snapshot.offer_version
        || offer.offer.offer_digest != snapshot.offer_digest
        || offer.offer.status != OFFER_STATUS_ACTIVE
    {
        bail!("候选 Price Snapshot 与当前 Offer 投影不一致");
    }
    let current_provider = current_registered_provider_on(conn, &snapshot.provider_id)?
        .ok_or_else(|| anyhow!("候选 Price Snapshot 的当前 Provider 不存在"))?;
    if current_provider.provider.status != PROVIDER_STATUS_ACTIVE {
        bail!("候选 Price Snapshot 的当前 Provider 不是 active");
    }
    let provider = registered_provider_version_on(
        conn,
        &offer.offer.provider_id,
        offer.provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("候选 Offer 的 Provider 历史版本不存在"))?;
    if provider.provider_digest != offer.provider_digest {
        bail!("候选 Offer 的 Provider 历史摘要不一致");
    }
    if !window_is_live(&offer.offer.valid_from, &offer.offer.valid_until, now)?
        || !expiry_is_future(&snapshot.expires_at, now)?
    {
        return Ok(None);
    }

    let binding = ComputeOfferBinding {
        provider_id: offer.offer.provider_id.clone(),
        offer_id: offer.offer.offer_id.clone(),
        offer_version: offer.offer.offer_version,
        offer_digest: offer.offer.offer_digest.clone(),
    };
    let mut selected_job = job.job.clone();
    selected_job.status = JOB_STATUS_QUOTED.to_string();
    selected_job.selected_offer = Some(binding.clone());
    selected_job.price_snapshot_id = Some(snapshot.snapshot_id.clone());
    if validate_job_contract(
        &selected_job,
        Some(&offer.offer),
        Some(&snapshot),
        Some(&provider.provider),
    )
    .is_err()
    {
        return Ok(None);
    }

    Ok(Some(ComputeJobQuoteCandidate {
        offer: binding,
        price_snapshot: snapshot,
        provider: ComputeQuoteCandidateProvider {
            provider_id: provider.provider.provider_id,
            provider_kind: provider.provider.provider_kind,
            display_name: provider.provider.display_name,
            trust_tier: provider.provider.trust_tier,
            home_region: provider.provider.home_region,
            policy_revision: provider.provider.policy_revision,
            provider_digest: provider.provider_digest,
        },
    }))
}

fn window_is_live(start: &str, end: &str, now: &DateTime<Utc>) -> Result<bool> {
    let start = DateTime::parse_from_rfc3339(start)
        .context("候选 Offer 生效时间不是 RFC3339")?
        .with_timezone(&Utc);
    let end = DateTime::parse_from_rfc3339(end)
        .context("候选 Offer 失效时间不是 RFC3339")?
        .with_timezone(&Utc);
    Ok(&start <= now && &end > now)
}

fn expiry_is_future(expiry: &str, now: &DateTime<Utc>) -> Result<bool> {
    let expiry = DateTime::parse_from_rfc3339(expiry)
        .context("候选 Price Snapshot 失效时间不是 RFC3339")?
        .with_timezone(&Utc);
    Ok(&expiry > now)
}
