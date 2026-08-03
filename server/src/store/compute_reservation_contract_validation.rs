use anyhow::{bail, Context, Result};
use chrono::{DateTime, FixedOffset};
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    capacity::{
        validate_capacity_claim, ComputeCapacityClaim, ComputeCapacityClaimKind,
        ComputeCapacityClaimState, COMPUTE_CAPACITY_CLAIM_SCHEMA,
    },
    execution::{
        ComputeJob, ComputeReservation, COMPUTE_RESERVATION_SCHEMA, JOB_STATUS_CANCELED,
        JOB_STATUS_FAILED, JOB_STATUS_QUOTED, JOB_STATUS_RESERVED, JOB_STATUS_RUNNING,
        JOB_STATUS_SETTLED, JOB_STATUS_VERIFICATION_PENDING, RESERVATION_STATUS_ACTIVE,
        RESERVATION_STATUS_CONSUMED, RESERVATION_STATUS_EXPIRED, RESERVATION_STATUS_PENDING,
        RESERVATION_STATUS_RELEASED,
    },
    market::ComputePriceSnapshot,
    offer::ComputeOffer,
    provider::ComputeProvider,
};

use super::{
    compute_capacity_claim_rows::finalize_claim_digest,
    compute_job_contract_validation::validate_job_contract,
};

mod capacity;

use self::capacity::validate_reserved_capacity;

const RESERVATION_CLAIM_SUBJECT_KIND: &str = "compute_reservation";

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_reservation_contract(
    reservation: &ComputeReservation,
    job: &ComputeJob,
    job_revision: i64,
    job_digest: &str,
    offer: &ComputeOffer,
    snapshot: &ComputePriceSnapshot,
    provider: &ComputeProvider,
    claim: &ComputeCapacityClaim,
) -> Result<String> {
    validate_reservation_identity(reservation)?;
    validate_claim_contract(claim)?;
    let computed_job_digest =
        validate_job_contract(job, Some(offer), Some(snapshot), Some(provider))?;
    if computed_job_digest != job_digest {
        bail!("Reservation 绑定的 Job 摘要无法通过重新计算审计");
    }
    validate_bindings(
        reservation,
        job,
        job_revision,
        job_digest,
        offer,
        snapshot,
        claim,
    )?;
    validate_reserved_capacity(
        reservation,
        &job.workload.usage_limits,
        offer,
        snapshot,
        claim,
    )?;
    validate_reservation_times(reservation, job, snapshot, claim)?;
    validate_state_alignment(reservation, job, claim)?;
    compute_reservation_digest(reservation)
}

pub(super) fn compute_reservation_digest(reservation: &ComputeReservation) -> Result<String> {
    let encoded = serde_json::to_vec(reservation)?;
    Ok(hex::encode(Sha256::digest(encoded)))
}

