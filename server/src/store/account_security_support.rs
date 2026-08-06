use rusqlite::{params, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{new_id, now};

pub(super) fn generate_recovery_code() -> String {
    let value = Uuid::new_v4().simple().to_string().to_ascii_uppercase();
    format!(
        "ELON-{}-{}-{}-{}-{}",
        &value[0..4],
        &value[4..8],
        &value[8..12],
        &value[12..16],
        &value[16..20]
    )
}

pub(super) fn recovery_code_hash(code: &str) -> String {
    let normalized = code.trim().to_ascii_uppercase();
    hex::encode(Sha256::digest(normalized.as_bytes()))
}

pub(super) fn security_request_outcome(
    transaction: &Transaction<'_>,
    user_id: &str,
    action: &str,
    request_id: &str,
) -> rusqlite::Result<Option<String>> {
    transaction
        .query_row(
            "SELECT outcome FROM account_security_requests
             WHERE user_id = ?1 AND action = ?2 AND request_id = ?3",
            params![user_id, action, request_id],
            |row| row.get(0),
        )
        .optional()
}

pub(super) fn record_security_request(
    transaction: &Transaction<'_>,
    user_id: &str,
    action: &str,
    request_id: &str,
    outcome: &str,
) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT INTO account_security_requests
         (id, user_id, action, request_id, outcome, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![new_id("asr"), user_id, action, request_id, outcome, now()],
    )?;
    Ok(())
}

pub(super) fn security_audit(
    transaction: &Transaction<'_>,
    user_id: Option<&str>,
    action: &str,
    outcome: &str,
    session_id: Option<&str>,
    request_id: Option<&str>,
    reason_code: Option<&str>,
) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT INTO auth_security_audit
         (id, user_id, action, outcome, session_id, request_id, reason_code, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            new_id("asa"),
            user_id,
            action,
            outcome,
            session_id,
            request_id,
            reason_code,
            now()
        ],
    )?;
    Ok(())
}
