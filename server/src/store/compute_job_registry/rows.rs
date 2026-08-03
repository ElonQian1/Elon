use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug)]
pub(super) struct CurrentJobProjection {
    pub job_id: String,
    pub consumer_account_id: String,
    pub project_id: Option<String>,
    pub merchant_id: Option<String>,
    pub idempotency_key: String,
    pub current_revision: i64,
    pub current_job_digest: String,
    pub status: String,
    pub selected_provider_id: Option<String>,
    pub selected_offer_id: Option<String>,
    pub selected_offer_version: Option<i64>,
    pub selected_offer_digest: Option<String>,
    pub price_snapshot_id: Option<String>,
    pub max_consumer_charge_micros: i64,
    pub currency: String,
    pub submitted_at: String,
    pub updated_at: String,
}

#[derive(Debug)]
pub(super) struct StoredJobVersion {
    pub job_id: String,
    pub revision: i64,
    pub job_digest: String,
    pub status: String,
    pub selected_provider_id: Option<String>,
    pub selected_offer_id: Option<String>,
    pub selected_offer_version: Option<i64>,
    pub selected_offer_digest: Option<String>,
    pub price_snapshot_id: Option<String>,
    pub job_json: String,
}

pub(super) fn current_job_projection_on(
    conn: &Connection,
    job_id: &str,
) -> Result<Option<CurrentJobProjection>> {
    conn.query_row(
        "SELECT job_id, consumer_account_id, project_id, merchant_id,
                idempotency_key, current_revision, current_job_digest, status,
                selected_provider_id, selected_offer_id, selected_offer_version,
                selected_offer_digest, price_snapshot_id,
                max_consumer_charge_micros, currency, submitted_at, updated_at
           FROM compute_jobs WHERE job_id=?1",
        params![job_id],
        |row| {
            Ok(CurrentJobProjection {
                job_id: row.get(0)?,
                consumer_account_id: row.get(1)?,
                project_id: row.get(2)?,
                merchant_id: row.get(3)?,
                idempotency_key: row.get(4)?,
                current_revision: row.get(5)?,
                current_job_digest: row.get(6)?,
                status: row.get(7)?,
                selected_provider_id: row.get(8)?,
                selected_offer_id: row.get(9)?,
                selected_offer_version: row.get(10)?,
                selected_offer_digest: row.get(11)?,
                price_snapshot_id: row.get(12)?,
                max_consumer_charge_micros: row.get(13)?,
                currency: row.get(14)?,
                submitted_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn job_version_on(
    conn: &Connection,
    job_id: &str,
    revision: i64,
) -> Result<Option<StoredJobVersion>> {
    conn.query_row(
        "SELECT job_id, revision, job_digest, status,
                selected_provider_id, selected_offer_id, selected_offer_version,
                selected_offer_digest, price_snapshot_id, job_json
           FROM compute_job_versions
          WHERE job_id=?1 AND revision=?2",
        params![job_id, revision],
        |row| {
            Ok(StoredJobVersion {
                job_id: row.get(0)?,
                revision: row.get(1)?,
                job_digest: row.get(2)?,
                status: row.get(3)?,
                selected_provider_id: row.get(4)?,
                selected_offer_id: row.get(5)?,
                selected_offer_version: row.get(6)?,
                selected_offer_digest: row.get(7)?,
                price_snapshot_id: row.get(8)?,
                job_json: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn job_id_for_idempotency_on(
    conn: &Connection,
    consumer_account_id: &str,
    idempotency_key: &str,
) -> Result<Option<String>> {
    conn.query_row(
        "SELECT job_id FROM compute_jobs
          WHERE consumer_account_id=?1 AND idempotency_key=?2",
        params![consumer_account_id, idempotency_key],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}
