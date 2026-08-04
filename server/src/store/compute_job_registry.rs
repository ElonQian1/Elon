use anyhow::{anyhow, bail, Context, Result};
use chrono::DateTime;
use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::{
    execution::{ComputeJob, JOB_STATUS_QUOTED},
    market::ComputePriceSnapshot,
    offer::OFFER_STATUS_ACTIVE,
    provider::PROVIDER_STATUS_ACTIVE,
};

use super::{
    compute_job_contract_validation::validate_job_contract,
    compute_offer_registry::{
        current_registered_offer_on, registered_offer_version_on, ComputeOfferRegistrationReceipt,
    },
    compute_price_snapshot_registry::registered_price_snapshot_on,
    compute_provider_registry::{
        current_registered_provider_on, registered_provider_version_on,
        ComputeProviderRegistrationReceipt,
    },
    now, Store,
};

mod queries;
mod rows;
mod transitions;

use queries::list_current_jobs_on;
use rows::{
    current_job_projection_on, job_id_for_idempotency_on, job_version_on, CurrentJobProjection,
    StoredJobVersion,
};
use transitions::{ensure_job_update, ensure_new_job, selected_contract_changed};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeJobRegistrationReceipt {
    pub job: ComputeJob,
    pub revision: i64,
    pub job_digest: String,
    pub replayed: bool,
}

struct RegisteredJobSelection {
    offer: ComputeOfferRegistrationReceipt,
    snapshot: ComputePriceSnapshot,
    provider: ComputeProviderRegistrationReceipt,
}

impl Store {
    pub(crate) fn register_compute_job(
        &self,
        job: &ComputeJob,
        expected_revision: i64,
    ) -> Result<ComputeJobRegistrationReceipt> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let receipt = register_compute_job_on(&tx, job, expected_revision)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_job(&self, job_id: &str) -> Result<ComputeJobRegistrationReceipt> {
        if job_id.trim().is_empty() {
            bail!("算力 Job ID 不能为空");
        }
        let conn = self.conn()?;
        current_registered_job_on(&conn, job_id.trim())?.ok_or_else(|| anyhow!("算力 Job 不存在"))
    }

    pub(crate) fn compute_job_version(
        &self,
        job_id: &str,
        revision: i64,
    ) -> Result<ComputeJobRegistrationReceipt> {
        if job_id.trim().is_empty() || revision <= 0 {
            bail!("算力 Job ID 或历史版本无效");
        }
        let conn = self.conn()?;
        registered_job_version_on(&conn, job_id.trim(), revision)?
            .ok_or_else(|| anyhow!("算力 Job 历史版本不存在"))
    }

