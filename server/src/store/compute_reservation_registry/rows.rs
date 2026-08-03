use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug)]
pub(super) struct CurrentReservationProjection {
    pub reservation_id: String,
    pub consumer_account_id: String,
    pub idempotency_key: String,
    pub current_revision: i64,
    pub current_reservation_digest: String,
    pub status: String,
    pub job_id: String,
    pub job_revision: i64,
    pub job_digest: String,
    pub provider_id: String,
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
    pub price_snapshot_id: String,
    pub capacity_claim_id: String,
    pub capacity_claim_revision: i64,
    pub capacity_claim_digest: String,
    pub consumer_authorization_ref: String,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: String,
    pub consumed_at: Option<String>,
    pub released_at: Option<String>,
}

#[derive(Debug)]
pub(super) struct StoredReservationVersion {
    pub reservation_id: String,
    pub revision: i64,
    pub reservation_digest: String,
    pub status: String,
    pub job_id: String,
    pub job_revision: i64,
    pub job_digest: String,
    pub provider_id: String,
    pub offer_id: String,
    pub offer_version: i64,
    pub offer_digest: String,
    pub price_snapshot_id: String,
    pub capacity_claim_id: String,
    pub capacity_claim_revision: i64,
    pub capacity_claim_digest: String,
    pub reservation_json: String,
}

pub(super) fn current_reservation_projection_on(
    conn: &Connection,
    reservation_id: &str,
) -> Result<Option<CurrentReservationProjection>> {
    conn.query_row(
        "SELECT reservation_id, consumer_account_id, idempotency_key,
                current_revision, current_reservation_digest, status,
                job_id, job_revision, job_digest, provider_id, offer_id,
                offer_version, offer_digest, price_snapshot_id,
                capacity_claim_id, capacity_claim_revision,
                capacity_claim_digest, consumer_authorization_ref,
                created_at, updated_at, expires_at, consumed_at, released_at
           FROM compute_reservations WHERE reservation_id=?1",
        params![reservation_id],
        |row| {
            Ok(CurrentReservationProjection {
                reservation_id: row.get(0)?,
                consumer_account_id: row.get(1)?,
                idempotency_key: row.get(2)?,
                current_revision: row.get(3)?,
                current_reservation_digest: row.get(4)?,
                status: row.get(5)?,
                job_id: row.get(6)?,
                job_revision: row.get(7)?,
                job_digest: row.get(8)?,
                provider_id: row.get(9)?,
                offer_id: row.get(10)?,
                offer_version: row.get(11)?,
                offer_digest: row.get(12)?,
                price_snapshot_id: row.get(13)?,
                capacity_claim_id: row.get(14)?,
                capacity_claim_revision: row.get(15)?,
                capacity_claim_digest: row.get(16)?,
                consumer_authorization_ref: row.get(17)?,
                created_at: row.get(18)?,
                updated_at: row.get(19)?,
                expires_at: row.get(20)?,
                consumed_at: row.get(21)?,
                released_at: row.get(22)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn reservation_version_on(
    conn: &Connection,
    reservation_id: &str,
    revision: i64,
) -> Result<Option<StoredReservationVersion>> {
    conn.query_row(
        "SELECT reservation_id, revision, reservation_digest, status,
                job_id, job_revision, job_digest, provider_id, offer_id,
                offer_version, offer_digest, price_snapshot_id,
                capacity_claim_id, capacity_claim_revision,
                capacity_claim_digest, reservation_json
           FROM compute_reservation_versions
          WHERE reservation_id=?1 AND revision=?2",
        params![reservation_id, revision],
        |row| {
            Ok(StoredReservationVersion {
                reservation_id: row.get(0)?,
                revision: row.get(1)?,
                reservation_digest: row.get(2)?,
                status: row.get(3)?,
                job_id: row.get(4)?,
                job_revision: row.get(5)?,
                job_digest: row.get(6)?,
                provider_id: row.get(7)?,
                offer_id: row.get(8)?,
                offer_version: row.get(9)?,
                offer_digest: row.get(10)?,
                price_snapshot_id: row.get(11)?,
                capacity_claim_id: row.get(12)?,
                capacity_claim_revision: row.get(13)?,
                capacity_claim_digest: row.get(14)?,
                reservation_json: row.get(15)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn reservation_id_for_idempotency_on(
    conn: &Connection,
    consumer_account_id: &str,
    idempotency_key: &str,
) -> Result<Option<String>> {
    conn.query_row(
        "SELECT reservation_id FROM compute_reservations
          WHERE consumer_account_id=?1 AND idempotency_key=?2",
        params![consumer_account_id, idempotency_key],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}
