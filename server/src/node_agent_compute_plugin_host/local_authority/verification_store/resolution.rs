use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{named_params, Transaction};

use super::{closure, outcome, read, ComputePluginCandidateVerificationAuthorityFacts};
use crate::node_agent_compute_plugin_host::{
    candidate_verification_contract::{
        CandidateArtifactSetHashDisposition, ComputePluginCandidateVerificationOutcome,
        ComputePluginCandidateVerificationOutcomeKind,
        ComputePluginCandidateVerificationRecoveryKey,
        ValidatedCandidateVerificationResolutionPermit,
    },
    candidate_verification_terminal_result::{
        encode_candidate_verification_resolution, CandidateVerificationDigestMismatch,
        CandidateVerificationResolutionInput, CandidateVerificationResolutionKind,
    },
    install_plan_admission::validate_inventory,
    keyring::ComputePluginBootstrapRootKeyResolver,
    lifecycle::{
        is_valid_slot_transition, ComputePluginInventorySnapshot, SLOT_DOWNLOADING, SLOT_FAILED,
        SLOT_VERIFYING,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

use super::super::{
    fetch_claim_revocation::revoke_for_verification_authority_epoch_advance,
    keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
    plan_application::{read_authority_plan_application_state, AuthorityPlanApplicationState},
    ComputePluginFetchProcessFence,
};

mod meta;

struct ResolutionProjection {
    inventory: ComputePluginInventorySnapshot,
    inventory_json: String,
    inventory_digest: String,
    state_revision: i64,
    authority_epoch: i64,
    slot_phase: &'static str,
}

pub(super) fn resolve_candidate_verification(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    roots: &dyn ComputePluginBootstrapRootKeyResolver,
    permit: ValidatedCandidateVerificationResolutionPermit<'_>,
) -> Result<ComputePluginCandidateVerificationOutcome> {
    let key = permit.key();
    if !permit.prepared().matches_recovery_key(key) {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_PREPARED_CHANGED");
    }
    let before = outcome::read_outcome_snapshot(transaction, process_fence, key)?;
    validate_s3(&before, &permit)?;

    let current = read::read_fresh_prepared_candidate_verification_authority(
        transaction,
        process_fence,
        trusted_now.clone(),
        roots,
        key,
    )?;
    validate_s4(&current, &permit, trusted_now.timestamp_millis())?;
    let authority = read_authority_plan_application_state(transaction, &trusted_now)?;
    validate_authority_alignment(&authority, &current)?;

    let projection = project_inventory(&authority, &current, permit.disposition(), &trusted_now)?;
    let input = resolution_input(&permit, &projection, trusted_now.timestamp_millis())?;
    let (result_json, result_digest) = encode_candidate_verification_resolution(&input)?;
    permit.cancellation_guard().ensure_current()?;

    let time_state = read_authority_keyring_state(transaction)?;
    if time_state.state_revision != authority.state_revision
        || time_state.authority_epoch != authority.authority_epoch
        || time_state.trusted_time_high_water_ms != Some(authority.trusted_time_high_water_ms)
        || time_state.clock_status != "trusted"
        || trusted_now.timestamp_millis() <= authority.trusted_time_high_water_ms
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_TIME_CHANGED");
    }
    advance_trusted_time(transaction, &time_state, trusted_now.timestamp_millis())?;
    permit.cancellation_guard().ensure_current()?;
    terminalize_target(transaction, key, &input, &result_json, &result_digest)?;
    revoke_for_verification_authority_epoch_advance(
        transaction,
        authority.authority_epoch,
        projection.authority_epoch,
        trusted_now.timestamp_millis(),
    )?;
    permit.cancellation_guard().ensure_current()?;
    meta::update_resolution_authority_meta(
        transaction,
        &authority,
        &projection,
        trusted_now.timestamp_millis(),
    )?;

    let after = outcome::read_outcome_snapshot(transaction, process_fence, key)?;
    let authority_after = read_authority_plan_application_state(transaction, &trusted_now)?;
    let closure_after =
        closure::read_candidate_closure_snapshot(transaction, key.candidate_token())?;
    validate_readback(
        transaction,
        &after,
        &authority_after,
        &closure_after,
        &projection,
        &input,
        &result_digest,
        key,
    )?;
    Ok(after.outcome)
}

fn validate_s3(
    before: &outcome::VerificationOutcomeSnapshot,
    permit: &ValidatedCandidateVerificationResolutionPermit<'_>,
) -> Result<()> {
    let key = permit.key();
    let s3 = permit.s3_binding();
    let closure = before.closure.as_ref().ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_S3_CLOSURE_MISSING")
    })?;
    if before.outcome.kind() != ComputePluginCandidateVerificationOutcomeKind::Prepared
        || before.outcome != s3.outcome
        || s3.authority_state_revision != key.authority_state_revision()
        || s3.inventory_revision != key.execution_inventory_revision()
        || s3.inventory_digest != key.inventory_digest()
        || s3.authority_epoch != key.authority_epoch()
        || s3.process_owner_epoch != key.process_owner_epoch()
        || s3.durable_candidate_closure_digest != key.durable_candidate_closure_digest()
        || before.authority.state_revision != s3.authority_state_revision
        || before.authority.inventory_revision != s3.inventory_revision
        || before.authority.inventory_digest != s3.inventory_digest
        || before.authority.authority_epoch != s3.authority_epoch
        || before.authority.process_owner_epoch != s3.process_owner_epoch
        || before.authority.trusted_time_high_water_ms != s3.trusted_time_high_water_ms
        || closure.durable_closure_digest != s3.durable_candidate_closure_digest
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_S3_CHANGED");
    }
    Ok(())
}