    pub(crate) fn compute_job_for_consumer_idempotency(
        &self,
        consumer_account_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<ComputeJobRegistrationReceipt>> {
        if consumer_account_id.trim().is_empty() || idempotency_key.trim().is_empty() {
            bail!("消费者账户 ID 和幂等键不能为空");
        }
        let conn = self.conn()?;
        let Some(job_id) =
            job_id_for_idempotency_on(&conn, consumer_account_id.trim(), idempotency_key.trim())?
        else {
            return Ok(None);
        };
        current_registered_job_on(&conn, &job_id)
    }

    pub(crate) fn list_compute_jobs_for_consumer(
        &self,
        consumer_account_id: &str,
        project_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ComputeJobRegistrationReceipt>> {
        if consumer_account_id.trim().is_empty() {
            bail!("消费者账户 ID 不能为空");
        }
        let conn = self.conn()?;
        list_current_jobs_on(
            &conn,
            consumer_account_id.trim(),
            project_id.map(str::trim),
            limit.clamp(1, 100),
        )
    }
}

pub(super) fn register_compute_job_on(
    conn: &Connection,
    job: &ComputeJob,
    expected_revision: i64,
) -> Result<ComputeJobRegistrationReceipt> {
    if job.job_id.trim().is_empty() || job.idempotency_key.trim().is_empty() {
        bail!("算力 Job ID 和幂等键不能为空");
    }
    if expected_revision < 0 {
        bail!("算力 Job expected_revision 不能为负数");
    }
    let job_json = serde_json::to_string(job)?;

    if let Some(current) = current_job_projection_on(conn, job.job_id.trim())? {
        let stored = job_version_on(conn, &current.job_id, current.current_revision)?
            .ok_or_else(|| anyhow!("算力 Job 当前历史版本缺失，拒绝继续写入"))?;
        let current_job = audited_job_on(conn, Some(&current), &stored)?;
        if stored.job_json == job_json {
            return Ok(ComputeJobRegistrationReceipt {
                job: current_job,
                revision: stored.revision,
                job_digest: stored.job_digest,
                replayed: true,
            });
        }
        if expected_revision != current.current_revision {
            bail!(
                "算力 Job expected_revision 与当前版本不一致，当前版本为 {}",
                current.current_revision
            );
        }
        ensure_job_update(&current_job, job)?;
        let selection = registered_selection_on(conn, job)?;
        let job_digest = validate_with_selection(job, selection.as_ref())?;
        if job.status == JOB_STATUS_QUOTED && selected_contract_changed(&current_job, job) {
            ensure_live_selection_on(conn, selection.as_ref())?;
        }
        let next_revision = current
            .current_revision
            .checked_add(1)
            .ok_or_else(|| anyhow!("算力 Job 版本溢出"))?;
        insert_job_version(conn, job, next_revision, &job_digest, &job_json)?;
        let selected = job.selected_offer.as_ref();
        let updated = conn.execute(
            "UPDATE compute_jobs
                SET current_revision=?1, current_job_digest=?2, status=?3,
                    selected_provider_id=?4, selected_offer_id=?5,
                    selected_offer_version=?6, selected_offer_digest=?7,
                    price_snapshot_id=?8, updated_at=?9, recorded_at=?10
              WHERE job_id=?11 AND current_revision=?12
                AND current_job_digest=?13",
            params![
                next_revision,
                job_digest,
                job.status,
                selected.map(|value| value.provider_id.as_str()),
                selected.map(|value| value.offer_id.as_str()),
                selected.map(|value| value.offer_version),
                selected.map(|value| value.offer_digest.as_str()),
                job.price_snapshot_id,
                job.updated_at,
                now(),
                job.job_id,
                current.current_revision,
                current.current_job_digest,
            ],
        )?;
        if updated != 1 {
            bail!("算力 Job 当前投影已变化，请基于最新版本重试");
        }
        return Ok(ComputeJobRegistrationReceipt {
            job: job.clone(),
            revision: next_revision,
            job_digest,
            replayed: false,
        });
    }

    if let Some(existing_job_id) = job_id_for_idempotency_on(
        conn,
        job.consumer_account_id.trim(),
        job.idempotency_key.trim(),
    )? {
        bail!("消费者幂等键已绑定算力 Job {existing_job_id}");
    }
    ensure_new_job(job, expected_revision)?;
    let selection = registered_selection_on(conn, job)?;
    let job_digest = validate_with_selection(job, selection.as_ref())?;
    let selected = job.selected_offer.as_ref();
    conn.execute(
        "INSERT INTO compute_jobs (
            job_id, consumer_account_id, project_id, merchant_id,
            idempotency_key, current_revision, current_job_digest, status,
            selected_provider_id, selected_offer_id, selected_offer_version,
            selected_offer_digest, price_snapshot_id,
            max_consumer_charge_micros, currency, submitted_at,
            updated_at, recorded_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?8, ?9,
            ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
         )",
        params![
            job.job_id,
            job.consumer_account_id,
            job.project_id,
            job.merchant_id,
            job.idempotency_key,
            job_digest,
            job.status,
            selected.map(|value| value.provider_id.as_str()),
            selected.map(|value| value.offer_id.as_str()),
            selected.map(|value| value.offer_version),
            selected.map(|value| value.offer_digest.as_str()),
            job.price_snapshot_id,
            job.max_consumer_charge_micros,
            job.currency,
            job.submitted_at,
            job.updated_at,
            now(),
        ],
    )?;
    insert_job_version(conn, job, 1, &job_digest, &job_json)?;
    Ok(ComputeJobRegistrationReceipt {
        job: job.clone(),
        revision: 1,
        job_digest,
        replayed: false,
    })
}

pub(super) fn current_registered_job_on(
    conn: &Connection,
    job_id: &str,
) -> Result<Option<ComputeJobRegistrationReceipt>> {
    let Some(projection) = current_job_projection_on(conn, job_id)? else {
        return Ok(None);
    };
    let stored = job_version_on(conn, job_id, projection.current_revision)?
        .ok_or_else(|| anyhow!("算力 Job 当前历史版本缺失"))?;
    let job = audited_job_on(conn, Some(&projection), &stored)?;
    Ok(Some(ComputeJobRegistrationReceipt {
        job,
        revision: stored.revision,
        job_digest: stored.job_digest,
        replayed: false,
    }))
}

pub(super) fn registered_job_version_on(
    conn: &Connection,
    job_id: &str,
    revision: i64,
) -> Result<Option<ComputeJobRegistrationReceipt>> {
    let Some(stored) = job_version_on(conn, job_id, revision)? else {
        return Ok(None);
    };
    let job = audited_job_on(conn, None, &stored)?;
    Ok(Some(ComputeJobRegistrationReceipt {
        job,
        revision: stored.revision,
        job_digest: stored.job_digest,
        replayed: false,
    }))
}

fn registered_selection_on(
    conn: &Connection,
    job: &ComputeJob,
) -> Result<Option<RegisteredJobSelection>> {
    let (Some(selected), Some(snapshot_id)) = (
        job.selected_offer.as_ref(),
        job.price_snapshot_id.as_deref(),
    ) else {
        if job.selected_offer.is_some() || job.price_snapshot_id.is_some() {
            bail!("算力 Job 必须同时绑定 Offer 与 Price Snapshot");
        }
        return Ok(None);
    };
    let offer = registered_offer_version_on(conn, &selected.offer_id, selected.offer_version)?
        .ok_or_else(|| anyhow!("算力 Job 绑定的 Offer 历史版本不存在"))?;
    if offer.offer.offer_digest != selected.offer_digest {
        bail!("算力 Job 绑定的 Offer 摘要与历史版本不一致");
    }
    let snapshot = registered_price_snapshot_on(conn, snapshot_id)?
        .ok_or_else(|| anyhow!("算力 Job 绑定的 Price Snapshot 不存在"))?;
    let provider = registered_provider_version_on(
        conn,
        &offer.offer.provider_id,
        offer.provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("算力 Job 绑定的 Provider 历史版本不存在"))?;
    if provider.provider_digest != offer.provider_digest {
        bail!("算力 Job 绑定的 Provider 摘要与 Offer 不一致");
    }
    Ok(Some(RegisteredJobSelection {
        offer,
        snapshot,
        provider,
    }))
}

fn validate_with_selection(
    job: &ComputeJob,
    selection: Option<&RegisteredJobSelection>,
) -> Result<String> {
    validate_job_contract(
        job,
        selection.map(|value| &value.offer.offer),
        selection.map(|value| &value.snapshot),
        selection.map(|value| &value.provider.provider),
    )
}

fn ensure_live_selection_on(
    conn: &Connection,
    selection: Option<&RegisteredJobSelection>,
) -> Result<()> {
    let selection = selection.ok_or_else(|| anyhow!("quoted Job 缺少锁价合同"))?;
    let current_offer = current_registered_offer_on(conn, &selection.offer.offer.offer_id)?
        .ok_or_else(|| anyhow!("quoted Job 的当前 Offer 不存在"))?;
    if current_offer.offer.offer_version != selection.offer.offer.offer_version
        || current_offer.offer.offer_digest != selection.offer.offer.offer_digest
        || current_offer.offer.status != OFFER_STATUS_ACTIVE
    {
        bail!("quoted Job 只能选择当前 active Offer 版本");
    }
    let current_provider =
        current_registered_provider_on(conn, &selection.provider.provider.provider_id)?
            .ok_or_else(|| anyhow!("quoted Job 的当前 Provider 不存在"))?;
    if current_provider.provider.status != PROVIDER_STATUS_ACTIVE {
        bail!("quoted Job 只能选择当前 active Provider");
    }
    ensure_not_expired(
        "Offer",
        &selection.offer.offer.valid_from,
        &selection.offer.offer.valid_until,
    )?;
    let snapshot_expires = DateTime::parse_from_rfc3339(&selection.snapshot.expires_at)
        .context("quoted Job 的 Price Snapshot 失效时间不是 RFC3339")?;
    if snapshot_expires <= chrono::Utc::now() {
        bail!("quoted Job 不能选择已经失效的 Price Snapshot");
    }
    Ok(())
}

fn ensure_not_expired(label: &str, starts_at: &str, ends_at: &str) -> Result<()> {
    let starts = DateTime::parse_from_rfc3339(starts_at)
        .with_context(|| format!("{label} 生效时间不是 RFC3339"))?;
    let ends = DateTime::parse_from_rfc3339(ends_at)
        .with_context(|| format!("{label} 失效时间不是 RFC3339"))?;
    let now = chrono::Utc::now();
    if starts > now || ends <= now {
        bail!("quoted Job 只能选择当前有效的 {label}");
    }
    Ok(())
}

fn insert_job_version(
    conn: &Connection,
    job: &ComputeJob,
    revision: i64,
    job_digest: &str,
    job_json: &str,
) -> Result<()> {
    let selected = job.selected_offer.as_ref();
    conn.execute(
        "INSERT INTO compute_job_versions (
            job_id, revision, job_digest, status, selected_provider_id,
            selected_offer_id, selected_offer_version, selected_offer_digest,
            price_snapshot_id, job_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            job.job_id,
            revision,
            job_digest,
            job.status,
            selected.map(|value| value.provider_id.as_str()),
            selected.map(|value| value.offer_id.as_str()),
            selected.map(|value| value.offer_version),
            selected.map(|value| value.offer_digest.as_str()),
            job.price_snapshot_id,
            job_json,
            now(),
        ],
    )?;
    Ok(())
}

