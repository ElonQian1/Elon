use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};

use super::super::{
    compute_reservation_registry::{
        current_registered_reservation_on, ComputeReservationRegistrationReceipt,
    },
    now,
};

pub(super) fn list_activation_candidates_on(
    conn: &Connection,
    provider_id: &str,
    limit: usize,
) -> Result<Vec<ComputeReservationRegistrationReceipt>> {
    let observed_at = now();
    let mut statement = conn.prepare(
        "SELECT reservation.reservation_id
           FROM compute_reservations AS reservation
           JOIN compute_jobs AS job ON job.job_id=reservation.job_id
          WHERE reservation.provider_id=?1
            AND reservation.status='active'
            AND reservation.expires_at>?2
            AND job.status='reserved'
            AND NOT EXISTS (
                SELECT 1
                  FROM compute_attempt_activations AS activation
                 WHERE activation.job_id=reservation.job_id
            )
          ORDER BY reservation.recorded_at ASC, reservation.reservation_id ASC
          LIMIT ?3",
    )?;
    let reservation_ids = statement
        .query_map(params![provider_id, observed_at, limit as i64], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    reservation_ids
        .into_iter()
        .map(|reservation_id| {
            current_registered_reservation_on(conn, &reservation_id)?
                .ok_or_else(|| anyhow!("Attempt 激活候选的 Reservation 当前投影缺失"))
        })
        .collect()
}
