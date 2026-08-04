use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::{VerificationRecoveryAuthorityRow, VerificationRunRow};
use crate::node_agent_compute_plugin_host::{
    candidate_verification_contract::{
        ComputePluginCandidateVerificationDigestMismatch,
        ComputePluginCandidateVerificationOutcome, ComputePluginCandidateVerificationRecoveryKey,
    },
    candidate_verification_terminal_result::{
        parse_candidate_verification_resolution, CandidateVerificationResolutionInput,
        CandidateVerificationResolutionKind,
    },
    identity::ComputePluginReleaseRef,
    lifecycle::{
        local_record_shape_is_valid, ComputePluginInventorySnapshot,
        COMPUTE_PLUGIN_INVENTORY_SCHEMA,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn classify_resolution(
    transaction: &Transaction<'_>,
    authority: &VerificationRecoveryAuthorityRow,
    run: &VerificationRunRow,
    key: &ComputePluginCandidateVerificationRecoveryKey,
    expected_kind: CandidateVerificationResolutionKind,
) -> Result<ComputePluginCandidateVerificationOutcome> {
    let resolved_at_ms = run
        .resolved_at_ms
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RESOLUTION_MISSING"))?;
    let reason = run
        .resolution_reason
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_REASON_MISSING"))?;
    let result_json = run
        .result_json
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RESULT_MISSING"))?;
    let result_digest = run
        .result_digest
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_DIGEST_MISSING"))?;
    let input = parse_candidate_verification_resolution(result_json, result_digest)?;
    validate_resolution_binding(
        authority,
        run,
        key,
        expected_kind,
        &input,
        resolved_at_ms,
        reason,
    )?;
    validate_current_inventory_if_unchanged(transaction, authority, key, &input)?;

    match expected_kind {
        CandidateVerificationResolutionKind::Verified => Ok(
            ComputePluginCandidateVerificationOutcome::verified_from_store(
                key,
                resolved_at_ms,
                result_digest.to_string(),
                input.observed_artifact_set_digest,
                input.authority_state_revision_after,
                input.inventory_revision_after,
                input.inventory_digest_after,
                input.authority_epoch_after,
                input.slot_phase_after,
            ),
        ),
        CandidateVerificationResolutionKind::Rejected => {
            let mismatch = input.mismatch.ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_MISMATCH_MISSING")
            })?;
            validate_expected_mismatch(transaction, key, &mismatch)?;
            Ok(
                ComputePluginCandidateVerificationOutcome::rejected_from_store(
                    key,
                    resolved_at_ms,
                    result_digest.to_string(),
                    input.observed_artifact_set_digest,
                    ComputePluginCandidateVerificationDigestMismatch::from_store(
                        usize::try_from(mismatch.ordinal)
                            .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_MISMATCH_ORDINAL")?,
                        mismatch.expected_digest,
                        mismatch.observed_digest,
                    ),
                    input.authority_state_revision_after,
                    input.inventory_revision_after,
                    input.inventory_digest_after,
                    input.authority_epoch_after,
                    input.slot_phase_after,
                ),
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_resolution_binding(
    authority: &VerificationRecoveryAuthorityRow,
    run: &VerificationRunRow,
    key: &ComputePluginCandidateVerificationRecoveryKey,
    expected_kind: CandidateVerificationResolutionKind,
    input: &CandidateVerificationResolutionInput,
    resolved_at_ms: i64,
    reason: &str,
) -> Result<()> {
    let columns_match = match expected_kind {
        CandidateVerificationResolutionKind::Verified => {
            run.mismatch_ordinal.is_none() && run.mismatch_observed_digest.is_none()
        }
        CandidateVerificationResolutionKind::Rejected => {
            input.mismatch.as_ref().is_some_and(|mismatch| {
                run.mismatch_ordinal == Some(mismatch.ordinal)
                    && run.mismatch_observed_digest.as_deref()
                        == Some(mismatch.observed_digest.as_str())
            })
        }
    };
    if input.kind != expected_kind
        || run.state != input.state()
        || reason != input.reason()
        || resolved_at_ms != input.resolved_at_ms
        || resolved_at_ms < key.prepared_at_ms()
        || resolved_at_ms > authority.trusted_time_high_water_ms
        || input.verification_id != key.verification_id()
        || input.candidate_token_digest != key.candidate_token_digest()
        || input.owner_plan_id != key.owner_plan_id()
        || input.owner_plan_digest != key.owner_plan_digest()
        || input.verification_generation != key.verification_generation()
        || input.candidate_generation != key.candidate_generation()
        || input.prepared_at_ms != key.prepared_at_ms()
        || usize::try_from(input.artifact_count).ok() != Some(key.artifact_count())
        || input.artifact_bytes != key.artifact_bytes()
        || input.expected_artifact_set_digest != key.expected_artifact_set_digest()
        || input.file_set_binding_digest != key.file_set_binding_digest()
        || input.authority_state_revision_before != key.authority_state_revision()
        || input.inventory_revision_before != key.execution_inventory_revision()
        || input.inventory_digest_before != key.inventory_digest()
        || input.authority_epoch_before != key.authority_epoch()
        || authority.state_revision < input.authority_state_revision_after
        || authority.inventory_revision < input.inventory_revision_after
        || authority.authority_epoch < input.authority_epoch_after
        || run.observed_artifact_set_digest.as_deref()
            != Some(input.observed_artifact_set_digest.as_str())
        || !is_sha256(&input.observed_artifact_set_digest)
        || !columns_match
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RESOLUTION_CORRUPT");
    }
    Ok(())
}

fn validate_current_inventory_if_unchanged(
    transaction: &Transaction<'_>,
    authority: &VerificationRecoveryAuthorityRow,
    key: &ComputePluginCandidateVerificationRecoveryKey,
    input: &CandidateVerificationResolutionInput,
) -> Result<()> {
    if authority.inventory_revision != input.inventory_revision_after {
        return Ok(());
    }
    let inventory: ComputePluginInventorySnapshot = serde_json::from_str(&authority.inventory_json)
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_INVENTORY_JSON")?;
    let (plugin_id, slot_ref, release_json, candidate_state, closed_at_ms) = transaction
        .query_row(
            r#"SELECT plugin_id, slot_ref, release_json, state, closed_at_ms
            FROM candidate_owners WHERE candidate_token = ?1"#,
            [key.candidate_token()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                ))
            },
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_CANDIDATE_READ")?;
    let release: ComputePluginReleaseRef = serde_json::from_str(&release_json)
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RELEASE_JSON")?;
    let matches = inventory
        .plugins
        .iter()
        .filter(|record| {
            record.plugin_id == plugin_id
                && record.candidate_slot_ref.as_deref() == Some(slot_ref.as_str())
                && record.slots.iter().any(|slot| {
                    slot.slot_ref == slot_ref
                        && slot.release == release
                        && slot.phase == input.slot_phase_after
                })
        })
        .count();
    if authority.inventory_digest != input.inventory_digest_after
        || inventory.schema != COMPUTE_PLUGIN_INVENTORY_SCHEMA
        || inventory.inventory_revision != input.inventory_revision_after
        || inventory
            .plugins
            .windows(2)
            .any(|pair| pair[0].plugin_id >= pair[1].plugin_id)
        || inventory
            .plugins
            .iter()
            .any(|record| !local_record_shape_is_valid(record))
        || jcs_sha256_hex(&inventory)? != authority.inventory_digest
        || candidate_state != "owned"
        || closed_at_ms.is_some()
        || matches != 1
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RESOLUTION_INVENTORY_CHANGED");
    }
    Ok(())
}

fn validate_expected_mismatch(
    transaction: &Transaction<'_>,
    key: &ComputePluginCandidateVerificationRecoveryKey,
    mismatch: &crate::node_agent_compute_plugin_host::candidate_verification_terminal_result::CandidateVerificationDigestMismatch,
) -> Result<()> {
    let (count, expected) = transaction
        .query_row(
            r#"SELECT COUNT(*), MIN(artifact_digest) FROM planned_downloads
            WHERE candidate_token = ?1 AND ordinal = ?2"#,
            params![key.candidate_token(), mismatch.ordinal],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_MISMATCH_READ")?;
    if count != 1 || expected.as_deref() != Some(mismatch.expected_digest.as_str()) {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_MISMATCH_CHANGED");
    }
    Ok(())
}