fn validate_s4(
    current: &ComputePluginCandidateVerificationAuthorityFacts,
    permit: &ValidatedCandidateVerificationResolutionPermit<'_>,
    trusted_now_ms: i64,
) -> Result<()> {
    let key = permit.key();
    let s3 = permit.s3_binding();
    if current.installation_id_digest != key.installation_id_digest()
        || current.applied_plan_id != key.owner_plan_id()
        || current.applied_plan_digest != key.owner_plan_digest()
        || current.candidate_owner_plan_id != key.owner_plan_id()
        || current.candidate_owner_plan_digest != key.owner_plan_digest()
        || current.next_verification_generation != key.verification_generation()
        || current.candidate_generation != key.candidate_generation()
        || current.candidate_application_inventory_revision != key.application_inventory_revision()
        || current.authority_state_revision != s3.authority_state_revision
        || current.execution_inventory_revision != s3.inventory_revision
        || current.inventory_digest != s3.inventory_digest
        || current.authority_epoch != s3.authority_epoch
        || current.process_owner_epoch != s3.process_owner_epoch
        || current.observed_trusted_time_high_water_ms != s3.trusted_time_high_water_ms
        || current.candidate_token_digest != key.candidate_token_digest()
        || current.artifacts.len() != key.artifact_count()
        || current.artifact_bytes != key.artifact_bytes()
        || current.expected_artifact_set_digest != key.expected_artifact_set_digest()
        || current.recompute_durable_candidate_closure_digest()?
            != key.durable_candidate_closure_digest()
        || current.trusted_now.timestamp_millis() != trusted_now_ms
        || trusted_now_ms <= s3.trusted_time_high_water_ms
        || trusted_now_ms <= key.prepared_at_ms()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_S4_CHANGED");
    }
    validate_disposition(current, permit)
}

