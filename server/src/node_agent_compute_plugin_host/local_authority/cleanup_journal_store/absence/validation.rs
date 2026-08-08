use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::ComputePluginCandidateCleanupParentAbsenceAuthoritySession;
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        build_parent_namespace_absence_event, validate_hashed_cleanup_step_event,
        HashedComputePluginCandidateCleanupStepEvent, ObservedCandidateCleanupParentAbsence,
    },
    local_authority::{
        cleanup_journal_store::validation::{
            count_event_identity_matches, count_events, read_exact_step_event,
        },
        cleanup_store::binding::validate_failed_candidate_inventory,
        cleanup_topology_store::read_exact_sealed_plan,
        plan_application::read_authority_plan_application_state_at_or_before_observation,
    },
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn validate_unstored_parent_absence(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupParentAbsenceAuthoritySession<'_>,
    observed: &ObservedCandidateCleanupParentAbsence,
    event: &HashedComputePluginCandidateCleanupStepEvent,
) -> Result<()> {
    validate_hashed_cleanup_step_event(event)?;
    let expected = build_parent_namespace_absence_event(
        observed.plan(),
        observed.intent_event(),
        observed.disposition_event(),
        observed.object_binding(),
        session.trusted_now_ms(),
    )?;
    if expected != *event || observed.state().completed_step_count() != 0 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_BINDING_CHANGED");
    }
    validate_authority_and_owner(
        transaction,
        session,
        observed,
        observed.disposition_event().event().recorded_at_ms(),
    )?;
    let stored_plan = read_exact_sealed_plan(
        transaction,
        observed.plan(),
        observed.state().staging_recovery_key().candidate_token(),
    )?;
    let stored_intent = read_exact_step_event(transaction, observed.intent_event())?;
    let stored_disposition = read_exact_step_event(transaction, observed.disposition_event())?;
    if stored_plan.as_ref() != Some(observed.plan())
        || stored_intent.as_ref() != Some(observed.intent_event())
        || stored_disposition.as_ref() != Some(observed.disposition_event())
        || count_event_identity_matches(transaction, observed.intent_event())? != 1
        || count_event_identity_matches(transaction, observed.disposition_event())? != 1
        || count_event_identity_matches(transaction, event)? != 0
        || count_events(transaction, observed.plan().plan().cleanup_id())? != 2
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_PREDECESSOR_CHANGED");
    }
    Ok(())
}

pub(super) fn validate_authority_and_owner(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupParentAbsenceAuthoritySession<'_>,
    observed: &ObservedCandidateCleanupParentAbsence,
    expected_high_water_ms: i64,
) -> Result<()> {
    let state = observed.state();
    let authorization = state.authorization_receipt();
    let receipt = authorization.receipt();
    let plan = observed.plan().plan();
    let authority = read_authority_plan_application_state_at_or_before_observation(
        transaction,
        &session.trusted_now,
    )?;
    let slot = state.staging_recovery_key().slot_expectation();
    validate_failed_candidate_inventory(
        &authority.inventory,
        &slot.plugin_id,
        &slot.slot_ref,
        &slot.release,
    )?;
    if authority.installation_id_digest != session.installation_id_digest()
        || authority.process_owner_epoch != session.process_owner_epoch()
        || authority.state_revision < receipt.authority_state_revision_after()
        || authority.inventory.inventory_revision < receipt.inventory_revision()
        || authority.authority_epoch < receipt.authority_epoch_after()
        || authority.trusted_time_high_water_ms < expected_high_water_ms
        || plan.cleanup_id() != receipt.cleanup_id()
        || plan.candidate_token_digest() != receipt.candidate_token_digest()
        || plan.authorization_receipt_digest() != authorization.receipt_digest()
        || plan.installation_id_digest() != session.installation_id_digest()
        || receipt.process_owner_epoch() != session.process_owner_epoch()
        || plan.process_owner_epoch() != session.process_owner_epoch()
        || plan.planned_at_ms() <= receipt.authorized_at_ms()
        || count_exact_authorization(transaction, observed)? != 1
        || count_exact_pending_owner(transaction, observed)? != 1
        || count_exact_authority_time(transaction, session, expected_high_water_ms)? != 1
        || super::super::recovery::count_completion(
            transaction,
            state.staging_recovery_key().candidate_token(),
        )? != 0
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_AUTHORITY_CHANGED");
    }
    Ok(())
}

