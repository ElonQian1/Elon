use anyhow::{bail, Context, Result};
use rusqlite::{named_params, Transaction};

use super::{closure, outcome};
use crate::node_agent_compute_plugin_host::{
    candidate_verification_contract::{
        ComputePluginCandidateVerificationOutcome, ComputePluginCandidateVerificationOutcomeKind,
        ValidatedCandidateVerificationRecoveryAbortPermit,
    },
    candidate_verification_terminal_result::encode_candidate_verification_abort,
};

use super::super::{
    keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
    ComputePluginFetchProcessFence,
};

const RECOVERY_ABORT_REASON: &str = "authority_recovery";

pub(super) fn abort_recovered_candidate_verification(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    trusted_now_ms: i64,
    permit: ValidatedCandidateVerificationRecoveryAbortPermit<'_>,
) -> Result<ComputePluginCandidateVerificationOutcome> {
    let key = permit.key();
    let before = outcome::read_outcome_snapshot(transaction, process_fence, key)?;
    if &before.outcome != permit.observed()
        || before.outcome.kind() != ComputePluginCandidateVerificationOutcomeKind::Prepared
        || before.run.is_none()
        || before.closure.is_none()
        || before.authority.clock_status != "trusted"
        || trusted_now_ms <= before.authority.trusted_time_high_water_ms
        || trusted_now_ms <= key.prepared_at_ms()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_ABORT_CHANGED");
    }
    let state = read_authority_keyring_state(transaction)?;
    if state.state_revision != before.authority.state_revision
        || state.authority_epoch != before.authority.authority_epoch
        || state.trusted_time_high_water_ms != Some(before.authority.trusted_time_high_water_ms)
        || state.clock_status != "trusted"
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_ABORT_TIME_CHANGED");
    }
    let (result_json, result_digest) =
        encode_candidate_verification_abort(RECOVERY_ABORT_REASON, trusted_now_ms)?;
    advance_trusted_time(transaction, &state, trusted_now_ms)?;
    terminalize_prepared_run(
        transaction,
        key,
        trusted_now_ms,
        &result_json,
        &result_digest,
    )?;
    let after = outcome::read_outcome_snapshot(transaction, process_fence, key)?;
    let expected = ComputePluginCandidateVerificationOutcome::from_store(
        ComputePluginCandidateVerificationOutcomeKind::Aborted,
        key,
        Some(trusted_now_ms),
        Some(RECOVERY_ABORT_REASON),
        Some(result_digest),
    );
    let mut expected_authority = before.authority.clone();
    expected_authority.trusted_time_high_water_ms = trusted_now_ms;
    expected_authority.clock_status = "trusted".to_string();
    expected_authority.updated_at_ms = trusted_now_ms;
    let closure_after =
        closure::read_candidate_closure_snapshot(transaction, key.candidate_token())?;
    if after.outcome != expected
        || after.authority != expected_authority
        || after.run.is_none()
        || after.closure.is_some()
        || Some(&closure_after) != before.closure.as_ref()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_ABORT_POST_WRITE_MISMATCH");
    }
    Ok(after.outcome)
}

fn terminalize_prepared_run(
    transaction: &Transaction<'_>,
    key: &crate::node_agent_compute_plugin_host::candidate_verification_contract::ComputePluginCandidateVerificationRecoveryKey,
    resolved_at_ms: i64,
    result_json: &str,
    result_digest: &str,
) -> Result<()> {
    let updated = transaction
        .execute(
            r#"UPDATE candidate_verification_runs SET
                state = 'aborted', resolved_at_ms = :resolved_at,
                resolution_reason = 'authority_recovery',
                result_json = :result_json, result_digest = :result_digest
            WHERE verification_id = :verification_id
              AND candidate_token = :candidate_token
              AND owner_plan_id = :owner_plan_id
              AND owner_plan_digest = :owner_plan_digest
              AND verification_generation = :verification_generation
              AND candidate_generation = :candidate_generation
              AND application_inventory_revision = :application_inventory_revision
              AND authority_state_revision = :authority_state_revision
              AND authority_epoch = :authority_epoch
              AND process_owner_epoch = :process_owner_epoch
              AND artifact_count = :artifact_count
              AND artifact_bytes = :artifact_bytes
              AND expected_artifact_set_digest = :expected_artifact_set_digest
              AND file_set_binding_digest = :file_set_binding_digest
              AND prepared_at_ms = :prepared_at_ms
              AND state = 'prepared'
              AND resolved_at_ms IS NULL AND resolution_reason IS NULL
              AND result_json IS NULL AND result_digest IS NULL
              AND observed_artifact_set_digest IS NULL
              AND mismatch_ordinal IS NULL AND mismatch_observed_digest IS NULL"#,
            named_params! {
                ":resolved_at": resolved_at_ms,
                ":result_json": result_json,
                ":result_digest": result_digest,
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
                ":artifact_count": i64::try_from(key.artifact_count())
                    .context("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_ABORT_ARTIFACT_COUNT")?,
                ":artifact_bytes": key.artifact_bytes(),
                ":expected_artifact_set_digest": key.expected_artifact_set_digest(),
                ":file_set_binding_digest": key.file_set_binding_digest(),
                ":prepared_at_ms": key.prepared_at_ms(),
            },
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_ABORT_WRITE")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_ABORT_CAS");
    }
    Ok(())
}