fn validate_disposition(
    current: &ComputePluginCandidateVerificationAuthorityFacts,
    permit: &ValidatedCandidateVerificationResolutionPermit<'_>,
) -> Result<()> {
    match permit.disposition() {
        CandidateArtifactSetHashDisposition::Matched
            if permit.mismatch_ordinal().is_none()
                && permit.mismatch_expected_digest().is_none()
                && permit.mismatch_observed_digest().is_none() =>
        {
            Ok(())
        }
        CandidateArtifactSetHashDisposition::DigestMismatch => {
            let ordinal = permit.mismatch_ordinal().ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_MISMATCH_MISSING")
            })?;
            let expected = permit.mismatch_expected_digest().ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_MISMATCH_MISSING")
            })?;
            let observed = permit.mismatch_observed_digest().ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_MISMATCH_MISSING")
            })?;
            let artifact = current
                .artifacts
                .iter()
                .find(|artifact| artifact.ordinal == ordinal)
                .ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_MISMATCH_CHANGED")
                })?;
            if artifact.planned_download.digest != expected || expected == observed {
                bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_MISMATCH_CHANGED");
            }
            Ok(())
        }
        _ => bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_DISPOSITION_CHANGED"),
    }
}

fn validate_authority_alignment(
    authority: &AuthorityPlanApplicationState,
    current: &ComputePluginCandidateVerificationAuthorityFacts,
) -> Result<()> {
    if authority.installation_id_digest != current.installation_id_digest
        || authority.state_revision != current.authority_state_revision
        || authority.inventory != current.inventory
        || authority.inventory.inventory_revision != current.execution_inventory_revision
        || authority.inventory_digest != current.inventory_digest
        || authority.authority_epoch != current.authority_epoch
        || authority.process_owner_epoch != current.process_owner_epoch
        || authority.trusted_time_high_water_ms != current.observed_trusted_time_high_water_ms
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_AUTHORITY_CHANGED");
    }
    Ok(())
}

fn project_inventory(
    authority: &AuthorityPlanApplicationState,
    current: &ComputePluginCandidateVerificationAuthorityFacts,
    disposition: CandidateArtifactSetHashDisposition,
    trusted_now: &DateTime<Utc>,
) -> Result<ResolutionProjection> {
    let slot_phase = match disposition {
        CandidateArtifactSetHashDisposition::Matched => SLOT_VERIFYING,
        CandidateArtifactSetHashDisposition::DigestMismatch => SLOT_FAILED,
    };
    if !is_valid_slot_transition(Some(SLOT_DOWNLOADING), Some(slot_phase)) {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_SLOT_TRANSITION_INVALID");
    }
    let mut inventory = authority.inventory.clone();
    inventory.inventory_revision = inventory
        .inventory_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_INVENTORY_EXHAUSTED"))?;
    let observed_at = trusted_now.to_rfc3339_opts(SecondsFormat::Millis, true);
    inventory.observed_at = observed_at.clone();
    let mut records = inventory.plugins.iter_mut().filter(|record| {
        record.plugin_id == current.candidate_plugin_id
            && record.candidate_slot_ref.as_deref() == Some(current.candidate_slot_ref.as_str())
    });
    let record = records
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_RECORD_MISSING"))?;
    if records.next().is_some() {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_RECORD_DUPLICATED");
    }
    let mut slots = record.slots.iter_mut().filter(|slot| {
        slot.slot_ref == current.candidate_slot_ref
            && slot.release == current.candidate_release
            && slot.phase == SLOT_DOWNLOADING
    });
    let slot = slots
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_SLOT_MISSING"))?;
    if slots.next().is_some() {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_SLOT_DUPLICATED");
    }
    slot.phase = slot_phase.to_string();
    slot.phase_changed_at = observed_at.clone();
    record.state_changed_at = observed_at;
    validate_inventory(&inventory, trusted_now.clone())?;
    let inventory_json = serde_json::to_string(&inventory)
        .context("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_INVENTORY_JSON")?;
    let inventory_digest = jcs_sha256_hex(&inventory)?;
    Ok(ResolutionProjection {
        inventory,
        inventory_json,
        inventory_digest,
        state_revision: authority
            .state_revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_STATE_EXHAUSTED"))?,
        authority_epoch: authority
            .authority_epoch
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_EPOCH_EXHAUSTED"))?,
        slot_phase,
    })
}

