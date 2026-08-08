use anyhow::{bail, Result};
use rusqlite::Transaction;

use super::{
    validation::{validate_authority_and_owner, validate_unstored_parent_absence},
    ComputePluginCandidateCleanupParentAbsenceAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        HashedComputePluginCandidateCleanupStepEvent, ValidatedCandidateCleanupParentAbsencePermit,
    },
    local_authority::{
        cleanup_journal_store::{
            validation::{count_event_identity_matches, count_events, read_exact_step_event},
            write::insert_event,
        },
        keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
    },
};

pub(super) fn persist_candidate_cleanup_parent_absence(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupParentAbsenceAuthoritySession<'_>,
    permit: ValidatedCandidateCleanupParentAbsencePermit<'_>,
) -> Result<HashedComputePluginCandidateCleanupStepEvent> {
    let observed = permit.observed();
    let event = permit.event();
    session.validate_source(observed.state().cancellation_guard())?;
    validate_unstored_parent_absence(transaction, session, observed, event)?;
    let disposition_time = observed.disposition_event().event().recorded_at_ms();
    let time_state = read_authority_keyring_state(transaction)?;
    if time_state.trusted_time_high_water_ms != Some(disposition_time)
        || time_state.clock_status != "trusted"
        || event.event().recorded_at_ms() <= disposition_time
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_TIME_CHANGED");
    }
    advance_trusted_time(transaction, &time_state, event.event().recorded_at_ms())?;
    session.validate_source(observed.state().cancellation_guard())?;
    insert_event(transaction, event)?;
    validate_authority_and_owner(
        transaction,
        session,
        observed,
        event.event().recorded_at_ms(),
    )?;
    let stored = read_exact_step_event(transaction, event)?.ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_READBACK_MISSING")
    })?;
    if stored != *event
        || count_event_identity_matches(transaction, event)? != 1
        || count_events(transaction, event.event().cleanup_id())? != 3
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_READBACK_CHANGED");
    }
    session.validate_source(observed.state().cancellation_guard())?;
    Ok(stored)
}

#[cfg(test)]
mod tests;
