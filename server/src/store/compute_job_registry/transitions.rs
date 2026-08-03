use anyhow::{bail, Context, Result};
use chrono::DateTime;

use crate::compute_federation::execution::{
    ComputeJob, JOB_STATUS_CANCELED, JOB_STATUS_FAILED, JOB_STATUS_QUOTED, JOB_STATUS_RESERVED,
    JOB_STATUS_RUNNING, JOB_STATUS_SETTLED, JOB_STATUS_SUBMITTED, JOB_STATUS_VERIFICATION_PENDING,
};

pub(super) fn ensure_new_job(job: &ComputeJob, expected_revision: i64) -> Result<()> {
    if expected_revision != 0 {
        bail!("新算力 Job 的 expected_revision 必须为 0");
    }
    if job.status != JOB_STATUS_SUBMITTED {
        bail!("新算力 Job 必须以 submitted 状态创建");
    }
    if job.submitted_at != job.updated_at {
        bail!("新算力 Job 的提交时间与更新时间必须一致");
    }
    Ok(())
}

pub(super) fn ensure_job_update(current: &ComputeJob, next: &ComputeJob) -> Result<()> {
    ensure_stable_demand(current, next)?;
    if !job_status_transition_allowed(&current.status, &next.status) {
        bail!(
            "算力 Job 状态不允许从 {} 变更为 {}",
            current.status,
            next.status
        );
    }
    let selection_changed = selected_contract_changed(current, next);
    if current.status == JOB_STATUS_SUBMITTED && next.status != JOB_STATUS_QUOTED {
        if selection_changed {
            bail!("submitted Job 只能在进入 quoted 状态时首次选择锁价合同");
        }
    } else if current.status == JOB_STATUS_QUOTED && next.status == JOB_STATUS_QUOTED {
        if !selection_changed {
            bail!("quoted Job 仅在重新选择 Offer 或 Price Snapshot 时才能追加同状态版本");
        }
    } else if current.status != JOB_STATUS_SUBMITTED && selection_changed {
        bail!("算力 Job 离开 quoted 自刷新路径后不能改变已选锁价合同");
    }
    ensure_updated_at_monotonic(&current.updated_at, &next.updated_at)
}

pub(super) fn selected_contract_changed(current: &ComputeJob, next: &ComputeJob) -> bool {
    current.selected_offer != next.selected_offer
        || current.price_snapshot_id != next.price_snapshot_id
}

fn ensure_stable_demand(current: &ComputeJob, next: &ComputeJob) -> Result<()> {
    if current.schema != next.schema
        || current.job_id != next.job_id
        || current.project_id != next.project_id
        || current.merchant_id != next.merchant_id
        || current.consumer_account_id != next.consumer_account_id
        || current.idempotency_key != next.idempotency_key
        || current.workload != next.workload
        || current.provider_scope != next.provider_scope
        || current.max_consumer_charge_micros != next.max_consumer_charge_micros
        || current.currency != next.currency
        || current.submitted_at != next.submitted_at
    {
        bail!("算力 Job 的需求、归属、幂等键、预算和提交时间不能原地改变");
    }
    Ok(())
}

fn job_status_transition_allowed(current: &str, next: &str) -> bool {
    match current {
        JOB_STATUS_SUBMITTED => matches!(
            next,
            JOB_STATUS_QUOTED | JOB_STATUS_FAILED | JOB_STATUS_CANCELED
        ),
        JOB_STATUS_QUOTED => matches!(
            next,
            JOB_STATUS_QUOTED | JOB_STATUS_RESERVED | JOB_STATUS_FAILED | JOB_STATUS_CANCELED
        ),
        JOB_STATUS_RESERVED => matches!(
            next,
            JOB_STATUS_RUNNING | JOB_STATUS_FAILED | JOB_STATUS_CANCELED
        ),
        JOB_STATUS_RUNNING => matches!(
            next,
            JOB_STATUS_VERIFICATION_PENDING | JOB_STATUS_FAILED | JOB_STATUS_CANCELED
        ),
        JOB_STATUS_VERIFICATION_PENDING => {
            matches!(next, JOB_STATUS_SETTLED | JOB_STATUS_FAILED)
        }
        JOB_STATUS_SETTLED | JOB_STATUS_FAILED | JOB_STATUS_CANCELED => false,
        _ => false,
    }
}

fn ensure_updated_at_monotonic(previous: &str, next: &str) -> Result<()> {
    let previous =
        DateTime::parse_from_rfc3339(previous).context("算力 Job 当前更新时间不是 RFC3339")?;
    let next = DateTime::parse_from_rfc3339(next).context("算力 Job 新更新时间不是 RFC3339")?;
    if next <= previous {
        bail!("算力 Job 新更新时间必须晚于当前版本");
    }
    Ok(())
}
