use anyhow::{anyhow, bail, Context, Result};
use chrono::DateTime;
use rusqlite::Connection;

use crate::compute_federation::{
    capacity::{ComputeCapacityClaim, ComputeCapacityClaimState},
    execution::{ComputeReservation, RESERVATION_STATUS_PENDING},
    market::ComputePriceSnapshot,
    offer::OFFER_STATUS_ACTIVE,
    provider::PROVIDER_STATUS_ACTIVE,
};

use super::super::{
    compute_capacity_claim_rows::{stored_claim_on, stored_claim_version_on},
    compute_job_registry::{
        current_registered_job_on, registered_job_version_on, ComputeJobRegistrationReceipt,
    },
    compute_offer_registry::{
        current_registered_offer_on, registered_offer_version_on, ComputeOfferRegistrationReceipt,
    },
    compute_price_snapshot_registry::registered_price_snapshot_on,
    compute_provider_registry::{
        current_registered_provider_on, registered_provider_version_on,
        ComputeProviderRegistrationReceipt,
    },
    compute_reservation_contract_validation::validate_reservation_contract,
};

pub(super) struct RegisteredReservationDependencies {
    pub(super) job: ComputeJobRegistrationReceipt,
    pub(super) offer: ComputeOfferRegistrationReceipt,
    pub(super) snapshot: ComputePriceSnapshot,
    pub(super) provider: ComputeProviderRegistrationReceipt,
    pub(super) claim: ComputeCapacityClaim,
}

pub(super) fn registered_dependencies_on(
    conn: &Connection,
    reservation: &ComputeReservation,
) -> Result<RegisteredReservationDependencies> {
    let job =
        registered_job_version_on(conn, &reservation.job.job_id, reservation.job.job_revision)?
            .ok_or_else(|| anyhow!("Reservation 绑定的 Job 历史版本不存在"))?;
    if job.job_digest != reservation.job.job_digest {
        bail!("Reservation 绑定的 Job 摘要与历史版本不一致");
    }
    let offer = registered_offer_version_on(
        conn,
        &reservation.offer.offer_id,
        reservation.offer.offer_version,
    )?
    .ok_or_else(|| anyhow!("Reservation 绑定的 Offer 历史版本不存在"))?;
    if offer.offer.offer_digest != reservation.offer.offer_digest {
        bail!("Reservation 绑定的 Offer 摘要与历史版本不一致");
    }
    let snapshot = registered_price_snapshot_on(conn, &reservation.price_snapshot.snapshot_id)?
        .ok_or_else(|| anyhow!("Reservation 绑定的 Price Snapshot 不存在"))?;
    let provider = registered_provider_version_on(
        conn,
        &offer.offer.provider_id,
        offer.provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("Reservation 绑定的 Provider 历史版本不存在"))?;
    if provider.provider_digest != offer.provider_digest {
        bail!("Reservation 绑定的 Provider 摘要与 Offer 历史版本不一致");
    }
    let claim = stored_claim_version_on(
        conn,
        &reservation.capacity_claim.claim_id,
        reservation.capacity_claim.claim_revision,
    )?
    .ok_or_else(|| anyhow!("Reservation 绑定的 Capacity Claim 历史版本不存在"))?;
    if claim.claim_digest != reservation.capacity_claim.claim_digest {
        bail!("Reservation 绑定的 Capacity Claim 摘要与历史版本不一致");
    }
    Ok(RegisteredReservationDependencies {
        job,
        offer,
        snapshot,
        provider,
        claim,
    })
}

pub(super) fn validate_with_dependencies(
    reservation: &ComputeReservation,
    dependencies: &RegisteredReservationDependencies,
) -> Result<String> {
    validate_reservation_contract(
        reservation,
        &dependencies.job.job,
        dependencies.job.revision,
        &dependencies.job.job_digest,
        &dependencies.offer.offer,
        &dependencies.snapshot,
        &dependencies.provider.provider,
        &dependencies.claim,
    )
}

pub(super) fn ensure_current_job_and_claim_on(
    conn: &Connection,
    reservation: &ComputeReservation,
    dependencies: &RegisteredReservationDependencies,
) -> Result<()> {
    let current_job = current_registered_job_on(conn, &reservation.job.job_id)?
        .ok_or_else(|| anyhow!("Reservation 绑定的当前 Job 不存在"))?;
    if current_job.revision != dependencies.job.revision
        || current_job.job_digest != dependencies.job.job_digest
    {
        bail!("Reservation 必须绑定 Job 的当前精确版本");
    }
    let current_claim = stored_claim_on(conn, &reservation.capacity_claim.claim_id)?
        .ok_or_else(|| anyhow!("Reservation 绑定的当前 Capacity Claim 不存在"))?;
    if current_claim.revision != dependencies.claim.revision
        || current_claim.claim_digest != dependencies.claim.claim_digest
    {
        bail!("Reservation 必须绑定 Capacity Claim 的当前精确版本");
    }
    Ok(())
}

pub(super) fn ensure_live_creation_dependencies_on(
    conn: &Connection,
    reservation: &ComputeReservation,
    dependencies: &RegisteredReservationDependencies,
) -> Result<()> {
    if reservation.status != RESERVATION_STATUS_PENDING
        || dependencies.claim.state != ComputeCapacityClaimState::Held
    {
        bail!("新 Reservation 必须绑定 held Capacity Claim 并处于 pending 状态");
    }
    let current_offer = current_registered_offer_on(conn, &reservation.offer.offer_id)?
        .ok_or_else(|| anyhow!("Reservation 绑定的当前 Offer 不存在"))?;
    if current_offer.offer.offer_version != reservation.offer.offer_version
        || current_offer.offer.offer_digest != reservation.offer.offer_digest
        || current_offer.offer.status != OFFER_STATUS_ACTIVE
    {
        bail!("新 Reservation 只能绑定当前 active Offer 版本");
    }
    let current_provider = current_registered_provider_on(conn, &reservation.offer.provider_id)?
        .ok_or_else(|| anyhow!("Reservation 绑定的当前 Provider 不存在"))?;
    if current_provider.provider.status != PROVIDER_STATUS_ACTIVE {
        bail!("新 Reservation 只能绑定当前 active Provider");
    }
    ensure_live_time(
        "Offer",
        &dependencies.offer.offer.valid_from,
        &dependencies.offer.offer.valid_until,
    )?;
    ensure_not_expired("Price Snapshot", &dependencies.snapshot.expires_at)?;
    ensure_not_expired("Reservation", &reservation.expires_at)
}

fn ensure_live_time(label: &str, starts_at: &str, ends_at: &str) -> Result<()> {
    let starts = DateTime::parse_from_rfc3339(starts_at)
        .with_context(|| format!("{label} 生效时间不是 RFC3339"))?;
    let ends = DateTime::parse_from_rfc3339(ends_at)
        .with_context(|| format!("{label} 失效时间不是 RFC3339"))?;
    let now = chrono::Utc::now();
    if starts > now || ends <= now {
        bail!("新 Reservation 只能绑定当前有效的 {label}");
    }
    Ok(())
}

fn ensure_not_expired(label: &str, expires_at: &str) -> Result<()> {
    let expires = DateTime::parse_from_rfc3339(expires_at)
        .with_context(|| format!("{label} 失效时间不是 RFC3339"))?;
    if expires <= chrono::Utc::now() {
        bail!("新 Reservation 不能绑定已经失效的 {label}");
    }
    Ok(())
}
