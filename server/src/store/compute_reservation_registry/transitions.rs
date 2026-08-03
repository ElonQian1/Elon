use anyhow::{bail, Context, Result};
use chrono::DateTime;

use crate::compute_federation::execution::{
    ComputeReservation, RESERVATION_STATUS_ACTIVE, RESERVATION_STATUS_CONSUMED,
    RESERVATION_STATUS_EXPIRED, RESERVATION_STATUS_PENDING, RESERVATION_STATUS_RELEASED,
};

pub(super) fn ensure_new_reservation(
    reservation: &ComputeReservation,
    expected_revision: i64,
) -> Result<()> {
    if expected_revision != 0 {
        bail!("新算力 Reservation 的 expected_revision 必须为 0");
    }
    if reservation.status != RESERVATION_STATUS_PENDING {
        bail!("新算力 Reservation 必须以 pending 状态创建");
    }
    if reservation.created_at != reservation.updated_at
        || reservation.consumed_at.is_some()
        || reservation.released_at.is_some()
    {
        bail!("新算力 Reservation 的创建时间、更新时间或终态时间无效");
    }
    Ok(())
}

pub(super) fn ensure_reservation_update(
    current: &ComputeReservation,
    next: &ComputeReservation,
) -> Result<()> {
    ensure_stable_contract(current, next)?;
    if !reservation_status_transition_allowed(&current.status, &next.status) {
        bail!(
            "算力 Reservation 状态不允许从 {} 变更为 {}",
            current.status,
            next.status
        );
    }
    if next.job.job_revision < current.job.job_revision
        || next.capacity_claim.claim_revision < current.capacity_claim.claim_revision
    {
        bail!("算力 Reservation 不能回退 Job 或 Capacity Claim 版本");
    }
    if next.job == current.job && next.capacity_claim == current.capacity_claim {
        bail!("算力 Reservation 新版本必须推进 Job 或 Capacity Claim 绑定");
    }
    ensure_updated_at_monotonic(&current.updated_at, &next.updated_at)
}

fn ensure_stable_contract(current: &ComputeReservation, next: &ComputeReservation) -> Result<()> {
    if current.schema != next.schema
        || current.reservation_id != next.reservation_id
        || current.job.job_id != next.job.job_id
        || current.idempotency_key != next.idempotency_key
        || current.offer != next.offer
        || current.price_snapshot != next.price_snapshot
        || current.capacity_claim.claim_id != next.capacity_claim.claim_id
        || current.reserved_capacity != next.reserved_capacity
        || current.consumer_authorization_ref != next.consumer_authorization_ref
        || current.created_at != next.created_at
        || current.expires_at != next.expires_at
    {
        bail!("算力 Reservation 的身份、锁价、容量、授权和有效期不能原地改变");
    }
    Ok(())
}

fn reservation_status_transition_allowed(current: &str, next: &str) -> bool {
    match current {
        RESERVATION_STATUS_PENDING => matches!(
            next,
            RESERVATION_STATUS_ACTIVE | RESERVATION_STATUS_RELEASED | RESERVATION_STATUS_EXPIRED
        ),
        RESERVATION_STATUS_ACTIVE => matches!(
            next,
            RESERVATION_STATUS_ACTIVE
                | RESERVATION_STATUS_CONSUMED
                | RESERVATION_STATUS_RELEASED
                | RESERVATION_STATUS_EXPIRED
        ),
        RESERVATION_STATUS_CONSUMED | RESERVATION_STATUS_RELEASED | RESERVATION_STATUS_EXPIRED => {
            false
        }
        _ => false,
    }
}

fn ensure_updated_at_monotonic(previous: &str, next: &str) -> Result<()> {
    let previous = DateTime::parse_from_rfc3339(previous)
        .context("算力 Reservation 当前更新时间不是 RFC3339")?;
    let next =
        DateTime::parse_from_rfc3339(next).context("算力 Reservation 新更新时间不是 RFC3339")?;
    if next <= previous {
        bail!("算力 Reservation 新更新时间必须晚于当前版本");
    }
    Ok(())
}
