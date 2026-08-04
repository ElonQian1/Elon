use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::compute_federation::receipts::ComputeMeterReading;

use super::StoredVerificationDecision;

pub(super) fn verification_decision_by_idempotency_on(
    conn: &Connection,
    idempotency_scope: &str,
    idempotency_key: &str,
) -> Result<Option<StoredVerificationDecision>> {
    query_one(
        conn,
        "SELECT verification_decision_id, terminal_candidate_id,
                terminal_candidate_event_digest, consumer_review_id,
                consumer_review_event_digest, platform_observation_id,
                platform_observation_event_digest, lease_id, policy_id,
                policy_version, decision, reason_codes_json,
                reason_codes_digest, decision_ref, verified_usage_json,
                verified_usage_digest, compensable_usage_json,
                compensable_usage_digest, request_digest, event_digest,
                idempotency_scope, idempotency_key, decided_by_user_id,
                decided_at, created_at
           FROM compute_attempt_verification_decisions
          WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![idempotency_scope, idempotency_key],
    )
}

pub(super) fn verification_decision_by_candidate_on(
    conn: &Connection,
    terminal_candidate_id: &str,
) -> Result<Option<StoredVerificationDecision>> {
    query_one(
        conn,
        "SELECT verification_decision_id, terminal_candidate_id,
                terminal_candidate_event_digest, consumer_review_id,
                consumer_review_event_digest, platform_observation_id,
                platform_observation_event_digest, lease_id, policy_id,
                policy_version, decision, reason_codes_json,
                reason_codes_digest, decision_ref, verified_usage_json,
                verified_usage_digest, compensable_usage_json,
                compensable_usage_digest, request_digest, event_digest,
                idempotency_scope, idempotency_key, decided_by_user_id,
                decided_at, created_at
           FROM compute_attempt_verification_decisions
          WHERE terminal_candidate_id=?1",
        params![terminal_candidate_id],
    )
}

pub(super) fn verification_decision_by_lease_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredVerificationDecision>> {
    query_one(
        conn,
        "SELECT verification_decision_id, terminal_candidate_id,
                terminal_candidate_event_digest, consumer_review_id,
                consumer_review_event_digest, platform_observation_id,
                platform_observation_event_digest, lease_id, policy_id,
                policy_version, decision, reason_codes_json,
                reason_codes_digest, decision_ref, verified_usage_json,
                verified_usage_digest, compensable_usage_json,
                compensable_usage_digest, request_digest, event_digest,
                idempotency_scope, idempotency_key, decided_by_user_id,
                decided_at, created_at
           FROM compute_attempt_verification_decisions
          WHERE lease_id=?1",
        params![lease_id],
    )
}

fn query_one<P: rusqlite::Params>(
    conn: &Connection,
    sql: &str,
    params: P,
) -> Result<Option<StoredVerificationDecision>> {
    conn.query_row(sql, params, stored_from_row)
        .optional()
        .map_err(Into::into)
}

fn stored_from_row(row: &Row<'_>) -> rusqlite::Result<StoredVerificationDecision> {
    let reason_codes_json: String = row.get(11)?;
    let verified_usage_json: String = row.get(14)?;
    let compensable_usage_json: String = row.get(16)?;
    Ok(StoredVerificationDecision {
        verification_decision_id: row.get(0)?,
        terminal_candidate_id: row.get(1)?,
        terminal_candidate_event_digest: row.get(2)?,
        consumer_review_id: row.get(3)?,
        consumer_review_event_digest: row.get(4)?,
        platform_observation_id: row.get(5)?,
        platform_observation_event_digest: row.get(6)?,
        lease_id: row.get(7)?,
        policy_id: row.get(8)?,
        policy_version: row.get(9)?,
        decision: row.get(10)?,
        reason_codes: serde_json::from_str::<Vec<String>>(&reason_codes_json)
            .map_err(json_error)?,
        reason_codes_digest: row.get(12)?,
        decision_ref: row.get(13)?,
        verified_usage: serde_json::from_str::<Vec<ComputeMeterReading>>(&verified_usage_json)
            .map_err(json_error)?,
        verified_usage_digest: row.get(15)?,
        compensable_usage: serde_json::from_str::<Vec<ComputeMeterReading>>(
            &compensable_usage_json,
        )
        .map_err(json_error)?,
        compensable_usage_digest: row.get(17)?,
        request_digest: row.get(18)?,
        event_digest: row.get(19)?,
        idempotency_scope: row.get(20)?,
        idempotency_key: row.get(21)?,
        decided_by_user_id: row.get(22)?,
        decided_at: row.get(23)?,
        created_at: row.get(24)?,
    })
}

fn json_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
