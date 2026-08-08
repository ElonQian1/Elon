use anyhow::{bail, Result};
use rusqlite::Transaction;

use super::{
    validation::{validate_authority_and_owner, validate_unstored_namespace_durability},
    ComputePluginCandidateCleanupNamespaceDurabilityAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        HashedComputePluginCandidateCleanupStepEvent,
        ValidatedCandidateCleanupNamespaceDurabilityPermit,
    },
    local_authority::{
        cleanup_journal_store::{
            validation::{count_event_identity_matches, count_events, read_exact_step_event},
            write::insert_event,
        },
        keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
    },
};

pub(super) fn persist_candidate_cleanup_namespace_durability(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupNamespaceDurabilityAuthoritySession<'_>,
    permit: ValidatedCandidateCleanupNamespaceDurabilityPermit<'_>,
) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
    let physical = permit.physical();
    let event = permit.event();
    session.validate_source(physical.state().cancellation_guard())?;
    validate_unstored_namespace_durability(transaction, session, physical, event)?;
    let absence_time = physical.absence_event().event().recorded_at_ms();
    let time_state = read_authority_keyring_state(transaction)?;
    if time_state.trusted_time_high_water_ms != Some(absence_time)
        || time_state.clock_status != "trusted"
        || event.event().recorded_at_ms() <= absence_time
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_NAMESPACE_DURABILITY_TIME_CHANGED");
    }
    session.validate_source(physical.state().cancellation_guard())?;
    physical.namespace().ensure_mutation_fence_active()?;
    advance_trusted_time(transaction, &time_state, event.event().recorded_at_ms())?;
    session.validate_source(physical.state().cancellation_guard())?;
    insert_event(transaction, event)?;
    validate_authority_and_owner(
        transaction,
        session,
        physical,
        event.event().recorded_at_ms(),
    )?;
    let stored_intent = read_exact_step_event(transaction, physical.intent_event())?;
    let stored_disposition = read_exact_step_event(transaction, physical.disposition_event())?;
    let stored_absence = read_exact_step_event(transaction, physical.absence_event())?;
    let stored = read_exact_step_event(transaction, event)?.ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_NAMESPACE_DURABILITY_READBACK_MISSING")
    })?;
    if stored_intent.as_ref() != Some(physical.intent_event())
        || stored_disposition.as_ref() != Some(physical.disposition_event())
        || stored_absence.as_ref() != Some(physical.absence_event())
        || stored != *event
        || count_event_identity_matches(transaction, physical.intent_event())? != 1
        || count_event_identity_matches(transaction, physical.disposition_event())? != 1
        || count_event_identity_matches(transaction, physical.absence_event())? != 1
        || count_event_identity_matches(transaction, event)? != 1
        || count_events(transaction, event.event().cleanup_id())? != 4
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_NAMESPACE_DURABILITY_READBACK_CHANGED");
    }
    physical.namespace().ensure_mutation_fence_active()?;
    session.validate_source(physical.state().cancellation_guard())?;
    Ok(stored)
}