fn count_exact_authorization(
    transaction: &Transaction<'_>,
    observed: &ObservedCandidateCleanupParentAbsence,
) -> Result<i64> {
    let state = observed.state();
    let authorization = state.authorization_receipt();
    let receipt = authorization.receipt();
    let receipt_json = serde_json::to_string(receipt)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_AUTHORIZATION_SERIALIZE")?;
    if jcs_sha256_hex(receipt)? != authorization.receipt_digest() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_AUTHORIZATION_CHANGED");
    }
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_authorizations
               WHERE cleanup_id = ?1 AND candidate_token = ?2
                 AND candidate_token_digest = ?3 AND quarantine_id = ?4
                 AND quarantine_receipt_digest = ?5 AND staging_id = ?6
                 AND staging_run_digest = ?7
                 AND authority_state_revision_before = ?8
                 AND authority_state_revision_after = ?9
                 AND inventory_revision = ?10 AND inventory_digest = ?11
                 AND authority_epoch_before = ?12 AND authority_epoch_after = ?13
                 AND process_owner_epoch = ?14
                 AND trusted_time_high_water_ms_before = ?15
                 AND authorized_at_ms = ?16 AND slot_phase_before = 'failed'
                 AND receipt_json = ?17 AND receipt_digest = ?18"#,
            params![
                receipt.cleanup_id(),
                state.staging_recovery_key().candidate_token(),
                receipt.candidate_token_digest(),
                receipt.quarantine_id(),
                receipt.quarantine_receipt_digest(),
                receipt.staging_id(),
                receipt.staging_run_digest(),
                receipt.authority_state_revision_before(),
                receipt.authority_state_revision_after(),
                receipt.inventory_revision(),
                receipt.inventory_digest(),
                receipt.authority_epoch_before(),
                receipt.authority_epoch_after(),
                receipt.process_owner_epoch(),
                receipt.trusted_time_high_water_ms_before(),
                receipt.authorized_at_ms(),
                receipt_json,
                authorization.receipt_digest(),
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_AUTHORIZATION_READ")
}

fn count_exact_pending_owner(
    transaction: &Transaction<'_>,
    observed: &ObservedCandidateCleanupParentAbsence,
) -> Result<i64> {
    let staging = observed.state().staging_recovery_key();
    let slot = staging.slot_expectation();
    let expected = staging.receipt_expectation();
    let release_json = serde_json::to_string(&slot.release)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_OWNER_RELEASE_SERIALIZE")?;
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_owners
               WHERE candidate_token = ?1 AND plugin_id = ?2 AND slot_ref = ?3
                 AND candidate_generation = ?4 AND release_json = ?5
                 AND owner_plan_id = ?6 AND owner_plan_digest = ?7
                 AND application_inventory_revision = ?8 AND state = 'cleanup_pending'
                 AND closed_at_ms IS NULL AND closed_by_plan_id IS NULL
                 AND closed_by_plan_digest IS NULL AND close_reason IS NULL"#,
            params![
                staging.candidate_token(),
                slot.plugin_id.as_str(),
                slot.slot_ref.as_str(),
                expected.candidate_generation,
                release_json,
                expected.owner_plan_id.as_str(),
                expected.owner_plan_digest.as_str(),
                expected.application_inventory_revision,
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_OWNER_READ")
}

fn count_exact_authority_time(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupParentAbsenceAuthoritySession<'_>,
    expected_ms: i64,
) -> Result<i64> {
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM authority_meta WHERE singleton = 1
               AND installation_id_digest = ?1 AND process_owner_epoch = ?2
               AND trusted_time_high_water_ms >= ?3
               AND updated_at_ms = trusted_time_high_water_ms
               AND clock_status = 'trusted'"#,
            params![
                session.installation_id_digest(),
                session.process_owner_epoch(),
                expected_ms,
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_TIME_READ")
}
