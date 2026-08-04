use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};

use super::StoredPlatformObservation;

pub(in crate::store::compute_attempt_platform_observations) fn platform_observation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredPlatformObservation>> {
    query_one(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(in crate::store::compute_attempt_platform_observations) fn platform_observation_by_candidate_on(
    conn: &Connection,
    candidate_id: &str,
) -> Result<Option<StoredPlatformObservation>> {
    query_one(
        conn,
        "WHERE terminal_candidate_id=?1",
        params![candidate_id],
    )
}

pub(in crate::store::compute_attempt_platform_observations) fn platform_observation_by_lease_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredPlatformObservation>> {
    query_one(conn, "WHERE lease_id=?1", params![lease_id])
}

fn query_one<P: rusqlite::Params>(
    conn: &Connection,
    clause: &str,
    params: P,
) -> Result<Option<StoredPlatformObservation>> {
    conn.query_row(
        &format!("{} {clause}", select_sql()),
        params,
        stored_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn select_sql() -> &'static str {
    "SELECT platform_observation_id, terminal_candidate_id,
            terminal_candidate_event_digest, lease_id, provider_id,
            consumer_account_id, source_lease_revision, source_lease_digest,
            fencing_generation, job_id, job_revision, job_digest,
            reservation_id, reservation_revision, reservation_digest,
            capacity_claim_id, capacity_claim_revision, capacity_claim_digest,
            final_usage_snapshot_id, final_usage_sequence_no,
            final_provider_usage_digest, candidate_outcome, observation_source,
            observer_ref, observed_outcome, cumulative_observed_usage_json,
            cumulative_observed_usage_digest, variance_meters_json,
            variance_meters_digest, evidence_refs_json, evidence_refs_digest,
            request_digest, event_digest, idempotency_scope, idempotency_key,
            observed_by_user_id, observed_at, created_at
       FROM compute_attempt_platform_observations"
}

fn stored_from_row(row: &Row<'_>) -> rusqlite::Result<StoredPlatformObservation> {
    Ok(StoredPlatformObservation {
        platform_observation_id: row.get(0)?,
        terminal_candidate_id: row.get(1)?,
        terminal_candidate_event_digest: row.get(2)?,
        lease_id: row.get(3)?,
        provider_id: row.get(4)?,
        consumer_account_id: row.get(5)?,
        source_lease_revision: row.get(6)?,
        source_lease_digest: row.get(7)?,
        fencing_generation: row.get(8)?,
        job_id: row.get(9)?,
        job_revision: row.get(10)?,
        job_digest: row.get(11)?,
        reservation_id: row.get(12)?,
        reservation_revision: row.get(13)?,
        reservation_digest: row.get(14)?,
        capacity_claim_id: row.get(15)?,
        capacity_claim_revision: row.get(16)?,
        capacity_claim_digest: row.get(17)?,
        final_usage_snapshot_id: row.get(18)?,
        final_usage_sequence_no: row.get(19)?,
        final_provider_usage_digest: row.get(20)?,
        candidate_outcome: row.get(21)?,
        observation_source: row.get(22)?,
        observer_ref: row.get(23)?,
        observed_outcome: row.get(24)?,
        cumulative_observed_usage: parse_json(row, 25)?,
        cumulative_observed_usage_digest: row.get(26)?,
        variance_meters: parse_json(row, 27)?,
        variance_meters_digest: row.get(28)?,
        evidence_refs: parse_json(row, 29)?,
        evidence_refs_digest: row.get(30)?,
        request_digest: row.get(31)?,
        event_digest: row.get(32)?,
        idempotency_scope: row.get(33)?,
        idempotency_key: row.get(34)?,
        observed_by_user_id: row.get(35)?,
        observed_at: row.get(36)?,
        created_at: row.get(37)?,
    })
}

fn parse_json<T: serde::de::DeserializeOwned>(row: &Row<'_>, index: usize) -> rusqlite::Result<T> {
    let value: String = row.get(index)?;
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}