fn resolution_input(
    permit: &ValidatedCandidateVerificationResolutionPermit<'_>,
    projection: &ResolutionProjection,
    resolved_at_ms: i64,
) -> Result<CandidateVerificationResolutionInput> {
    let key = permit.key();
    let mismatch = match permit.disposition() {
        CandidateArtifactSetHashDisposition::Matched => None,
        CandidateArtifactSetHashDisposition::DigestMismatch => {
            Some(CandidateVerificationDigestMismatch {
                ordinal: i64::try_from(permit.mismatch_ordinal().ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_MISMATCH_MISSING")
                })?)
                .context("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_MISMATCH_ORDINAL")?,
                expected_digest: permit
                    .mismatch_expected_digest()
                    .ok_or_else(|| {
                        anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_MISMATCH_MISSING")
                    })?
                    .to_string(),
                observed_digest: permit
                    .mismatch_observed_digest()
                    .ok_or_else(|| {
                        anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_MISMATCH_MISSING")
                    })?
                    .to_string(),
            })
        }
    };
    Ok(CandidateVerificationResolutionInput {
        kind: match permit.disposition() {
            CandidateArtifactSetHashDisposition::Matched => {
                CandidateVerificationResolutionKind::Verified
            }
            CandidateArtifactSetHashDisposition::DigestMismatch => {
                CandidateVerificationResolutionKind::Rejected
            }
        },
        verification_id: key.verification_id().to_string(),
        candidate_token_digest: key.candidate_token_digest().to_string(),
        owner_plan_id: key.owner_plan_id().to_string(),
        owner_plan_digest: key.owner_plan_digest().to_string(),
        verification_generation: key.verification_generation(),
        candidate_generation: key.candidate_generation(),
        prepared_at_ms: key.prepared_at_ms(),
        resolved_at_ms,
        artifact_count: i64::try_from(key.artifact_count())
            .context("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_ARTIFACT_COUNT")?,
        artifact_bytes: key.artifact_bytes(),
        expected_artifact_set_digest: key.expected_artifact_set_digest().to_string(),
        observed_artifact_set_digest: permit.observed_artifact_set_digest().to_string(),
        file_set_binding_digest: key.file_set_binding_digest().to_string(),
        mismatch,
        authority_state_revision_before: key.authority_state_revision(),
        authority_state_revision_after: projection.state_revision,
        inventory_revision_before: key.execution_inventory_revision(),
        inventory_revision_after: projection.inventory.inventory_revision,
        inventory_digest_before: key.inventory_digest().to_string(),
        inventory_digest_after: projection.inventory_digest.clone(),
        authority_epoch_before: key.authority_epoch(),
        authority_epoch_after: projection.authority_epoch,
        slot_phase_after: projection.slot_phase.to_string(),
    })
}

