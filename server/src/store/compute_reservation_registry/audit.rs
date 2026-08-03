use anyhow::{bail, Context, Result};
use rusqlite::Connection;

use crate::compute_federation::execution::ComputeReservation;

use super::{
    dependencies::{
        registered_dependencies_on, validate_with_dependencies, RegisteredReservationDependencies,
    },
    rows::{CurrentReservationProjection, StoredReservationVersion},
};

pub(super) fn audited_reservation_on(
    conn: &Connection,
    projection: Option<&CurrentReservationProjection>,
    stored: &StoredReservationVersion,
) -> Result<ComputeReservation> {
    let reservation: ComputeReservation = serde_json::from_str(&stored.reservation_json)
        .context("算力 Reservation 历史版本 JSON 无效")?;
    let dependencies = registered_dependencies_on(conn, &reservation)?;
    let computed_digest = validate_with_dependencies(&reservation, &dependencies)?;
    if computed_digest != stored.reservation_digest
        || reservation.reservation_id != stored.reservation_id
        || reservation.status != stored.status
        || reservation.job.job_id != stored.job_id
        || reservation.job.job_revision != stored.job_revision
        || reservation.job.job_digest != stored.job_digest
        || reservation.offer.provider_id != stored.provider_id
        || reservation.offer.offer_id != stored.offer_id
        || reservation.offer.offer_version != stored.offer_version
        || reservation.offer.offer_digest != stored.offer_digest
        || reservation.price_snapshot.snapshot_id != stored.price_snapshot_id
        || reservation.capacity_claim.claim_id != stored.capacity_claim_id
        || reservation.capacity_claim.claim_revision != stored.capacity_claim_revision
        || reservation.capacity_claim.claim_digest != stored.capacity_claim_digest
    {
        bail!("算力 Reservation 历史版本身份、摘要或索引字段审计失败");
    }
    if let Some(projection) = projection {
        ensure_current_projection(&reservation, stored, projection, &dependencies)?;
    }
    Ok(reservation)
}

fn ensure_current_projection(
    reservation: &ComputeReservation,
    stored: &StoredReservationVersion,
    projection: &CurrentReservationProjection,
    dependencies: &RegisteredReservationDependencies,
) -> Result<()> {
    if reservation.reservation_id != projection.reservation_id
        || dependencies.job.job.consumer_account_id != projection.consumer_account_id
        || reservation.idempotency_key != projection.idempotency_key
        || stored.revision != projection.current_revision
        || stored.reservation_digest != projection.current_reservation_digest
        || reservation.status != projection.status
        || reservation.job.job_id != projection.job_id
        || reservation.job.job_revision != projection.job_revision
        || reservation.job.job_digest != projection.job_digest
        || reservation.offer.provider_id != projection.provider_id
        || reservation.offer.offer_id != projection.offer_id
        || reservation.offer.offer_version != projection.offer_version
        || reservation.offer.offer_digest != projection.offer_digest
        || reservation.price_snapshot.snapshot_id != projection.price_snapshot_id
        || reservation.capacity_claim.claim_id != projection.capacity_claim_id
        || reservation.capacity_claim.claim_revision != projection.capacity_claim_revision
        || reservation.capacity_claim.claim_digest != projection.capacity_claim_digest
        || reservation.consumer_authorization_ref != projection.consumer_authorization_ref
        || reservation.created_at != projection.created_at
        || reservation.updated_at != projection.updated_at
        || reservation.expires_at != projection.expires_at
        || reservation.consumed_at != projection.consumed_at
        || reservation.released_at != projection.released_at
    {
        bail!("算力 Reservation 当前投影与不可变版本不一致");
    }
    Ok(())
}
