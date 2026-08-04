use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{named_params, params, OptionalExtension, Transaction};

use super::{read, ComputePluginFetchAuthorityFacts, ComputePluginPreparedFetchClaimFacts};
use crate::node_agent_compute_plugin_host::{
    install_plan_admission_validation::is_identifier,
    keyring::ComputePluginBootstrapRootKeyResolver, manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

use super::super::{
    keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
    ComputePluginFetchProcessFence,
};

const MAX_DOWNLOAD_SEGMENT_BYTES: i64 = 16 * 1_024 * 1_024;
const MAX_REDIRECT_GENERATION: i64 = 5;

pub(super) struct FetchClaimCommand<'command> {
    pub plan_id: &'command str,
    pub plan_digest: &'command str,
    pub ordinal: usize,
    pub offset_bytes: i64,
    pub length_bytes: i64,
    pub redirect_generation: i64,
    pub redirect_from_claim_id: Option<&'command str>,
    pub new_claim_id: Option<&'command str>,
}

pub(super) fn claim_validated_segment(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    roots: &dyn ComputePluginBootstrapRootKeyResolver,
    command: &FetchClaimCommand<'_>,
    validated: &ComputePluginFetchAuthorityFacts,
) -> Result<ComputePluginPreparedFetchClaimFacts> {
    validate_command(command, validated, &trusted_now)?;
    // The caller already holds BEGIN IMMEDIATE, so this exhaustive equality is the write CAS:
    // no competing writer can alter authority between this read and the statements below.
    let current = read::read_fresh_segment_authority(
        transaction,
        process_fence,
        trusted_now.clone(),
        roots,
        command.plan_id,
        command.plan_digest,
        command.ordinal,
    )?;
    if &current != validated {
        bail!("COMPUTE_PLUGIN_FETCH_CLAIM_AUTHORITY_CAS");
    }
    advance_claim_trusted_time(transaction, validated, trusted_now.timestamp_millis())?;
    let candidate_token = read_candidate_token(transaction, command, validated)?;
    let prepared = if command.redirect_generation == 0 {
        insert_initial_claim(
            transaction,
            command,
            validated,
            &candidate_token,
            trusted_now.timestamp_millis(),
        )?
    } else {
        advance_redirect_claim(transaction, command, validated, &candidate_token)?
    };
    let post_write = read::read_fresh_segment_authority(
        transaction,
        process_fence,
        trusted_now.clone(),
        roots,
        command.plan_id,
        command.plan_digest,
        command.ordinal,
    )?;
    let expected = expected_post_write(validated, prepared, trusted_now.timestamp_millis())?;
    if post_write != expected {
        bail!("COMPUTE_PLUGIN_FETCH_CLAIM_POST_WRITE_MISMATCH");
    }
    post_write
        .prepared_claim
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_CLAIM_POST_WRITE_MISSING"))
}

fn validate_command(
    command: &FetchClaimCommand<'_>,
    validated: &ComputePluginFetchAuthorityFacts,
    trusted_now: &DateTime<Utc>,
) -> Result<()> {
    let end_offset = command
        .offset_bytes
        .checked_add(command.length_bytes)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_CLAIM_RANGE_OVERFLOW"))?;
    let lineage_is_valid = match (
        command.redirect_generation,
        command.redirect_from_claim_id,
        command.new_claim_id,
        validated.prepared_claim.as_ref(),
    ) {
        (0, None, Some(new_claim_id), None) => is_identifier(new_claim_id),
        (generation, Some(claim_id), None, Some(prepared)) if generation > 0 => {
            prepared.claim_id == claim_id
                && prepared
                    .redirect_generation
                    .checked_add(1)
                    .is_some_and(|next| next == generation)
        }
        _ => false,
    };
    if !is_identifier(command.plan_id)
        || !is_sha256(command.plan_digest)
        || command.plan_id != validated.applied_plan_id
        || command.plan_digest != validated.applied_plan_digest
        || command.offset_bytes < 0
        || command.length_bytes <= 0
        || command.length_bytes > MAX_DOWNLOAD_SEGMENT_BYTES
        || end_offset > validated.planned_download.size_bytes
        || command.redirect_generation < 0
        || command.redirect_generation > MAX_REDIRECT_GENERATION
        || trusted_now != &validated.trusted_now
        || trusted_now.timestamp_millis() < validated.observed_trusted_time_high_water_ms
        || !lineage_is_valid
    {
        bail!("COMPUTE_PLUGIN_FETCH_CLAIM_COMMAND_INVALID");
    }
    Ok(())
}