fn terminalize_target(
    transaction: &Transaction<'_>,
    key: &ComputePluginCandidateVerificationRecoveryKey,
    input: &CandidateVerificationResolutionInput,
    result_json: &str,
    result_digest: &str,
) -> Result<()> {
    let updated = transaction
        .execute(
            r#"UPDATE candidate_verification_runs SET
                state = :state, resolved_at_ms = :resolved_at,
                resolution_reason = :reason, result_json = :result_json,
                result_digest = :result_digest,
                observed_artifact_set_digest = :observed_artifact_set_digest,
                mismatch_ordinal = :mismatch_ordinal,
                mismatch_observed_digest = :mismatch_observed_digest
            WHERE verification_id = :verification_id
              AND candidate_token = :candidate_token
              AND owner_plan_id = :owner_plan_id AND owner_plan_digest = :owner_plan_digest
              AND verification_generation = :verification_generation
              AND candidate_generation = :candidate_generation
              AND application_inventory_revision = :application_inventory_revision
              AND authority_state_revision = :authority_state_revision
              AND authority_epoch = :authority_epoch
              AND process_owner_epoch = :process_owner_epoch
              AND artifact_count = :artifact_count AND artifact_bytes = :artifact_bytes
              AND expected_artifact_set_digest = :expected_artifact_set_digest
              AND file_set_binding_digest = :file_set_binding_digest
              AND prepared_at_ms = :prepared_at_ms AND state = 'prepared'
              AND resolved_at_ms IS NULL AND resolution_reason IS NULL
              AND result_json IS NULL AND result_digest IS NULL
              AND observed_artifact_set_digest IS NULL
              AND mismatch_ordinal IS NULL AND mismatch_observed_digest IS NULL"#,
            named_params! {
                ":state": input.state(),
                ":resolved_at": input.resolved_at_ms,
                ":reason": input.reason(),
                ":result_json": result_json,
                ":result_digest": result_digest,
                ":observed_artifact_set_digest": &input.observed_artifact_set_digest,
                ":mismatch_ordinal": input.mismatch_ordinal(),
                ":mismatch_observed_digest": input.mismatch_observed_digest(),
                ":verification_id": key.verification_id(),
                ":candidate_token": key.candidate_token(),
                ":owner_plan_id": key.owner_plan_id(),
                ":owner_plan_digest": key.owner_plan_digest(),
                ":verification_generation": key.verification_generation(),
                ":candidate_generation": key.candidate_generation(),
                ":application_inventory_revision": key.application_inventory_revision(),
                ":authority_state_revision": key.authority_state_revision(),
                ":authority_epoch": key.authority_epoch(),
                ":process_owner_epoch": key.process_owner_epoch(),
                ":artifact_count": input.artifact_count,
                ":artifact_bytes": key.artifact_bytes(),
                ":expected_artifact_set_digest": key.expected_artifact_set_digest(),
                ":file_set_binding_digest": key.file_set_binding_digest(),
                ":prepared_at_ms": key.prepared_at_ms(),
            },
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_WRITE")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_CAS");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_readback(
    transaction: &Transaction<'_>,
    after: &outcome::VerificationOutcomeSnapshot,
    authority: &AuthorityPlanApplicationState,
    closure_after: &closure::CandidateClosureSnapshot,
    projection: &ResolutionProjection,
    input: &CandidateVerificationResolutionInput,
    result_digest: &str,
    key: &ComputePluginCandidateVerificationRecoveryKey,
) -> Result<()> {
    let expected_kind = match input.kind {
        CandidateVerificationResolutionKind::Verified => {
            ComputePluginCandidateVerificationOutcomeKind::Verified
        }
        CandidateVerificationResolutionKind::Rejected => {
            ComputePluginCandidateVerificationOutcomeKind::Rejected
        }
    };
    let prepared = transaction.query_row(
        "SELECT (SELECT COUNT(*) FROM fetch_claims WHERE state = 'prepared') + (SELECT COUNT(*) FROM candidate_verification_runs WHERE state = 'prepared')",
        [],
        |row| row.get::<_, i64>(0),
    ).context("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_PREPARED_READBACK")?;
    if after.outcome.kind() != expected_kind
        || after.outcome.resolved_at_ms() != Some(input.resolved_at_ms)
        || after.outcome.result_digest() != Some(result_digest)
        || after.outcome.observed_artifact_set_digest()
            != Some(input.observed_artifact_set_digest.as_str())
        || after.outcome.authority_state_revision_after() != Some(projection.state_revision)
        || after.outcome.inventory_revision_after() != Some(projection.inventory.inventory_revision)
        || after.outcome.inventory_digest_after() != Some(projection.inventory_digest.as_str())
        || after.outcome.authority_epoch_after() != Some(projection.authority_epoch)
        || after.outcome.slot_phase_after() != Some(projection.slot_phase)
        || authority.state_revision != projection.state_revision
        || authority.inventory != projection.inventory
        || authority.inventory_digest != projection.inventory_digest
        || authority.authority_epoch != projection.authority_epoch
        || authority.process_owner_epoch != key.process_owner_epoch()
        || authority.trusted_time_high_water_ms != input.resolved_at_ms
        || closure_after.durable_closure_digest != key.durable_candidate_closure_digest()
        || closure_after.expected_artifact_set_digest != key.expected_artifact_set_digest()
        || closure_after.artifact_count != key.artifact_count()
        || closure_after.artifact_bytes != key.artifact_bytes()
        || prepared != 0
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_READBACK_CHANGED");
    }
    Ok(())
}
