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
        cleanup_topology_store::read_exact_sealed_plan,
        plan_application::read_authority_plan_application_state,
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
    let receipt = state.authorization_receipt().receipt();
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    if authority.installation_id_digest != session.installation_id_digest()
        || authority.process_owner_epoch != session.process_owner_epoch()
        || authority.state_revision != receipt.authority_state_revision_after()
        || authority.inventory.inventory_revision != receipt.inventory_revision()
        || authority.inventory_digest != receipt.inventory_digest()
        || authority.authority_epoch != receipt.authority_epoch_after()
        || authority.trusted_time_high_water_ms != expected_high_water_ms
        || count_exact_authorization(transaction, physical)? != 1
        || super::super::recovery::count_pending_owner(
            transaction,
            state.staging_recovery_key().candidate_token(),
        )? != 1
        || super::super::recovery::count_completion(
            transaction,
            state.staging_recovery_key().candidate_token(),
        )? != 0
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_AUTHORITY_CHANGED");
    }
    Ok(())
}

fn count_exact_authorization(
    transaction: &Transaction<'_>,
    physical: &PhysicallyDisposedCandidateCleanupObject,
) -> Result<i64> {
    let state = physical.state();
    let receipt = state.authorization_receipt().receipt();
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_authorizations
           WHERE cleanup_id = ?1 AND candidate_token = ?2
             AND candidate_token_digest = ?3 AND receipt_digest = ?4
             AND process_owner_epoch = ?5 AND authorized_at_ms = ?6
             AND slot_phase_before = 'failed'"#,
            rusqlite::params![
                receipt.cleanup_id(),
                state.staging_recovery_key().candidate_token(),
                receipt.candidate_token_digest(),
                state.authorization_receipt().receipt_digest(),
                receipt.process_owner_epoch(),
                receipt.authorized_at_ms(),
            ],
            |row| row.get(0),
        )
        .map_err(Into::into)
}
