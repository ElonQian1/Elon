use anyhow::{bail, Context, Result};
use rusqlite::Connection;

use crate::compute_federation::execution::ComputeJob;

use super::{
    registered_selection_with_dependency_policy_on,
    rows::{CurrentJobProjection, StoredJobVersion},
    validate_with_selection,
};

pub(super) fn audited_job_on(
    conn: &Connection,
    projection: Option<&CurrentJobProjection>,
    stored: &StoredJobVersion,
) -> Result<ComputeJob> {
    audited_job_with_dependency_policy_on(conn, projection, stored, false)
}

pub(super) fn audited_historical_job_on(
    conn: &Connection,
    stored: &StoredJobVersion,
) -> Result<ComputeJob> {
    audited_job_with_dependency_policy_on(conn, None, stored, true)
}

fn audited_job_with_dependency_policy_on(
    conn: &Connection,
    projection: Option<&CurrentJobProjection>,
    stored: &StoredJobVersion,
    use_historical_dependencies: bool,
) -> Result<ComputeJob> {
    let job: ComputeJob =
        serde_json::from_str(&stored.job_json).context("算力 Job 历史版本 JSON 无效")?;
    let selection =
        registered_selection_with_dependency_policy_on(conn, &job, use_historical_dependencies)?;
    let computed_digest = validate_with_selection(&job, selection.as_ref())?;
    let selected = job.selected_offer.as_ref();
    if stored.job_json != serde_json::to_string(&job)?
        || computed_digest != stored.job_digest
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
