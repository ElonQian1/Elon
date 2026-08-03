use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::execution::ComputeReservation;

use super::{now, Store};

mod audit;
mod dependencies;
mod rows;
mod transitions;

use audit::audited_reservation_on;
use dependencies::{
    ensure_current_job_and_claim_on, ensure_live_creation_dependencies_on,
    registered_dependencies_on, validate_with_dependencies,
};
use rows::{
    current_reservation_projection_on, reservation_id_for_idempotency_on, reservation_version_on,
};
use transitions::{ensure_new_reservation, ensure_reservation_update};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeReservationRegistrationReceipt {
    pub reservation: ComputeReservation,
    pub revision: i64,
    pub reservation_digest: String,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn register_compute_reservation(
        &self,
        reservation: &ComputeReservation,
        expected_revision: i64,
    ) -> Result<ComputeReservationRegistrationReceipt> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let receipt = register_compute_reservation_on(&tx, reservation, expected_revision)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_reservation(
        &self,
        reservation_id: &str,
    ) -> Result<ComputeReservationRegistrationReceipt> {
        if reservation_id.trim().is_empty() {
            bail!("算力 Reservation ID 不能为空");
        }
        let conn = self.conn()?;
        current_registered_reservation_on(&conn, reservation_id.trim())?
            .ok_or_else(|| anyhow!("算力 Reservation 不存在"))
    }
}

pub(super) fn register_compute_reservation_on(
    conn: &Connection,
    reservation: &ComputeReservation,
    expected_revision: i64,
) -> Result<ComputeReservationRegistrationReceipt> {
    if reservation.reservation_id.trim().is_empty() || reservation.idempotency_key.trim().is_empty()
    {
        bail!("算力 Reservation ID 和幂等键不能为空");
    }
    if expected_revision < 0 {
        bail!("算力 Reservation expected_revision 不能为负数");
    }
    let reservation_json = serde_json::to_string(reservation)?;

    if let Some(current) =
        current_reservation_projection_on(conn, reservation.reservation_id.trim())?
    {
        let stored =
            reservation_version_on(conn, &current.reservation_id, current.current_revision)?
                .ok_or_else(|| anyhow!("算力 Reservation 当前历史版本缺失，拒绝继续写入"))?;
        let current_reservation = audited_reservation_on(conn, Some(&current), &stored)?;
        if stored.reservation_json == reservation_json {
            return Ok(ComputeReservationRegistrationReceipt {
                reservation: current_reservation,
                revision: stored.revision,
                reservation_digest: stored.reservation_digest,
                replayed: true,
            });
        }
        if expected_revision != current.current_revision {
            bail!(
                "算力 Reservation expected_revision 与当前版本不一致，当前版本为 {}",
                current.current_revision
            );
        }
        ensure_reservation_update(&current_reservation, reservation)?;
        let dependencies = registered_dependencies_on(conn, reservation)?;
        ensure_current_job_and_claim_on(conn, reservation, &dependencies)?;
        let reservation_digest = validate_with_dependencies(reservation, &dependencies)?;
        let next_revision = current
            .current_revision
            .checked_add(1)
            .ok_or_else(|| anyhow!("算力 Reservation 版本溢出"))?;
        insert_reservation_version(
            conn,
            reservation,
            next_revision,
            &reservation_digest,
            &reservation_json,
        )?;
        let updated = conn.execute(
            "UPDATE compute_reservations
                SET current_revision=?1, current_reservation_digest=?2,
                    status=?3, job_revision=?4, job_digest=?5,
                    capacity_claim_revision=?6, capacity_claim_digest=?7,
                    updated_at=?8, consumed_at=?9, released_at=?10,
                    recorded_at=?11
              WHERE reservation_id=?12 AND current_revision=?13
                AND current_reservation_digest=?14",
            params![
                next_revision,
                reservation_digest,
                reservation.status,
                reservation.job.job_revision,
                reservation.job.job_digest,
                reservation.capacity_claim.claim_revision,
                reservation.capacity_claim.claim_digest,
                reservation.updated_at,
                reservation.consumed_at,
                reservation.released_at,
                now(),
                reservation.reservation_id,
                current.current_revision,
                current.current_reservation_digest,
            ],
        )?;
        if updated != 1 {
            bail!("算力 Reservation 当前投影已变化，请基于最新版本重试");
        }
        return Ok(ComputeReservationRegistrationReceipt {
            reservation: reservation.clone(),
            revision: next_revision,
            reservation_digest,
            replayed: false,
        });
    }

    ensure_new_reservation(reservation, expected_revision)?;
    let dependencies = registered_dependencies_on(conn, reservation)?;
    ensure_current_job_and_claim_on(conn, reservation, &dependencies)?;
    ensure_live_creation_dependencies_on(conn, reservation, &dependencies)?;
    let reservation_digest = validate_with_dependencies(reservation, &dependencies)?;
    let consumer_account_id = dependencies.job.job.consumer_account_id.as_str();
    if let Some(existing_id) = reservation_id_for_idempotency_on(
        conn,
        consumer_account_id,
        reservation.idempotency_key.trim(),
    )? {
        bail!("算力 Reservation 幂等键已属于 {existing_id}");
    }
    conn.execute(
        "INSERT INTO compute_reservations (
            reservation_id, consumer_account_id, idempotency_key,
            current_revision, current_reservation_digest, status,
            job_id, job_revision, job_digest, provider_id, offer_id,
            offer_version, offer_digest, price_snapshot_id,
            capacity_claim_id, capacity_claim_revision,
            capacity_claim_digest, consumer_authorization_ref,
            created_at, updated_at, expires_at, consumed_at, released_at,
            recorded_at
         ) VALUES (
            ?1, ?2, ?3, 1, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
            ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
         )",
        params![
            reservation.reservation_id,
            consumer_account_id,
            reservation.idempotency_key,
            reservation_digest,
            reservation.status,
            reservation.job.job_id,
            reservation.job.job_revision,
            reservation.job.job_digest,
            reservation.offer.provider_id,
            reservation.offer.offer_id,
            reservation.offer.offer_version,
            reservation.offer.offer_digest,
            reservation.price_snapshot.snapshot_id,
            reservation.capacity_claim.claim_id,
            reservation.capacity_claim.claim_revision,
            reservation.capacity_claim.claim_digest,
            reservation.consumer_authorization_ref,
            reservation.created_at,
            reservation.updated_at,
            reservation.expires_at,
            reservation.consumed_at,
            reservation.released_at,
            now(),
        ],
    )?;
    insert_reservation_version(conn, reservation, 1, &reservation_digest, &reservation_json)?;
    Ok(ComputeReservationRegistrationReceipt {
        reservation: reservation.clone(),
        revision: 1,
        reservation_digest,
        replayed: false,
    })
}

