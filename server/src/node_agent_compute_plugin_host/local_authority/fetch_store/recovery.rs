use anyhow::{bail, Context, Result};
use rusqlite::{named_params, OptionalExtension, Transaction};

use super::ComputePluginFetchAuthoritySession;
use crate::node_agent_compute_plugin_host::{
    fetch_contract::{
        ComputePluginFetchClaimOutcome, ComputePluginFetchClaimOutcomeKind,
        ComputePluginFetchClaimRecoveryKey, ValidatedComputePluginFetchRecoveryAbortPermit,
    },
    install_plan_admission_validation::is_identifier,
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

use super::super::{
    keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
    ComputePluginFetchProcessFence,
};

const RECOVERY_ABORT_REASON: &str = "authority_recovery";

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryAuthorityRow {
    installation_id_digest: String,
    state_revision: i64,
    authority_epoch: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms: i64,
    clock_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryClaimRow {
    candidate_token: String,
    redirect_generation: i64,
    state: String,
    resolved_at_ms: Option<i64>,
    resolution_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryDownloadRow {
    artifact_digest: String,
    part_relative_path: String,
    size_bytes: i64,
    committed_offset: i64,
    cursor_generation: i64,
    state: String,
    updated_at_ms: i64,
}

struct RecoverySnapshot {
    authority: RecoveryAuthorityRow,
    claim: RecoveryClaimRow,
    download: RecoveryDownloadRow,
    outcome: ComputePluginFetchClaimOutcome,
}

impl ComputePluginFetchAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_installation_id_digest(
        &self,
    ) -> &str {
        self.process_fence.installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn read_claim_outcome(
        &self,
        key: &ComputePluginFetchClaimRecoveryKey,
    ) -> Result<ComputePluginFetchClaimOutcome> {
        self.authority.with_deferred(|transaction| {
            Ok(read_outcome_snapshot(transaction, self.process_fence, key)?.outcome)
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn abort_recovered_prepared_claim(
        &self,
        permit: ValidatedComputePluginFetchRecoveryAbortPermit<'_>,
    ) -> Result<ComputePluginFetchClaimOutcome> {
        self.authority.with_immediate(|transaction| {
            abort_recovered_prepared_claim(
                transaction,
                self.process_fence,
                self.trusted_now.timestamp_millis(),
                permit,
            )
        })
    }
}

fn read_outcome_snapshot(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    key: &ComputePluginFetchClaimRecoveryKey,
) -> Result<RecoverySnapshot> {
    validate_recovery_key(key)?;
    let authority = read_recovery_authority(transaction)?;
    validate_reader(&authority, process_fence, key)?;
    let (claim, download) = read_claim_and_download(transaction, key)?;
    validate_claim_and_download(&authority, key, &claim, &download)?;
    let (kind, reason) = classify_claim(&authority, key, &claim, &download)?;
    let outcome = ComputePluginFetchClaimOutcome::from_store(
        kind,
        key.ordinal(),
        claim.redirect_generation,
        authority.authority_epoch,
        authority.process_owner_epoch,
        download.cursor_generation,
        download.committed_offset,
        download.state.clone(),
        claim.resolved_at_ms,
        reason,
    );
    Ok(RecoverySnapshot {
        authority,
        claim,
        download,
        outcome,
    })
}

fn validate_recovery_key(key: &ComputePluginFetchClaimRecoveryKey) -> Result<()> {
    let expected_end = key
        .offset_bytes()
        .checked_add(key.length_bytes())
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_OUTCOME_RANGE_OVERFLOW"))?;
    if !is_sha256(key.installation_id_digest())
        || !is_identifier(key.claim_id())
        || !is_identifier(key.plan_id())
        || !is_sha256(key.plan_digest())
        || !is_sha256(key.candidate_token_digest())
        || !is_sha256(key.artifact_digest())
        || !relative_fetch_path_is_valid(key.part_relative_path())
        || key.artifact_size_bytes() <= 0
        || key.authority_epoch() <= 0
        || key.process_owner_epoch() <= 0
        || key.cursor_generation() <= 0
        || key.observed_redirect_generation() < 0
        || key.observed_redirect_generation() > 5
        || key.offset_bytes() < 0
        || key.length_bytes() <= 0
        || key.end_offset_bytes() != expected_end
        || key.end_offset_bytes() > key.artifact_size_bytes()
        || key.prepared_at_ms() < 0
    {
        bail!("COMPUTE_PLUGIN_FETCH_OUTCOME_KEY_INVALID");
    }
    Ok(())
}

fn read_recovery_authority(transaction: &Transaction<'_>) -> Result<RecoveryAuthorityRow> {
    transaction
        .query_row(
            r#"SELECT installation_id_digest, state_revision, authority_epoch,
                process_owner_epoch, trusted_time_high_water_ms, clock_status
            FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| {
                let high_water = row.get::<_, Option<i64>>(4)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    high_water,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_FETCH_OUTCOME_AUTHORITY_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_UNINITIALIZED"))
        .and_then(|row| {
            Ok(RecoveryAuthorityRow {
                installation_id_digest: row.0,
                state_revision: row.1,
                authority_epoch: row.2,
                process_owner_epoch: row.3,
                trusted_time_high_water_ms: row
                    .4
                    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_OUTCOME_TIME_MISSING"))?,
                clock_status: row.5,
            })
        })
}

fn validate_reader(
    authority: &RecoveryAuthorityRow,
    process_fence: &ComputePluginFetchProcessFence,
    key: &ComputePluginFetchClaimRecoveryKey,
) -> Result<()> {
    if !is_sha256(&authority.installation_id_digest)
        || authority.installation_id_digest != key.installation_id_digest()
        || authority.installation_id_digest != process_fence.installation_id_digest()
        || authority.state_revision < 0
        || authority.authority_epoch < key.authority_epoch()
        || authority.process_owner_epoch < key.process_owner_epoch()
        || authority.process_owner_epoch != process_fence.process_owner_epoch()
        || process_fence.acquired_at_ms() < 0
        || process_fence.acquired_at_ms() > authority.trusted_time_high_water_ms
        || authority.trusted_time_high_water_ms < key.prepared_at_ms()
        || !matches!(
            authority.clock_status.as_str(),
            "trusted" | "clock_untrusted"
        )
    {
        bail!("COMPUTE_PLUGIN_FETCH_OUTCOME_READER_CHANGED");
    }
    Ok(())
}

fn read_claim_and_download(
    transaction: &Transaction<'_>,
    key: &ComputePluginFetchClaimRecoveryKey,
) -> Result<(RecoveryClaimRow, RecoveryDownloadRow)> {
    type Row = (
        String,
        String,
        String,
        i64,
        String,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        String,
        i64,
        Option<i64>,
        Option<String>,
        String,
        String,
        i64,
        i64,
        i64,
        String,
        i64,
    );
    let row: Row = transaction
        .query_row(
            r#"SELECT claim.claim_id, claim.plan_id, claim.plan_digest, claim.ordinal,
                claim.candidate_token, claim.authority_epoch, claim.process_owner_epoch,
                claim.cursor_generation, claim.redirect_generation, claim.offset_bytes,
                claim.length_bytes, claim.end_offset_bytes, claim.state, claim.prepared_at_ms,
                claim.resolved_at_ms, claim.resolution_reason, download.artifact_digest,
                download.part_relative_path, download.size_bytes, download.committed_offset,
                download.cursor_generation, download.state, download.updated_at_ms
            FROM fetch_claims AS claim
            JOIN planned_downloads AS download
              ON download.plan_id = claim.plan_id
             AND download.plan_digest = claim.plan_digest
             AND download.ordinal = claim.ordinal
             AND download.candidate_token = claim.candidate_token
            WHERE claim.claim_id = ?1"#,
            [key.claim_id()],
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
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                    row.get(16)?,
                    row.get(17)?,
                    row.get(18)?,
                    row.get(19)?,
                    row.get(20)?,
                    row.get(21)?,
                    row.get(22)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_FETCH_OUTCOME_CLAIM_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_OUTCOME_IDENTITY_CHANGED"))?;
    let ordinal = usize::try_from(row.3).context("COMPUTE_PLUGIN_FETCH_OUTCOME_ORDINAL")?;
    if row.0 != key.claim_id()
        || row.1 != key.plan_id()
        || row.2 != key.plan_digest()
        || ordinal != key.ordinal()
        || !is_identifier(&row.4)
        || jcs_sha256_hex(&row.4)? != key.candidate_token_digest()
        || row.5 != key.authority_epoch()
        || row.6 != key.process_owner_epoch()
        || row.7 != key.cursor_generation()
        || row.8 < key.observed_redirect_generation()
        || row.8 > 5
        || row.9 != key.offset_bytes()
        || row.10 != key.length_bytes()
        || row.11 != key.end_offset_bytes()
        || row.13 != key.prepared_at_ms()
    {
        bail!("COMPUTE_PLUGIN_FETCH_OUTCOME_IDENTITY_CHANGED");
    }
    Ok((
        RecoveryClaimRow {
            candidate_token: row.4,
            redirect_generation: row.8,
            state: row.12,
            resolved_at_ms: row.14,
            resolution_reason: row.15,
        },
        RecoveryDownloadRow {
            artifact_digest: row.16,
            part_relative_path: row.17,
            size_bytes: row.18,
            committed_offset: row.19,
            cursor_generation: row.20,
            state: row.21,
            updated_at_ms: row.22,
        },
    ))
}

fn validate_claim_and_download(
    authority: &RecoveryAuthorityRow,
    key: &ComputePluginFetchClaimRecoveryKey,
    claim: &RecoveryClaimRow,
    download: &RecoveryDownloadRow,
) -> Result<()> {
    if download.artifact_digest != key.artifact_digest()
        || download.part_relative_path != key.part_relative_path()
        || download.size_bytes != key.artifact_size_bytes()
        || download.committed_offset < 0
        || download.committed_offset > download.size_bytes
        || download.cursor_generation < 0
        || download.updated_at_ms < key.prepared_at_ms()
        || !matches!(
            download.state.as_str(),
            "pending" | "downloading" | "complete" | "canceled" | "failed"
        )
        || claim.redirect_generation < 0
        || claim.resolved_at_ms.is_some_and(|resolved| {
            resolved < key.prepared_at_ms() || resolved > authority.trusted_time_high_water_ms
        })
    {
        bail!("COMPUTE_PLUGIN_FETCH_OUTCOME_STATE_CORRUPT");
    }
    Ok(())
}

fn classify_claim(
    authority: &RecoveryAuthorityRow,
    key: &ComputePluginFetchClaimRecoveryKey,
    claim: &RecoveryClaimRow,
    download: &RecoveryDownloadRow,
) -> Result<(ComputePluginFetchClaimOutcomeKind, Option<&'static str>)> {
    let (kind, reason) = classify_terminal_fields(claim)?;
    let progress_valid = match kind {
        ComputePluginFetchClaimOutcomeKind::Prepared => {
            authority.authority_epoch == key.authority_epoch()
                && authority.process_owner_epoch == key.process_owner_epoch()
                && download.cursor_generation == key.cursor_generation()
                && download.committed_offset == key.offset_bytes()
                && download.state == "downloading"
                && download.updated_at_ms == key.prepared_at_ms()
        }
        ComputePluginFetchClaimOutcomeKind::Committed => {
            download.cursor_generation >= key.cursor_generation()
                && download.committed_offset >= key.end_offset_bytes()
                && (download.cursor_generation != key.cursor_generation()
                    || download.committed_offset == key.end_offset_bytes())
        }
        ComputePluginFetchClaimOutcomeKind::Aborted
        | ComputePluginFetchClaimOutcomeKind::Revoked => {
            download.cursor_generation >= key.cursor_generation()
                && download.committed_offset >= key.offset_bytes()
                && (download.cursor_generation != key.cursor_generation()
                    || download.committed_offset == key.offset_bytes())
        }
    };
    if !progress_valid {
        bail!("COMPUTE_PLUGIN_FETCH_OUTCOME_PROGRESS_CORRUPT");
    }
    Ok((kind, reason))
}

fn classify_terminal_fields(
    claim: &RecoveryClaimRow,
) -> Result<(ComputePluginFetchClaimOutcomeKind, Option<&'static str>)> {
    let classified = match (
        claim.state.as_str(),
        claim.resolved_at_ms,
        claim.resolution_reason.as_deref(),
    ) {
        ("prepared", None, None) => (ComputePluginFetchClaimOutcomeKind::Prepared, None),
        ("committed", Some(_), Some("segment_committed")) => (
            ComputePluginFetchClaimOutcomeKind::Committed,
            Some("segment_committed"),
        ),
        ("aborted", Some(_), Some(reason)) => (
            ComputePluginFetchClaimOutcomeKind::Aborted,
            Some(parse_abort_reason(reason)?),
        ),
        ("revoked", Some(_), Some(reason)) => (
            ComputePluginFetchClaimOutcomeKind::Revoked,
            Some(parse_revocation_reason(reason)?),
        ),
        _ => bail!("COMPUTE_PLUGIN_FETCH_OUTCOME_TERMINAL_CORRUPT"),
    };
    Ok(classified)
}

fn parse_abort_reason(reason: &str) -> Result<&'static str> {
    match reason {
        "downloader_canceled" => Ok("downloader_canceled"),
        "transport_failed" => Ok("transport_failed"),
        "durable_write_failed" => Ok("durable_write_failed"),
        "file_binding_mismatch" => Ok("file_binding_mismatch"),
        "authority_recovery" => Ok(RECOVERY_ABORT_REASON),
        _ => bail!("COMPUTE_PLUGIN_FETCH_OUTCOME_ABORT_REASON_CORRUPT"),
    }
}

fn parse_revocation_reason(reason: &str) -> Result<&'static str> {
    match reason {
        "authority_epoch_advanced_by_keyring" => Ok("authority_epoch_advanced_by_keyring"),
        "authority_epoch_advanced_by_plan" => Ok("authority_epoch_advanced_by_plan"),
        "process_owner_epoch_advanced" => Ok("process_owner_epoch_advanced"),
        "candidate_released_by_plan" => Ok("candidate_released_by_plan"),
        _ => bail!("COMPUTE_PLUGIN_FETCH_OUTCOME_REVOKE_REASON_CORRUPT"),
    }
}

fn abort_recovered_prepared_claim(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    trusted_now_ms: i64,
    permit: ValidatedComputePluginFetchRecoveryAbortPermit<'_>,
) -> Result<ComputePluginFetchClaimOutcome> {
    let before = read_outcome_snapshot(transaction, process_fence, permit.key())?;
    if &before.outcome != permit.observed()
        || before.outcome.kind() != ComputePluginFetchClaimOutcomeKind::Prepared
        || trusted_now_ms <= before.authority.trusted_time_high_water_ms
        || trusted_now_ms <= permit.key().prepared_at_ms()
        || before.authority.clock_status != "trusted"
    {
        bail!("COMPUTE_PLUGIN_FETCH_RECOVERY_ABORT_CHANGED");
    }
    let state = read_authority_keyring_state(transaction)?;
    if state.state_revision != before.authority.state_revision
        || state.authority_epoch != before.authority.authority_epoch
        || state.trusted_time_high_water_ms != Some(before.authority.trusted_time_high_water_ms)
        || state.clock_status != "trusted"
    {
        bail!("COMPUTE_PLUGIN_FETCH_RECOVERY_TIME_CHANGED");
    }
    advance_trusted_time(transaction, &state, trusted_now_ms)?;
    terminalize_recovered_claim(transaction, permit.key(), &before.claim, trusted_now_ms)?;
    let after = read_outcome_snapshot(transaction, process_fence, permit.key())?;
    let expected = ComputePluginFetchClaimOutcome::from_store(
        ComputePluginFetchClaimOutcomeKind::Aborted,
        permit.key().ordinal(),
        before.claim.redirect_generation,
        before.authority.authority_epoch,
        before.authority.process_owner_epoch,
        before.download.cursor_generation,
        before.download.committed_offset,
        before.download.state.clone(),
        Some(trusted_now_ms),
        Some(RECOVERY_ABORT_REASON),
    );
    let mut expected_authority = before.authority.clone();
    expected_authority.trusted_time_high_water_ms = trusted_now_ms;
    expected_authority.clock_status = "trusted".to_string();
    if after.outcome != expected
        || after.download != before.download
        || after.authority != expected_authority
    {
        bail!("COMPUTE_PLUGIN_FETCH_RECOVERY_ABORT_POST_WRITE_MISMATCH");
    }
    Ok(after.outcome)
}

fn terminalize_recovered_claim(
    transaction: &Transaction<'_>,
    key: &ComputePluginFetchClaimRecoveryKey,
    claim: &RecoveryClaimRow,
    resolved_at_ms: i64,
) -> Result<()> {
    let ordinal = i64::try_from(key.ordinal()).context("COMPUTE_PLUGIN_FETCH_OUTCOME_ORDINAL")?;
    let updated = transaction
        .execute(
            r#"UPDATE fetch_claims SET state = 'aborted',
                resolved_at_ms = :resolved_at,
                resolution_reason = 'authority_recovery'
            WHERE claim_id = :claim_id AND plan_id = :plan_id
              AND plan_digest = :plan_digest AND ordinal = :ordinal
              AND candidate_token = :candidate_token
              AND authority_epoch = :authority_epoch
              AND process_owner_epoch = :process_owner_epoch
              AND cursor_generation = :cursor_generation
              AND redirect_generation = :redirect_generation
              AND offset_bytes = :offset_bytes
              AND length_bytes = :length_bytes
              AND end_offset_bytes = :end_offset_bytes
              AND prepared_at_ms = :prepared_at
              AND state = 'prepared'
              AND resolved_at_ms IS NULL AND resolution_reason IS NULL"#,
            named_params! {
                ":resolved_at": resolved_at_ms,
                ":claim_id": key.claim_id(),
                ":plan_id": key.plan_id(),
                ":plan_digest": key.plan_digest(),
                ":ordinal": ordinal,
                ":candidate_token": &claim.candidate_token,
                ":authority_epoch": key.authority_epoch(),
                ":process_owner_epoch": key.process_owner_epoch(),
                ":cursor_generation": key.cursor_generation(),
                ":redirect_generation": claim.redirect_generation,
                ":offset_bytes": key.offset_bytes(),
                ":length_bytes": key.length_bytes(),
                ":end_offset_bytes": key.end_offset_bytes(),
                ":prepared_at": key.prepared_at_ms(),
            },
        )
        .context("COMPUTE_PLUGIN_FETCH_RECOVERY_ABORT_WRITE")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_FETCH_RECOVERY_ABORT_CAS");
    }
    Ok(())
}

fn relative_fetch_path_is_valid(value: &str) -> bool {
    let path = std::path::Path::new(value);
    !value.is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}
