use anyhow::{anyhow, bail, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension};

use crate::{
    compute_federation::{
        execution::{JOB_STATUS_RESERVED, RESERVATION_STATUS_ACTIVE},
        offer::{OFFER_STATUS_ACTIVE, OFFER_STATUS_DRAINING},
        provider::{PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_DRAINING},
    },
    store::{
        compute_broker_reservation::BrokerReserveBinding,
        compute_job_registry::ComputeJobRegistrationReceipt,
        compute_offer_registry::current_registered_offer_on,
        compute_provider_registry::current_registered_provider_on,
        compute_reservation_registry::ComputeReservationRegistrationReceipt,
    },
};

use super::validation::{parse_utc, NormalizedAttemptActivation};

pub(super) fn ensure_reservation_matches(
    request: &NormalizedAttemptActivation,
    source: &ComputeReservationRegistrationReceipt,
) -> Result<()> {
    if source.revision != request.expected_reservation_revision
        || source.reservation_digest != request.expected_reservation_digest
        || source.reservation.status != RESERVATION_STATUS_ACTIVE
        || source.reservation.capacity_claim.claim_revision != request.expected_claim_revision
        || source.reservation.capacity_claim.claim_digest != request.expected_claim_digest
    {
        bail!("Attempt 激活只能基于当前 active Reservation 与精确 Capacity Claim 版本");
    }
    Ok(())
}

pub(super) fn ensure_job_matches(
    request: &NormalizedAttemptActivation,
    reservation: &ComputeReservationRegistrationReceipt,
    job: &ComputeJobRegistrationReceipt,
) -> Result<()> {
    if job.revision != request.expected_job_revision
        || job.job_digest != request.expected_job_digest
        || job.job.status != JOB_STATUS_RESERVED
        || reservation.reservation.job.job_revision != job.revision
        || reservation.reservation.job.job_digest != job.job_digest
    {
        bail!("Attempt 激活只能基于 Reservation 绑定的当前 reserved Job 精确版本");
    }
    Ok(())
}

pub(super) fn ensure_provider_and_offer_live(
    conn: &rusqlite::Connection,
    request: &NormalizedAttemptActivation,
    reservation: &ComputeReservationRegistrationReceipt,
) -> Result<()> {
    let provider = current_registered_provider_on(conn, &request.provider_id)?
        .ok_or_else(|| anyhow!("Attempt 激活的 Provider 不存在"))?;
    if provider.provider.owner_account_id != request.activated_by_user_id
        || !matches!(
            provider.provider.status.as_str(),
            PROVIDER_STATUS_ACTIVE | PROVIDER_STATUS_DRAINING
        )
    {
        bail!("只有当前 Provider 所有者可在 active/draining 状态履行既有 Reservation");
    }
    let current_offer = current_registered_offer_on(conn, &reservation.reservation.offer.offer_id)?
        .ok_or_else(|| anyhow!("Attempt 激活绑定的 Offer 不存在"))?;
    if current_offer.offer.provider_id != request.provider_id
        || !matches!(
            current_offer.offer.status.as_str(),
            OFFER_STATUS_ACTIVE | OFFER_STATUS_DRAINING
        )
    {
        bail!("终态或身份变化的 Offer 不能再激活 Attempt");
    }
    Ok(())
}

pub(super) fn ensure_broker_binding(
    broker: &BrokerReserveBinding,
    reservation: &ComputeReservationRegistrationReceipt,
    job: &ComputeJobRegistrationReceipt,
) -> Result<()> {
    if broker.reservation_revision != reservation.revision
        || broker.reservation_digest != reservation.reservation_digest
        || broker.reserved_job.job_id != job.job.job_id
        || broker.reserved_job.job_revision != job.revision
        || broker.reserved_job.job_digest != job.job_digest
        || broker.capacity_claim != reservation.reservation.capacity_claim
    {
        bail!("Attempt 激活前 Broker 预留回执与当前合同不一致");
    }
    Ok(())
}

pub(super) fn ensure_budget_reserved(
    conn: &rusqlite::Connection,
    broker: &BrokerReserveBinding,
    consumer_account_id: &str,
    activated_at: &str,
) -> Result<()> {
    let current_expiry = conn
        .query_row(
            "SELECT expires_at FROM billing_reservations
              WHERE id=?1 AND user_id=?2 AND reserved_fen=?3 AND status='reserved'",
            params![
                broker.budget_reservation_id,
                consumer_account_id,
                broker.budget_reserved_fen,
            ],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?;
    let valid = match current_expiry {
        Some(Some(expires_at)) => parse_utc(&expires_at)? >= parse_utc(activated_at)?,
        Some(None) => true,
        None => false,
    };
    if !valid {
        bail!("Attempt 激活要求原 Broker 平台余额预授权仍有效且未发生资金变化");
    }
    Ok(())
}

pub(super) fn ensure_lease_window(
    request: &NormalizedAttemptActivation,
    reservation_expires_at: &str,
    activated_at: &str,
) -> Result<()> {
    let activated = parse_utc(activated_at)?;
    let lease_expires = parse_utc(&request.expires_at)?;
    let hard_deadline = parse_utc(&request.hard_deadline_at)?;
    let reservation_expires = parse_utc(reservation_expires_at)?;
    if lease_expires <= activated
        || hard_deadline <= lease_expires
        || hard_deadline > reservation_expires
    {
        bail!("Attempt Lease 时间窗必须位于未过期 Reservation 内");
    }
    Ok(())
}

pub(super) fn activation_timestamp(
    job_updated_at: &str,
    reservation_updated_at: &str,
    supplied: Option<&str>,
) -> Result<String> {
    let floor = std::cmp::max(
        parse_utc(job_updated_at)?,
        parse_utc(reservation_updated_at)?,
    )
    .checked_add_signed(Duration::nanoseconds(1))
    .ok_or_else(|| anyhow!("Attempt 激活时间溢出"))?;
    match supplied {
        Some(value) => {
            if parse_utc(value)? < floor {
                bail!("Attempt 激活显式时间早于当前 Job/Reservation 版本");
            }
            Ok(value.to_string())
        }
        None => Ok(std::cmp::max(Utc::now(), floor).to_rfc3339()),
    }
}
