use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{named_params, params, OptionalExtension, Transaction};

use super::{read, ComputePluginFetchAuthorityFacts};
use crate::node_agent_compute_plugin_host::{
    fetch_contract::{
        ValidatedComputePluginFetchAbortPermit, ValidatedComputePluginFetchCommitPermit,
    },
    install_plan_admission_validation::is_identifier,
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

use super::super::{
    keyring_snapshot::{advance_trusted_time, read_authority_keyring_state, AuthorityKeyringState},
    ComputePluginFetchProcessFence,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolutionDownloadRow {
    candidate_token: String,
    part_relative_path: String,
    size_bytes: i64,
    committed_offset: i64,
    cursor_generation: i64,
    state: String,
    updated_at_ms: i64,
}

struct ClaimIdentity<'claim> {
    claim_id: &'claim str,
    plan_id: &'claim str,
    plan_digest: &'claim str,
    part_relative_path: &'claim str,
    ordinal: usize,
    candidate_token_digest: &'claim str,
    authority_epoch: i64,
    process_owner_epoch: i64,
    cursor_generation: i64,
    redirect_generation: i64,
    offset_bytes: i64,
    length_bytes: i64,
    end_offset_bytes: i64,
    prepared_at_ms: i64,
}

pub(super) fn commit_validated_segment(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    roots: &dyn crate::node_agent_compute_plugin_host::keyring::ComputePluginBootstrapRootKeyResolver,
    permit: ValidatedComputePluginFetchCommitPermit<'_>,
) -> Result<()> {
    let claim = ClaimIdentity::from_commit(&permit);
    validate_claim_identity(&claim)?;
    if !is_sha256(permit.file_identity_digest())
        || permit.facts().trusted_now != trusted_now
        || trusted_now.timestamp_millis() <= permit.facts().observed_trusted_time_high_water_ms
    {
        bail!("COMPUTE_PLUGIN_FETCH_COMMIT_PERMIT_INVALID");
    }
    let current = read::read_fresh_segment_authority(
        transaction,
        process_fence,
        trusted_now.clone(),
        roots,
        claim.plan_id,
        claim.plan_digest,
        claim.ordinal,
    )?;
    if &current != permit.facts() || !prepared_claim_matches(&claim, &current) {
        bail!("COMPUTE_PLUGIN_FETCH_COMMIT_AUTHORITY_CAS");
    }
    let download = read_resolution_download(transaction, &claim)?;
    validate_resolution_download(&claim, &download, Some(&current))?;
    advance_commit_trusted_time(transaction, &current, trusted_now.timestamp_millis())?;
    let next_state = if claim.end_offset_bytes == download.size_bytes {
        "complete"
    } else {
        "downloading"
    };
    update_download_for_commit(
        transaction,
        &claim,
        &download,
        next_state,
        trusted_now.timestamp_millis(),
    )?;
    terminalize_claim(
        transaction,
        &claim,
        &download.candidate_token,
        "committed",
        "segment_committed",
        trusted_now.timestamp_millis(),
    )?;
    let post_write = read::read_fresh_segment_authority(
        transaction,
        process_fence,
        trusted_now.clone(),
        roots,
        claim.plan_id,
        claim.plan_digest,
        claim.ordinal,
    )?;
    let expected = expected_commit_post_write(
        &current,
        claim.end_offset_bytes,
        next_state,
        trusted_now.timestamp_millis(),
    );
    if post_write != expected {
        bail!("COMPUTE_PLUGIN_FETCH_COMMIT_POST_WRITE_MISMATCH");
    }
    require_terminal_claim(
        transaction,
        &claim,
        &download.candidate_token,
        "committed",
        "segment_committed",
        trusted_now.timestamp_millis(),
    )
}

pub(super) fn abort_validated_segment(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    permit: ValidatedComputePluginFetchAbortPermit<'_>,
) -> Result<()> {
    let claim = ClaimIdentity::from_abort(&permit);
    validate_claim_identity(&claim)?;
    let download = read_resolution_download(transaction, &claim)?;
    validate_resolution_download(&claim, &download, None)?;
    require_prepared_claim(transaction, &claim, &download.candidate_token)?;
    let state = validate_abort_authority(
        transaction,
        process_fence,
        &claim,
        trusted_now.timestamp_millis(),
    )?;
    advance_trusted_time(transaction, &state, trusted_now.timestamp_millis())?;
    terminalize_claim(
        transaction,
        &claim,
        &download.candidate_token,
        "aborted",
        permit.reason().as_str(),
        trusted_now.timestamp_millis(),
    )?;
    require_terminal_claim(
        transaction,
        &claim,
        &download.candidate_token,
        "aborted",
        permit.reason().as_str(),
        trusted_now.timestamp_millis(),
    )?;
    if read_resolution_download(transaction, &claim)? != download {
        bail!("COMPUTE_PLUGIN_FETCH_ABORT_DOWNLOAD_CHANGED");
    }
    Ok(())
}

impl<'claim> ClaimIdentity<'claim> {
    fn from_commit(permit: &'claim ValidatedComputePluginFetchCommitPermit<'_>) -> Self {
        Self {
            claim_id: permit.claim_id(),
            plan_id: permit.plan_id(),
            plan_digest: permit.plan_digest(),
            part_relative_path: permit.part_relative_path(),
            ordinal: permit.ordinal(),
            candidate_token_digest: permit.candidate_token_digest(),
            authority_epoch: permit.authority_epoch(),
            process_owner_epoch: permit.process_owner_epoch(),
            cursor_generation: permit.cursor_generation(),
            redirect_generation: permit.redirect_generation(),
            offset_bytes: permit.offset_bytes(),
            length_bytes: permit.length_bytes(),
            end_offset_bytes: permit.end_offset_bytes(),
            prepared_at_ms: permit.prepared_at_ms(),
        }
    }

    fn from_abort(permit: &'claim ValidatedComputePluginFetchAbortPermit<'_>) -> Self {
        Self {
            claim_id: permit.claim_id(),
            plan_id: permit.plan_id(),
            plan_digest: permit.plan_digest(),
            part_relative_path: permit.part_relative_path(),
            ordinal: permit.ordinal(),
            candidate_token_digest: permit.candidate_token_digest(),
            authority_epoch: permit.authority_epoch(),
            process_owner_epoch: permit.process_owner_epoch(),
            cursor_generation: permit.cursor_generation(),
            redirect_generation: permit.redirect_generation(),
            offset_bytes: permit.offset_bytes(),
            length_bytes: permit.length_bytes(),
            end_offset_bytes: permit.end_offset_bytes(),
            prepared_at_ms: permit.prepared_at_ms(),
        }
    }
}

fn validate_claim_identity(claim: &ClaimIdentity<'_>) -> Result<()> {
    let expected_end = claim
        .offset_bytes
        .checked_add(claim.length_bytes)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_RESOLVE_RANGE_OVERFLOW"))?;
    if !is_identifier(claim.claim_id)
        || !is_identifier(claim.plan_id)
        || !is_sha256(claim.plan_digest)
        || !is_sha256(claim.candidate_token_digest)
        || !relative_fetch_path_is_valid(claim.part_relative_path)
        || claim.authority_epoch <= 0
        || claim.process_owner_epoch <= 0
        || claim.cursor_generation <= 0
        || claim.redirect_generation < 0
        || claim.redirect_generation > 5
        || claim.offset_bytes < 0
        || claim.length_bytes <= 0
        || claim.end_offset_bytes != expected_end
        || claim.prepared_at_ms < 0
    {
        bail!("COMPUTE_PLUGIN_FETCH_RESOLVE_CLAIM_INVALID");
    }
    Ok(())
}

fn read_resolution_download(
    transaction: &Transaction<'_>,
    claim: &ClaimIdentity<'_>,
) -> Result<ResolutionDownloadRow> {
    let ordinal = i64::try_from(claim.ordinal).context("COMPUTE_PLUGIN_FETCH_RESOLVE_ORDINAL")?;
    transaction
        .query_row(
            r#"SELECT candidate_token, part_relative_path, size_bytes,
                committed_offset, cursor_generation, state, updated_at_ms
            FROM planned_downloads
            WHERE plan_id = ?1 AND plan_digest = ?2 AND ordinal = ?3"#,
            params![claim.plan_id, claim.plan_digest, ordinal],
            |row| {
                Ok(ResolutionDownloadRow {
                    candidate_token: row.get(0)?,
                    part_relative_path: row.get(1)?,
                    size_bytes: row.get(2)?,
                    committed_offset: row.get(3)?,
                    cursor_generation: row.get(4)?,
                    state: row.get(5)?,
                    updated_at_ms: row.get(6)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_FETCH_RESOLVE_DOWNLOAD_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_RESOLVE_DOWNLOAD_MISSING"))
}

fn validate_resolution_download(
    claim: &ClaimIdentity<'_>,
    download: &ResolutionDownloadRow,
    fresh: Option<&ComputePluginFetchAuthorityFacts>,
) -> Result<()> {
    if !is_identifier(&download.candidate_token)
        || jcs_sha256_hex(&download.candidate_token)? != claim.candidate_token_digest
        || download.size_bytes <= 0
        || download.part_relative_path != claim.part_relative_path
        || claim.end_offset_bytes > download.size_bytes
        || download.committed_offset != claim.offset_bytes
        || download.cursor_generation != claim.cursor_generation
        || download.state != "downloading"
        || download.updated_at_ms != claim.prepared_at_ms
        || fresh.is_some_and(|facts| {
            facts.part_relative_path != download.part_relative_path
                || facts.planned_download.size_bytes != download.size_bytes
                || facts.download_updated_at_ms != download.updated_at_ms
        })
    {
        bail!("COMPUTE_PLUGIN_FETCH_RESOLVE_DOWNLOAD_CHANGED");
    }
    Ok(())
}

fn advance_commit_trusted_time(
    transaction: &Transaction<'_>,
    validated: &ComputePluginFetchAuthorityFacts,
    trusted_now_ms: i64,
) -> Result<()> {
    let state = read_authority_keyring_state(transaction)?;
    if state.state_revision != validated.authority_state_revision
        || state.authority_epoch != validated.authority_epoch
        || state.trusted_time_high_water_ms != Some(validated.observed_trusted_time_high_water_ms)
        || state.clock_status != "trusted"
    {
        bail!("COMPUTE_PLUGIN_FETCH_COMMIT_TIME_FENCE_CHANGED");
    }
    advance_trusted_time(transaction, &state, trusted_now_ms)
}

fn validate_abort_authority(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    claim: &ClaimIdentity<'_>,
    trusted_now_ms: i64,
) -> Result<AuthorityKeyringState> {
    let row = transaction
        .query_row(
            r#"SELECT installation_id_digest, state_revision, authority_epoch,
                process_owner_epoch, trusted_time_high_water_ms, clock_status
            FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_FETCH_ABORT_AUTHORITY_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_UNINITIALIZED"))?;
    let high_water = row
        .4
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_ABORT_CLOCK_UNTRUSTED"))?;
    let state = read_authority_keyring_state(transaction)?;
    if !is_sha256(&row.0)
        || row.0 != process_fence.installation_id_digest()
        || row.1 != state.state_revision
        || row.2 != claim.authority_epoch
        || row.2 != state.authority_epoch
        || row.3 != claim.process_owner_epoch
        || row.3 != process_fence.process_owner_epoch()
        || process_fence.acquired_at_ms() < 0
        || process_fence.acquired_at_ms() > high_water
        || trusted_now_ms < high_water
        || state.trusted_time_high_water_ms != Some(high_water)
        || row.5 != "trusted"
        || state.clock_status != row.5
    {
        bail!("COMPUTE_PLUGIN_FETCH_ABORT_AUTHORITY_CHANGED");
    }
    Ok(state)
}

fn update_download_for_commit(
    transaction: &Transaction<'_>,
    claim: &ClaimIdentity<'_>,
    download: &ResolutionDownloadRow,
    next_state: &str,
    trusted_now_ms: i64,
) -> Result<()> {
    let ordinal = i64::try_from(claim.ordinal).context("COMPUTE_PLUGIN_FETCH_RESOLVE_ORDINAL")?;
    let updated = transaction
        .execute(
            r#"UPDATE planned_downloads SET
                committed_offset = :claim_end,
                state = :next_state,
                updated_at_ms = :trusted_now
            WHERE plan_id = :plan_id AND plan_digest = :plan_digest
              AND ordinal = :ordinal AND candidate_token = :candidate_token
              AND part_relative_path = :part_relative_path
              AND size_bytes = :size_bytes
              AND committed_offset = :claim_offset
              AND cursor_generation = :cursor_generation
              AND state = 'downloading'
              AND updated_at_ms = :prepared_at"#,
            named_params! {
                ":claim_end": claim.end_offset_bytes,
                ":next_state": next_state,
                ":trusted_now": trusted_now_ms,
                ":plan_id": claim.plan_id,
                ":plan_digest": claim.plan_digest,
                ":ordinal": ordinal,
                ":candidate_token": &download.candidate_token,
                ":part_relative_path": &download.part_relative_path,
                ":size_bytes": download.size_bytes,
                ":claim_offset": claim.offset_bytes,
                ":cursor_generation": claim.cursor_generation,
                ":prepared_at": claim.prepared_at_ms,
            },
        )
        .context("COMPUTE_PLUGIN_FETCH_COMMIT_DOWNLOAD")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_FETCH_COMMIT_DOWNLOAD_CAS");
    }
    Ok(())
}

fn terminalize_claim(
    transaction: &Transaction<'_>,
    claim: &ClaimIdentity<'_>,
    candidate_token: &str,
    state: &str,
    reason: &str,
    resolved_at_ms: i64,
) -> Result<()> {
    let ordinal = i64::try_from(claim.ordinal).context("COMPUTE_PLUGIN_FETCH_RESOLVE_ORDINAL")?;
    let updated = transaction
        .execute(
            r#"UPDATE fetch_claims SET
                state = :next_state,
                resolved_at_ms = :resolved_at,
                resolution_reason = :reason
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
                ":next_state": state,
                ":resolved_at": resolved_at_ms,
                ":reason": reason,
                ":claim_id": claim.claim_id,
                ":plan_id": claim.plan_id,
                ":plan_digest": claim.plan_digest,
                ":ordinal": ordinal,
                ":candidate_token": candidate_token,
                ":authority_epoch": claim.authority_epoch,
                ":process_owner_epoch": claim.process_owner_epoch,
                ":cursor_generation": claim.cursor_generation,
                ":redirect_generation": claim.redirect_generation,
                ":offset_bytes": claim.offset_bytes,
                ":length_bytes": claim.length_bytes,
                ":end_offset_bytes": claim.end_offset_bytes,
                ":prepared_at": claim.prepared_at_ms,
            },
        )
        .context("COMPUTE_PLUGIN_FETCH_RESOLVE_CLAIM")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_FETCH_RESOLVE_CLAIM_CAS");
    }
    Ok(())
}

fn require_prepared_claim(
    transaction: &Transaction<'_>,
    claim: &ClaimIdentity<'_>,
    candidate_token: &str,
) -> Result<()> {
    require_claim_state(transaction, claim, candidate_token, "prepared", None, None)
}

fn require_terminal_claim(
    transaction: &Transaction<'_>,
    claim: &ClaimIdentity<'_>,
    candidate_token: &str,
    state: &str,
    reason: &str,
    resolved_at_ms: i64,
) -> Result<()> {
    require_claim_state(
        transaction,
        claim,
        candidate_token,
        state,
        Some(reason),
        Some(resolved_at_ms),
    )
}

fn require_claim_state(
    transaction: &Transaction<'_>,
    claim: &ClaimIdentity<'_>,
    candidate_token: &str,
    state: &str,
    reason: Option<&str>,
    resolved_at_ms: Option<i64>,
) -> Result<()> {
    let ordinal = i64::try_from(claim.ordinal).context("COMPUTE_PLUGIN_FETCH_RESOLVE_ORDINAL")?;
    let exists = transaction
        .query_row(
            r#"SELECT EXISTS (
                SELECT 1 FROM fetch_claims
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
                  AND state = :state
                  AND resolved_at_ms IS :resolved_at
                  AND resolution_reason IS :reason
            )"#,
            named_params! {
                ":claim_id": claim.claim_id,
                ":plan_id": claim.plan_id,
                ":plan_digest": claim.plan_digest,
                ":ordinal": ordinal,
                ":candidate_token": candidate_token,
                ":authority_epoch": claim.authority_epoch,
                ":process_owner_epoch": claim.process_owner_epoch,
                ":cursor_generation": claim.cursor_generation,
                ":redirect_generation": claim.redirect_generation,
                ":offset_bytes": claim.offset_bytes,
                ":length_bytes": claim.length_bytes,
                ":end_offset_bytes": claim.end_offset_bytes,
                ":prepared_at": claim.prepared_at_ms,
                ":state": state,
                ":resolved_at": resolved_at_ms,
                ":reason": reason,
            },
            |row| row.get::<_, bool>(0),
        )
        .context("COMPUTE_PLUGIN_FETCH_RESOLVE_CLAIM_READ")?;
    if !exists {
        bail!("COMPUTE_PLUGIN_FETCH_RESOLVE_CLAIM_CHANGED");
    }
    Ok(())
}

fn prepared_claim_matches(
    claim: &ClaimIdentity<'_>,
    facts: &ComputePluginFetchAuthorityFacts,
) -> bool {
    facts.prepared_claim.as_ref().is_some_and(|prepared| {
        prepared.claim_id == claim.claim_id
            && prepared.plan_id == claim.plan_id
            && prepared.plan_digest == claim.plan_digest
            && prepared.ordinal == claim.ordinal
            && prepared.candidate_token_digest == claim.candidate_token_digest
            && prepared.part_relative_path == claim.part_relative_path
            && prepared.authority_epoch == claim.authority_epoch
            && prepared.process_owner_epoch == claim.process_owner_epoch
            && prepared.cursor_generation == claim.cursor_generation
            && prepared.redirect_generation == claim.redirect_generation
            && prepared.offset_bytes == claim.offset_bytes
            && prepared.length_bytes == claim.length_bytes
            && prepared.end_offset_bytes == claim.end_offset_bytes
            && prepared.prepared_at_ms == claim.prepared_at_ms
    })
}

fn expected_commit_post_write(
    validated: &ComputePluginFetchAuthorityFacts,
    committed_offset: i64,
    next_state: &str,
    trusted_now_ms: i64,
) -> ComputePluginFetchAuthorityFacts {
    let mut expected = validated.clone();
    expected.observed_trusted_time_high_water_ms = trusted_now_ms;
    expected.committed_offset = committed_offset;
    expected.download_state = next_state.to_string();
    expected.download_updated_at_ms = trusted_now_ms;
    expected.prepared_claim = None;
    expected
}

fn relative_fetch_path_is_valid(value: &str) -> bool {
    let path = std::path::Path::new(value);
    !value.is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}