pub(super) fn current_registered_reservation_on(
    conn: &Connection,
    reservation_id: &str,
) -> Result<Option<ComputeReservationRegistrationReceipt>> {
    let Some(projection) = current_reservation_projection_on(conn, reservation_id)? else {
        return Ok(None);
    };
    let stored = reservation_version_on(conn, reservation_id, projection.current_revision)?
        .ok_or_else(|| anyhow!("算力 Reservation 当前历史版本缺失"))?;
    let reservation = audited_reservation_on(conn, Some(&projection), &stored)?;
    Ok(Some(ComputeReservationRegistrationReceipt {
        reservation,
        revision: stored.revision,
        reservation_digest: stored.reservation_digest,
        replayed: false,
    }))
}

pub(super) fn registered_reservation_version_on(
    conn: &Connection,
    reservation_id: &str,
    revision: i64,
) -> Result<Option<ComputeReservationRegistrationReceipt>> {
    let Some(stored) = reservation_version_on(conn, reservation_id, revision)? else {
        return Ok(None);
    };
    let reservation = audited_reservation_on(conn, None, &stored)?;
    Ok(Some(ComputeReservationRegistrationReceipt {
        reservation,
        revision: stored.revision,
        reservation_digest: stored.reservation_digest,
        replayed: false,
    }))
}

fn insert_reservation_version(
    conn: &Connection,
    reservation: &ComputeReservation,
    revision: i64,
    reservation_digest: &str,
    reservation_json: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_reservation_versions (
            reservation_id, revision, reservation_digest, status,
            job_id, job_revision, job_digest, provider_id, offer_id,
            offer_version, offer_digest, price_snapshot_id,
            capacity_claim_id, capacity_claim_revision,
            capacity_claim_digest, reservation_json, recorded_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17
         )",
        params![
            reservation.reservation_id,
            revision,
            reservation_digest,
            reservation.status,
            reservation.job.job_id,
            reservation.job.job_revision,
            reservation.job.job_digest,
            reservation.offer.provider_id,
            reservation.offer.offer_id,
            reservation.offer.offer_version,
            reservation.offer.offer_digest,
            reservation.price_snapshot.snapshot_id,
            reservation.capacity_claim.claim_id,
            reservation.capacity_claim.claim_revision,
            reservation.capacity_claim.claim_digest,
            reservation_json,
            now(),
        ],
    )?;
    Ok(())
}