fn advance_claim_trusted_time(
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
        bail!("COMPUTE_PLUGIN_FETCH_CLAIM_TIME_FENCE_CHANGED");
    }
    advance_trusted_time(transaction, &state, trusted_now_ms)
}

fn read_candidate_token(
    transaction: &Transaction<'_>,
    command: &FetchClaimCommand<'_>,
    validated: &ComputePluginFetchAuthorityFacts,
) -> Result<String> {
    let ordinal = i64::try_from(command.ordinal).context("COMPUTE_PLUGIN_FETCH_CLAIM_ORDINAL")?;
    let token = transaction
        .query_row(
            r#"SELECT candidate_token FROM planned_downloads
            WHERE plan_id = ?1 AND plan_digest = ?2 AND ordinal = ?3"#,
            params![command.plan_id, command.plan_digest, ordinal],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .context("COMPUTE_PLUGIN_FETCH_CLAIM_TOKEN_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_CLAIM_TOKEN_MISSING"))?;
    if !is_identifier(&token) || jcs_sha256_hex(&token)? != validated.candidate_token_digest {
        bail!("COMPUTE_PLUGIN_FETCH_CLAIM_TOKEN_CHANGED");
    }
    Ok(token)
}

fn insert_initial_claim(
    transaction: &Transaction<'_>,
    command: &FetchClaimCommand<'_>,
    validated: &ComputePluginFetchAuthorityFacts,
    candidate_token: &str,
    trusted_now_ms: i64,
) -> Result<ComputePluginPreparedFetchClaimFacts> {
    let claim_id = command
        .new_claim_id
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_CLAIM_ID_MISSING"))?;
    let ordinal = i64::try_from(command.ordinal).context("COMPUTE_PLUGIN_FETCH_CLAIM_ORDINAL")?;
    let cursor_generation = validated
        .download_cursor_generation
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_CURSOR_EXHAUSTED"))?;
    let end_offset_bytes = command
        .offset_bytes
        .checked_add(command.length_bytes)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_CLAIM_RANGE_OVERFLOW"))?;
    let inserted = transaction
        .execute(
            r#"INSERT INTO fetch_claims (
                claim_id, plan_id, plan_digest, ordinal, candidate_token,
                authority_epoch, process_owner_epoch, cursor_generation,
                redirect_generation, offset_bytes, length_bytes, end_offset_bytes,
                state, prepared_at_ms, resolved_at_ms, resolution_reason
            ) VALUES (
                :claim_id, :plan_id, :plan_digest, :ordinal, :candidate_token,
                :authority_epoch, :process_owner_epoch, :cursor_generation,
                0, :offset_bytes, :length_bytes, :end_offset_bytes,
                'prepared', :trusted_now, NULL, NULL
            )"#,
            named_params! {
                ":claim_id": claim_id,
                ":plan_id": command.plan_id,
                ":plan_digest": command.plan_digest,
                ":ordinal": ordinal,
                ":candidate_token": candidate_token,
                ":authority_epoch": validated.authority_epoch,
                ":process_owner_epoch": validated.process_owner_epoch,
                ":cursor_generation": cursor_generation,
                ":offset_bytes": command.offset_bytes,
                ":length_bytes": command.length_bytes,
                ":end_offset_bytes": end_offset_bytes,
                ":trusted_now": trusted_now_ms,
            },
        )
        .context("COMPUTE_PLUGIN_FETCH_CLAIM_INSERT")?;
    let updated = transaction
        .execute(
            r#"UPDATE planned_downloads SET
                cursor_generation = :next_cursor,
                state = 'downloading',
                updated_at_ms = :trusted_now
            WHERE plan_id = :plan_id AND plan_digest = :plan_digest
              AND ordinal = :ordinal AND candidate_token = :candidate_token
              AND committed_offset = :offset_bytes
              AND cursor_generation = :old_cursor
              AND state = :old_state
              AND updated_at_ms = :old_updated_at"#,
            named_params! {
                ":next_cursor": cursor_generation,
                ":trusted_now": trusted_now_ms,
                ":plan_id": command.plan_id,
                ":plan_digest": command.plan_digest,
                ":ordinal": ordinal,
                ":candidate_token": candidate_token,
                ":offset_bytes": command.offset_bytes,
                ":old_cursor": validated.download_cursor_generation,
                ":old_state": &validated.download_state,
                ":old_updated_at": validated.download_updated_at_ms,
            },
        )
        .context("COMPUTE_PLUGIN_FETCH_DOWNLOAD_BEGIN")?;
    if inserted != 1 || updated != 1 {
        bail!("COMPUTE_PLUGIN_FETCH_CLAIM_BEGIN_CAS");
    }
    Ok(ComputePluginPreparedFetchClaimFacts {
        claim_id: claim_id.to_string(),
        plan_id: command.plan_id.to_string(),
        plan_digest: command.plan_digest.to_string(),
        ordinal: command.ordinal,
        candidate_token_digest: validated.candidate_token_digest.clone(),
        part_relative_path: validated.part_relative_path.clone(),
        authority_epoch: validated.authority_epoch,
        process_owner_epoch: validated.process_owner_epoch,
        cursor_generation,
        redirect_generation: 0,
        offset_bytes: command.offset_bytes,
        length_bytes: command.length_bytes,
        end_offset_bytes,
        prepared_at_ms: trusted_now_ms,
    })
}

