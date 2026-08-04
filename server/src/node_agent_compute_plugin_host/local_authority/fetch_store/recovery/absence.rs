use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{RecoveryAuthorityRow, RecoveryDownloadRow};
use crate::node_agent_compute_plugin_host::{
    fetch_contract::ComputePluginFetchClaimRecoveryKey,
    install_plan_admission_validation::is_identifier, signed_artifact_verification::jcs_sha256_hex,
};

/// Separates a genuinely missing identity from an identity that exists with different binding.
/// The latter must continue through the normal reader and fail closed on its exact comparisons.
pub(super) fn exact_claim_exists(
    transaction: &Transaction<'_>,
    key: &ComputePluginFetchClaimRecoveryKey,
) -> Result<bool> {
    let exists = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM fetch_claims WHERE claim_id = ?1)",
            [key.claim_id()],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_FETCH_OUTCOME_CLAIM_EXISTENCE_READ")?;
    match exists {
        0 => Ok(false),
        1 => Ok(true),
        _ => bail!("COMPUTE_PLUGIN_FETCH_OUTCOME_CLAIM_EXISTENCE_CORRUPT"),
    }
}

/// Classifies an initial claim as not created only when every authority and download fact is still
/// the exact pre-mutation snapshot and no competing claim occupies the expected cursor.
pub(super) fn read_not_created_download(
    transaction: &Transaction<'_>,
    authority: &RecoveryAuthorityRow,
    key: &ComputePluginFetchClaimRecoveryKey,
) -> Result<RecoveryDownloadRow> {
    let before = key
        .initial_absence()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_OUTCOME_CLAIM_MISSING"))?;
    let expected_cursor = before
        .download_cursor_generation()
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_OUTCOME_CURSOR_EXHAUSTED"))?;
    if before.expected_redirect_generation() != 0
        || key.observed_redirect_generation() != 0
        || before.authority_state_revision() <= 0
        || before.trusted_time_high_water_ms() < 0
        || before.trusted_time_high_water_ms() > key.prepared_at_ms()
        || before.download_committed_offset() != key.offset_bytes()
        || before.download_cursor_generation() < 0
        || expected_cursor != key.cursor_generation()
        || !matches!(
            before.download_state(),
            "pending" | "downloading" | "failed"
        )
        || before.download_updated_at_ms() < 0
        || before.download_updated_at_ms() > before.trusted_time_high_water_ms()
        || authority.state_revision != before.authority_state_revision()
        || authority.authority_epoch != key.authority_epoch()
        || authority.process_owner_epoch != key.process_owner_epoch()
        || authority.trusted_time_high_water_ms != before.trusted_time_high_water_ms()
        || authority.clock_status != "trusted"
    {
        bail!("COMPUTE_PLUGIN_FETCH_OUTCOME_NOT_CREATED_AUTHORITY_CHANGED");
    }

    type Row = (String, String, String, i64, i64, i64, String, i64);
    let row: Row = transaction
        .query_row(
            r#"SELECT candidate_token, artifact_digest, part_relative_path, size_bytes,
                committed_offset, cursor_generation, state, updated_at_ms
            FROM planned_downloads
            WHERE plan_id = ?1 AND plan_digest = ?2 AND ordinal = ?3"#,
            params![
                key.plan_id(),
                key.plan_digest(),
                i64::try_from(key.ordinal()).context("COMPUTE_PLUGIN_FETCH_OUTCOME_ORDINAL")?,
            ],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_FETCH_OUTCOME_NOT_CREATED_DOWNLOAD_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_OUTCOME_IDENTITY_CHANGED"))?;
    if !is_identifier(&row.0)
        || jcs_sha256_hex(&row.0)? != key.candidate_token_digest()
        || row.1 != key.artifact_digest()
        || row.2 != key.part_relative_path()
        || row.3 != key.artifact_size_bytes()
        || row.3 <= 0
        || row.4 != before.download_committed_offset()
        || row.5 != before.download_cursor_generation()
        || row.6 != before.download_state()
        || row.7 != before.download_updated_at_ms()
    {
        bail!("COMPUTE_PLUGIN_FETCH_OUTCOME_NOT_CREATED_DOWNLOAD_CHANGED");
    }

    let conflicting_claims = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM fetch_claims
            WHERE plan_id = ?1 AND plan_digest = ?2 AND ordinal = ?3
              AND (cursor_generation >= ?4 OR state = 'prepared')"#,
            params![
                key.plan_id(),
                key.plan_digest(),
                i64::try_from(key.ordinal()).context("COMPUTE_PLUGIN_FETCH_OUTCOME_ORDINAL")?,
                key.cursor_generation(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_FETCH_OUTCOME_NOT_CREATED_CONFLICT_READ")?;
    if conflicting_claims != 0 {
        bail!("COMPUTE_PLUGIN_FETCH_OUTCOME_NOT_CREATED_CLAIM_CONFLICT");
    }

    Ok(RecoveryDownloadRow {
        artifact_digest: row.1,
        part_relative_path: row.2,
        size_bytes: row.3,
        committed_offset: row.4,
        cursor_generation: row.5,
        state: row.6,
        updated_at_ms: row.7,
    })
}
