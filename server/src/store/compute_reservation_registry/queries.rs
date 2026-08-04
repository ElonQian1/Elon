use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};

use super::{current_registered_reservation_on, ComputeReservationRegistrationReceipt};

pub(super) fn list_current_reservations_on(
    conn: &Connection,
    consumer_account_id: &str,
    project_id: Option<&str>,
    limit: usize,
) -> Result<Vec<ComputeReservationRegistrationReceipt>> {
    let reservation_ids = if let Some(project_id) = project_id {
        let mut stmt = conn.prepare(
            "SELECT reservation.reservation_id
               FROM compute_reservations AS reservation
               JOIN compute_jobs AS job ON job.job_id=reservation.job_id
              WHERE reservation.consumer_account_id=?1 AND job.project_id=?2
              ORDER BY reservation.recorded_at DESC, reservation.reservation_id ASC
              LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(
                params![consumer_account_id, project_id, limit as i64],
                |row| row.get::<_, String>(0),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    } else {
        let mut stmt = conn.prepare(
            "SELECT reservation_id
               FROM compute_reservations
              WHERE consumer_account_id=?1
              ORDER BY recorded_at DESC, reservation_id ASC
              LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![consumer_account_id, limit as i64], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    reservation_ids
        .into_iter()
        .map(|reservation_id| {
            current_registered_reservation_on(conn, &reservation_id)?
                .ok_or_else(|| anyhow!("算力 Reservation 当前投影缺失"))
        })
        .collect()
}