fn advance_redirect_claim(
    transaction: &Transaction<'_>,
    command: &FetchClaimCommand<'_>,
    validated: &ComputePluginFetchAuthorityFacts,
    candidate_token: &str,
) -> Result<ComputePluginPreparedFetchClaimFacts> {
    let prepared = validated
        .prepared_claim
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_REDIRECT_CLAIM_MISSING"))?;
    let ordinal = i64::try_from(command.ordinal).context("COMPUTE_PLUGIN_FETCH_CLAIM_ORDINAL")?;
    let updated = transaction
        .execute(
            r#"UPDATE fetch_claims SET redirect_generation = :next_redirect
            WHERE claim_id = :claim_id AND plan_id = :plan_id
              AND plan_digest = :plan_digest AND ordinal = :ordinal
              AND candidate_token = :candidate_token
              AND authority_epoch = :authority_epoch
              AND process_owner_epoch = :process_owner_epoch
              AND cursor_generation = :cursor_generation
              AND redirect_generation = :old_redirect
              AND offset_bytes = :offset_bytes
              AND length_bytes = :length_bytes
              AND end_offset_bytes = :end_offset_bytes
              AND state = 'prepared' AND prepared_at_ms = :prepared_at
              AND resolved_at_ms IS NULL AND resolution_reason IS NULL"#,
            named_params! {
                ":next_redirect": command.redirect_generation,
                ":claim_id": &prepared.claim_id,
                ":plan_id": command.plan_id,
                ":plan_digest": command.plan_digest,
                ":ordinal": ordinal,
                ":candidate_token": candidate_token,
                ":authority_epoch": prepared.authority_epoch,
                ":process_owner_epoch": prepared.process_owner_epoch,
                ":cursor_generation": prepared.cursor_generation,
                ":old_redirect": prepared.redirect_generation,
                ":offset_bytes": prepared.offset_bytes,
                ":length_bytes": prepared.length_bytes,
                ":end_offset_bytes": prepared.end_offset_bytes,
                ":prepared_at": prepared.prepared_at_ms,
            },
        )
        .context("COMPUTE_PLUGIN_FETCH_REDIRECT_UPDATE")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_FETCH_REDIRECT_CAS");
    }
    let mut advanced = prepared.clone();
    advanced.redirect_generation = command.redirect_generation;
    Ok(advanced)
}

fn expected_post_write(
    validated: &ComputePluginFetchAuthorityFacts,
    prepared: ComputePluginPreparedFetchClaimFacts,
    trusted_now_ms: i64,
) -> Result<ComputePluginFetchAuthorityFacts> {
    let mut expected = validated.clone();
    expected.observed_trusted_time_high_water_ms = trusted_now_ms;
    if prepared.redirect_generation == 0 {
        expected.download_cursor_generation = prepared.cursor_generation;
        expected.download_state = "downloading".to_string();
        expected.download_updated_at_ms = trusted_now_ms;
    } else if expected.prepared_claim.is_none() {
        bail!("COMPUTE_PLUGIN_FETCH_REDIRECT_EXPECTATION_MISSING");
    }
    expected.prepared_claim = Some(prepared);
    Ok(expected)
}