fn audited_job_on(
    conn: &Connection,
    projection: Option<&CurrentJobProjection>,
    stored: &StoredJobVersion,
) -> Result<ComputeJob> {
    let job: ComputeJob =
        serde_json::from_str(&stored.job_json).context("算力 Job 历史版本 JSON 无效")?;
    let selection = registered_selection_on(conn, &job)?;
    let computed_digest = validate_with_selection(&job, selection.as_ref())?;
    let selected = job.selected_offer.as_ref();
    if computed_digest != stored.job_digest
        || job.job_id != stored.job_id
        || job.status != stored.status
        || selected.map(|value| value.provider_id.as_str())
            != stored.selected_provider_id.as_deref()
        || selected.map(|value| value.offer_id.as_str()) != stored.selected_offer_id.as_deref()
        || selected.map(|value| value.offer_version) != stored.selected_offer_version
        || selected.map(|value| value.offer_digest.as_str())
            != stored.selected_offer_digest.as_deref()
        || job.price_snapshot_id != stored.price_snapshot_id
    {
        bail!("算力 Job 历史版本身份、摘要或索引字段审计失败");
    }
    if let Some(projection) = projection {
        ensure_current_job_projection(&job, stored, projection)?;
    }
    Ok(job)
}

fn ensure_current_job_projection(
    job: &ComputeJob,
    stored: &StoredJobVersion,
    projection: &CurrentJobProjection,
) -> Result<()> {
    let selected = job.selected_offer.as_ref();
    if job.job_id != projection.job_id
        || job.consumer_account_id != projection.consumer_account_id
        || job.project_id != projection.project_id
        || job.merchant_id != projection.merchant_id
        || job.idempotency_key != projection.idempotency_key
        || stored.revision != projection.current_revision
        || stored.job_digest != projection.current_job_digest
        || job.status != projection.status
        || selected.map(|value| value.provider_id.as_str())
            != projection.selected_provider_id.as_deref()
        || selected.map(|value| value.offer_id.as_str()) != projection.selected_offer_id.as_deref()
        || selected.map(|value| value.offer_version) != projection.selected_offer_version
        || selected.map(|value| value.offer_digest.as_str())
            != projection.selected_offer_digest.as_deref()
        || job.price_snapshot_id != projection.price_snapshot_id
        || job.max_consumer_charge_micros != projection.max_consumer_charge_micros
        || job.currency != projection.currency
        || job.submitted_at != projection.submitted_at
        || job.updated_at != projection.updated_at
    {
        bail!("算力 Job 当前投影与不可变版本不一致");
    }
    Ok(())
}
