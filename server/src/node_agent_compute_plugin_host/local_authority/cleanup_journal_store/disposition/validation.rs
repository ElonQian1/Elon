use anyhow::{bail, Result};
use rusqlite::Transaction;

use super::ComputePluginCandidateCleanupDispositionAuthoritySession;
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        build_exact_handle_disposition_event, validate_hashed_cleanup_step_event,
        HashedComputePluginCandidateCleanupStepEvent, PhysicallyDisposedCandidateCleanupObject,
    },
    local_authority::{
        cleanup_journal_store::validation::{
            count_event_identity_matches, count_events, read_exact_step_event,
        },
        cleanup_store::binding::validate_candidate_cleanup_continuation,
        cleanup_topology_store::read_exact_sealed_plan,
        plan_application::read_authority_plan_application_state_at_or_before_observation,
    },
};

pub(super) fn validate_unstored_disposition(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupDispositionAuthoritySession<'_>,
    physical: &PhysicallyDisposedCandidateCleanupObject,
    event: &HashedComputePluginCandidateCleanupStepEvent,
) -> Result<()> {
    validate_hashed_cleanup_step_event(event)?;
    let expected = build_exact_handle_disposition_event(
        physical.plan(),
        physical.intent_event(),
        physical.object_binding(),
        session.trusted_now_ms(),
    )?;
    if expected != *event || physical.state().completed_step_count() != 0 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_BINDING_CHANGED");
    }
    validate_authority_and_owner(
        transaction,
        session,
        physical,
        physical.intent_event().event().recorded_at_ms(),
    )?;
    let stored_plan = read_exact_sealed_plan(
        transaction,
        physical.plan(),
        physical.state().staging_recovery_key().candidate_token(),
    )?;
    let stored_intent = read_exact_step_event(transaction, physical.intent_event())?;
    if stored_plan.as_ref() != Some(physical.plan())
        || stored_intent.as_ref() != Some(physical.intent_event())
        || count_event_identity_matches(transaction, physical.intent_event())? != 1
        || count_event_identity_matches(transaction, event)? != 0
        || count_events(transaction, physical.plan().plan().cleanup_id())? != 1
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_PREDECESSOR_CHANGED");
    }
    Ok(())
}

pub(super) fn validate_authority_and_owner(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupDispositionAuthoritySession<'_>,
    physical: &PhysicallyDisposedCandidateCleanupObject,
    expected_high_water_ms: i64,
) -> Result<()> {
    let state = physical.state();
    let authorization = state.authorization_receipt();
    let receipt = authorization.receipt();
    let authority = read_authority_plan_application_state_at_or_before_observation(
        transaction,
        &session.trusted_now,
    )?;
    let recovery = state.staging_recovery_key();
    let owner = crate::node_agent_compute_plugin_host::candidate_cleanup_contract::CandidateCleanupOwnerExpectation::from_staging(recovery);
    validate_candidate_cleanup_continuation(
        transaction,
        &authority,
        recovery.candidate_token(),
        authorization,
        &owner,
    )?;
    if authority.installation_id_digest != session.installation_id_digest()
        || receipt.process_owner_epoch() != session.process_owner_epoch()
        || authority.trusted_time_high_water_ms < expected_high_water_ms
        || super::super::recovery::count_completion(transaction, recovery.candidate_token())? != 0
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_AUTHORITY_CHANGED");
    }
    Ok(())
}