fn validate_reservation_identity(reservation: &ComputeReservation) -> Result<()> {
    if reservation.schema != COMPUTE_RESERVATION_SCHEMA {
        bail!("算力 Reservation schema 不受支持");
    }
    for (label, value) in [
        ("Reservation ID", reservation.reservation_id.as_str()),
        ("Reservation 幂等键", reservation.idempotency_key.as_str()),
        ("Job ID", reservation.job.job_id.as_str()),
        ("Job 摘要", reservation.job.job_digest.as_str()),
        ("Offer Provider ID", reservation.offer.provider_id.as_str()),
        ("Offer ID", reservation.offer.offer_id.as_str()),
        ("Offer 摘要", reservation.offer.offer_digest.as_str()),
        (
            "Capacity Claim ID",
            reservation.capacity_claim.claim_id.as_str(),
        ),
        (
            "Capacity Claim 摘要",
            reservation.capacity_claim.claim_digest.as_str(),
        ),
        (
            "消费者授权引用",
            reservation.consumer_authorization_ref.as_str(),
        ),
    ] {
        validate_exact_value(label, value)?;
    }
    if reservation.job.job_revision <= 0
        || reservation.offer.offer_version <= 0
        || reservation.capacity_claim.claim_revision <= 0
    {
        bail!("Reservation 绑定的 Job、Offer 或 Capacity Claim 版本无效");
    }
    if !matches!(
        reservation.status.as_str(),
        RESERVATION_STATUS_PENDING
            | RESERVATION_STATUS_ACTIVE
            | RESERVATION_STATUS_CONSUMED
            | RESERVATION_STATUS_RELEASED
            | RESERVATION_STATUS_EXPIRED
    ) {
        bail!("算力 Reservation 状态不受支持");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_bindings(
    reservation: &ComputeReservation,
    job: &ComputeJob,
    job_revision: i64,
    job_digest: &str,
    offer: &ComputeOffer,
    snapshot: &ComputePriceSnapshot,
    claim: &ComputeCapacityClaim,
) -> Result<()> {
    let selected = job
        .selected_offer
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Reservation 绑定的 Job 尚未选择 Offer"))?;
    if reservation.job.job_id != job.job_id
        || reservation.job.job_revision != job_revision
        || reservation.job.job_digest != job_digest
        || &reservation.offer != selected
        || &reservation.price_snapshot != snapshot
        || job.price_snapshot_id.as_deref() != Some(snapshot.snapshot_id.as_str())
        || reservation.capacity_claim.claim_id != claim.claim_id
        || reservation.capacity_claim.claim_revision != claim.revision
        || reservation.capacity_claim.claim_digest != claim.claim_digest
        || claim.pool != offer.capacity_pool
        || claim.delivery_window != snapshot.delivery_window.binding
    {
        bail!("Reservation 与 Job、Offer、Price Snapshot 或 Capacity Claim 绑定不一致");
    }
    if claim.claim_kind != ComputeCapacityClaimKind::Reservation
        || claim.subject_kind != RESERVATION_CLAIM_SUBJECT_KIND
        || claim.subject_id != reservation.reservation_id
        || claim.idempotency_key != reservation.idempotency_key
    {
        bail!("Capacity Claim 未由当前 Reservation 以相同幂等键持有");
    }
    Ok(())
}

fn validate_claim_contract(claim: &ComputeCapacityClaim) -> Result<()> {
    if claim.schema != COMPUTE_CAPACITY_CLAIM_SCHEMA {
        bail!("Reservation Capacity Claim schema 不受支持");
    }
    for (label, value) in [
        ("Capacity Claim ID", claim.claim_id.as_str()),
        ("Capacity Claim 摘要", claim.claim_digest.as_str()),
        ("Capacity Claim 主体类型", claim.subject_kind.as_str()),
        ("Capacity Claim 主体 ID", claim.subject_id.as_str()),
        ("Capacity Claim 幂等范围", claim.idempotency_scope.as_str()),
        ("Capacity Claim 幂等键", claim.idempotency_key.as_str()),
        ("Capacity Claim 请求摘要", claim.request_digest.as_str()),
    ] {
        validate_exact_value(label, value)?;
    }
    validate_capacity_claim(claim)
        .map_err(|error| anyhow::anyhow!("Reservation Capacity Claim 无效: {error:?}"))?;
    let stored_digest = claim.claim_digest.clone();
    let mut recomputed = claim.clone();
    finalize_claim_digest(&mut recomputed)?;
    if recomputed.claim_digest != stored_digest {
        bail!("Reservation Capacity Claim 摘要审计失败");
    }
    Ok(())
}

fn validate_reservation_times(
    reservation: &ComputeReservation,
    job: &ComputeJob,
    snapshot: &ComputePriceSnapshot,
    claim: &ComputeCapacityClaim,
) -> Result<()> {
    let created = parse_utc("Reservation 创建时间", &reservation.created_at)?;
    let updated = parse_utc("Reservation 更新时间", &reservation.updated_at)?;
    let expires = parse_utc("Reservation 到期时间", &reservation.expires_at)?;
    let submitted = parse_utc("Job 提交时间", &job.submitted_at)?;
    let job_updated = parse_utc("Job 绑定版本更新时间", &job.updated_at)?;
    let deadline = parse_utc("Job 截止时间", &job.workload.deadline_at)?;
    let quoted = parse_utc("Price Snapshot 报价时间", &snapshot.quoted_at)?;
    let snapshot_expires = parse_utc("Price Snapshot 失效时间", &snapshot.expires_at)?;
    let window_end = parse_utc(
        "Price Snapshot 窗口结束时间",
        &snapshot.delivery_window.ends_at_utc,
    )?;
    let claim_updated = parse_utc("Capacity Claim 更新时间", &claim.updated_at)?;
    let claim_update_is_misaligned = match reservation.status.as_str() {
        RESERVATION_STATUS_ACTIVE => claim_updated > updated,
        RESERVATION_STATUS_PENDING
        | RESERVATION_STATUS_CONSUMED
        | RESERVATION_STATUS_RELEASED
        | RESERVATION_STATUS_EXPIRED => claim_updated != updated,
        _ => true,
    };
    if submitted > created
        || quoted > created
        || created > updated
        || created >= expires
        || job_updated > updated
        || expires > deadline
        || expires > snapshot_expires
        || expires > window_end
        || claim.created_at != reservation.created_at
        || claim_update_is_misaligned
        || claim.expires_at.as_deref() != Some(reservation.expires_at.as_str())
    {
        bail!("Reservation、Job、Price Snapshot 与 Capacity Claim 时间边界不一致");
    }
    validate_optional_time("Reservation 消费时间", reservation.consumed_at.as_deref())?;
    validate_optional_time("Reservation 释放时间", reservation.released_at.as_deref())?;
    Ok(())
}

fn validate_state_alignment(
    reservation: &ComputeReservation,
    job: &ComputeJob,
    claim: &ComputeCapacityClaim,
) -> Result<()> {
    let no_terminal_times = reservation.consumed_at.is_none()
        && reservation.released_at.is_none()
        && claim.terminal_at.is_none();
    let aligned = match reservation.status.as_str() {
        RESERVATION_STATUS_PENDING => {
            job.status == JOB_STATUS_QUOTED
                && claim.state == ComputeCapacityClaimState::Held
                && no_terminal_times
                && time_is_before(
                    Some(reservation.updated_at.as_str()),
                    &reservation.expires_at,
                    "Reservation 更新时间",
                )?
        }
        RESERVATION_STATUS_ACTIVE => {
            matches!(
                job.status.as_str(),
                JOB_STATUS_RESERVED | JOB_STATUS_RUNNING | JOB_STATUS_VERIFICATION_PENDING
            ) && matches!(
                claim.state,
                ComputeCapacityClaimState::Held | ComputeCapacityClaimState::Active
            ) && no_terminal_times
                && parse_utc("Reservation 更新时间", &reservation.updated_at)?
                    < parse_utc("Reservation 到期时间", &reservation.expires_at)?
        }
        RESERVATION_STATUS_CONSUMED => {
            matches!(
                job.status.as_str(),
                JOB_STATUS_RUNNING | JOB_STATUS_VERIFICATION_PENDING | JOB_STATUS_SETTLED
            ) && claim.state == ComputeCapacityClaimState::Consumed
                && terminal_time_matches(
                    reservation.consumed_at.as_deref(),
                    reservation.released_at.as_deref(),
                    claim.terminal_at.as_deref(),
                    &reservation.updated_at,
                )
                && time_is_at_or_before(
                    reservation.consumed_at.as_deref(),
                    &reservation.expires_at,
                    "Reservation 消费时间",
                )?
        }
        RESERVATION_STATUS_RELEASED => {
            matches!(job.status.as_str(), JOB_STATUS_FAILED | JOB_STATUS_CANCELED)
                && matches!(
                    claim.state,
                    ComputeCapacityClaimState::Released | ComputeCapacityClaimState::Canceled
                )
                && terminal_time_matches(
                    reservation.released_at.as_deref(),
                    reservation.consumed_at.as_deref(),
                    claim.terminal_at.as_deref(),
                    &reservation.updated_at,
                )
        }
        RESERVATION_STATUS_EXPIRED => {
            matches!(job.status.as_str(), JOB_STATUS_FAILED | JOB_STATUS_CANCELED)
                && claim.state == ComputeCapacityClaimState::Expired
                && terminal_time_matches(
                    reservation.released_at.as_deref(),
                    reservation.consumed_at.as_deref(),
                    claim.terminal_at.as_deref(),
                    &reservation.updated_at,
                )
                && time_is_at_or_after(
                    reservation.released_at.as_deref(),
                    &reservation.expires_at,
                    "Reservation 到期释放时间",
                )?
        }
        _ => false,
    };
    if !aligned {
        bail!("Reservation、Job 与 Capacity Claim 状态或终态时间不一致");
    }
    Ok(())
}

fn terminal_time_matches(
    primary: Option<&str>,
    forbidden: Option<&str>,
    claim_terminal: Option<&str>,
    updated_at: &str,
) -> bool {
    forbidden.is_none()
        && primary.is_some()
        && primary == claim_terminal
        && primary == Some(updated_at)
}

fn time_is_before(value: Option<&str>, boundary: &str, label: &str) -> Result<bool> {
    let Some(value) = value else {
        return Ok(false);
    };
    Ok(parse_utc(label, value)? < parse_utc("Reservation 到期时间", boundary)?)
}

fn time_is_at_or_before(value: Option<&str>, boundary: &str, label: &str) -> Result<bool> {
    let Some(value) = value else {
        return Ok(false);
    };
    Ok(parse_utc(label, value)? <= parse_utc("Reservation 到期时间", boundary)?)
}

fn time_is_at_or_after(value: Option<&str>, boundary: &str, label: &str) -> Result<bool> {
    let Some(value) = value else {
        return Ok(false);
    };
    Ok(parse_utc(label, value)? >= parse_utc("Reservation 到期时间", boundary)?)
}

fn validate_optional_time(label: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = value {
        parse_utc(label, value)?;
    }
    Ok(())
}

fn validate_exact_value(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label}不能为空");
    }
    if value != value.trim() {
        bail!("{label}不能包含首尾空白");
    }
    Ok(())
}

fn parse_utc(label: &str, value: &str) -> Result<DateTime<FixedOffset>> {
    let parsed =
        DateTime::parse_from_rfc3339(value).with_context(|| format!("{label}不是 RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("{label}必须使用 UTC 时区");
    }
    Ok(parsed)
}
