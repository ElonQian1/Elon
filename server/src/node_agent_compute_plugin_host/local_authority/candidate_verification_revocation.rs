use crate::node_agent_compute_plugin_host::candidate_verification_terminal_result::encode_candidate_verification_revocation;
use anyhow::{bail, Context, Result};
use rusqlite::{named_params, params, Transaction};

pub(super) fn revoke_for_process_owner_epoch_advance(
    transaction: &Transaction<'_>,
    expected_authority_epoch: i64,
    expected_old_process_epoch: i64,
    resolved_at_ms: i64,
) -> Result<()> {
    let impossible = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_verification_runs
            WHERE state = 'prepared' AND (
                authority_epoch > ?1
                OR process_owner_epoch > ?2
                OR prepared_at_ms > ?3
            )"#,
            params![
                expected_authority_epoch,
                expected_old_process_epoch,
                resolved_at_ms
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_REVOKE_FUTURE_PROCESS_CHECK")?;
    if impossible != 0 {
        bail!("COMPUTE_PLUGIN_VERIFICATION_REVOKE_FUTURE_PROCESS_RUN");
    }
    revoke_matching(
        transaction,
        "process_owner_epoch_advanced",
        resolved_at_ms,
        r#"UPDATE candidate_verification_runs SET
            state = 'revoked', resolved_at_ms = :resolved_at,
            resolution_reason = :reason, result_json = :result_json,
            result_digest = :result_digest
        WHERE state = 'prepared' AND process_owner_epoch <= :fence"#,
        expected_old_process_epoch,
    )?;
    require_no_prepared(transaction)
}

pub(super) fn revoke_for_authority_epoch_advance(
    transaction: &Transaction<'_>,
    expected_old_authority_epoch: i64,
    current_process_owner_epoch: i64,
    reason: &str,
    resolved_at_ms: i64,
) -> Result<()> {
    if !matches!(
        reason,
        "authority_epoch_advanced_by_keyring"
            | "authority_epoch_advanced_by_plan"
            | "authority_epoch_advanced_by_verification"
    ) {
        bail!("COMPUTE_PLUGIN_VERIFICATION_REVOKE_AUTHORITY_REASON_INVALID");
    }
    let impossible = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_verification_runs
            WHERE state = 'prepared' AND (
                authority_epoch > ?1
                OR process_owner_epoch > ?2
                OR prepared_at_ms > ?3
            )"#,
            params![
                expected_old_authority_epoch,
                current_process_owner_epoch,
                resolved_at_ms
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_REVOKE_FUTURE_AUTHORITY_CHECK")?;
    if impossible != 0 {
        bail!("COMPUTE_PLUGIN_VERIFICATION_REVOKE_FUTURE_AUTHORITY_RUN");
    }
    revoke_matching(
        transaction,
        reason,
        resolved_at_ms,
        r#"UPDATE candidate_verification_runs SET
            state = 'revoked', resolved_at_ms = :resolved_at,
            resolution_reason = :reason, result_json = :result_json,
            result_digest = :result_digest
        WHERE state = 'prepared' AND authority_epoch <= :fence"#,
        expected_old_authority_epoch,
    )?;
    require_no_prepared(transaction)
}

pub(super) fn revoke_for_candidate_release(
    transaction: &Transaction<'_>,
    candidate_token: &str,
    resolved_at_ms: i64,
) -> Result<()> {
    let (result_json, result_digest) =
        revocation_result("candidate_released_by_plan", resolved_at_ms)?;
    transaction
        .execute(
            r#"UPDATE candidate_verification_runs SET
                state = 'revoked', resolved_at_ms = :resolved_at,
                resolution_reason = 'candidate_released_by_plan',
                result_json = :result_json, result_digest = :result_digest
            WHERE state = 'prepared' AND candidate_token = :candidate_token"#,
            named_params! {
                ":resolved_at": resolved_at_ms,
                ":result_json": result_json,
                ":result_digest": result_digest,
                ":candidate_token": candidate_token,
            },
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_REVOKE_CANDIDATE")?;
    let remaining = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_verification_runs
            WHERE state = 'prepared' AND candidate_token = ?1"#,
            [candidate_token],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_REVOKE_CANDIDATE_CHECK")?;
    if remaining != 0 {
        bail!("COMPUTE_PLUGIN_VERIFICATION_REVOKE_CANDIDATE_INCOMPLETE");
    }
    Ok(())
}

fn revoke_matching(
    transaction: &Transaction<'_>,
    reason: &str,
    resolved_at_ms: i64,
    statement: &str,
    fence: i64,
) -> Result<()> {
    let (result_json, result_digest) = revocation_result(reason, resolved_at_ms)?;
    transaction
        .execute(
            statement,
            named_params! {
                ":resolved_at": resolved_at_ms,
                ":reason": reason,
                ":result_json": result_json,
                ":result_digest": result_digest,
                ":fence": fence,
            },
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_REVOKE_UPDATE")?;
    Ok(())
}

fn revocation_result(reason: &str, resolved_at_ms: i64) -> Result<(String, String)> {
    encode_candidate_verification_revocation(reason, resolved_at_ms)
}

fn require_no_prepared(transaction: &Transaction<'_>) -> Result<()> {
    let remaining = transaction
        .query_row(
            "SELECT COUNT(*) FROM candidate_verification_runs WHERE state = 'prepared'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_REVOKE_REMAINING_CHECK")?;
    if remaining != 0 {
        bail!("COMPUTE_PLUGIN_VERIFICATION_REVOKE_INCOMPLETE");
    }
    Ok(())
}
