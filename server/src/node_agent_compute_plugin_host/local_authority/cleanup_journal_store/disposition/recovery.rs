use std::time::Instant;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Transaction};

use super::super::{
    validation::{count_event_identity_matches, count_events, read_exact_step_event},
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        build_initial_delete_intent, validate_hashed_cleanup_step_event,
        validate_hashed_execution_plan, CandidateCleanupDispositionRecoveryKey,
        HashedComputePluginCandidateCleanupStepEvent,
    },
    local_authority::{
        cleanup_store::binding::validate_candidate_cleanup_continuation,
        cleanup_topology_store::read_exact_sealed_plan,
        plan_application::read_authority_plan_application_state_at_or_before_observation,
        AuthorizedCandidateCleanupDeletionGuard,
    },
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupDispositionRecoveryAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginCandidateCleanupDispositionRecoveryOutcome
{
    NotCreated,
    Durable(HashedComputePluginCandidateCleanupStepEvent),
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_cleanup_disposition_recovery_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginCandidateCleanupDispositionRecoveryAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let observed_at = observation.observed_at();
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || !is_sha256(observation.installation_id_digest())
            || !is_sha256(observation.clock_epoch_digest())
            || process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
            || observed_at <= process_fence.acquired_observed_at()
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_RECOVERY_SESSION_INVALID");
        }
        Ok(
            ComputePluginCandidateCleanupDispositionRecoveryAuthoritySession {
                authority: self,
                process_fence,
                trusted_now,
                observed_at,
                clock_epoch_digest: observation.clock_epoch_digest().to_string(),
            },
        )
    }
}

impl ComputePluginCandidateCleanupDispositionRecoveryAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.process_fence.authority_instance_binding()
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.process_fence.installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_fence.process_owner_epoch()
    }

    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn observed_at(&self) -> Instant {
        self.observed_at
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_now_ms(&self) -> i64 {
        self.trusted_now.timestamp_millis()
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate_source(
        &self,
        guard: &AuthorizedCandidateCleanupDeletionGuard,
    ) -> Result<()> {
        guard.validate_process_fence(self.process_fence)
    }

    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_cleanup_disposition_outcome(
        &self,
        key: &CandidateCleanupDispositionRecoveryKey,
    ) -> Result<ComputePluginCandidateCleanupDispositionRecoveryOutcome> {
        validate_recovery_provenance(self, key)?;
        self.authority
            .with_deferred(|transaction| read_outcome(transaction, self, key))
    }
}

impl ComputePluginCandidateCleanupDispositionRecoveryOutcome {
    pub(in crate::node_agent_compute_plugin_host) fn is_not_created(&self) -> bool {
        matches!(self, Self::NotCreated)
    }

    pub(in crate::node_agent_compute_plugin_host) fn event(
        &self,
    ) -> Option<&HashedComputePluginCandidateCleanupStepEvent> {
        match self {
            Self::NotCreated => None,
            Self::Durable(event) => Some(event),
        }
    }
}

fn validate_recovery_provenance(
    session: &ComputePluginCandidateCleanupDispositionRecoveryAuthoritySession<'_>,
    key: &CandidateCleanupDispositionRecoveryKey,
) -> Result<()> {
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.plan().plan().process_owner_epoch() != session.process_owner_epoch()
        || key.disposition_event().event().recorded_at_ms() > session.trusted_now_ms()
        || session.observed_at <= session.process_fence.acquired_observed_at()
        || session.observed_at <= key.prepared_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_RECOVERY_PROVENANCE_CHANGED");
    }
    let plan = key.plan().plan();
    let authorization = key.authorization_receipt();
    let receipt = authorization.receipt();
    if authorization.receipt_digest() != plan.authorization_receipt_digest()
        || receipt.cleanup_id() != plan.cleanup_id()
        || receipt.candidate_token_digest() != plan.candidate_token_digest()
        || receipt.process_owner_epoch() != plan.process_owner_epoch()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_RECOVERY_ANCHOR_CHANGED");
    }
    validate_hashed_execution_plan(key.plan())?;
    validate_hashed_cleanup_step_event(key.intent_event())?;
    validate_hashed_cleanup_step_event(key.disposition_event())?;
    let expected_intent =
        build_initial_delete_intent(key.plan(), key.intent_event().event().recorded_at_ms())?;
    let expected_disposition = key.plan().objects().first().ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_OBJECT_MISSING")
    })?;
    let disposition = key.disposition_event().event();
    if expected_intent != *key.intent_event()
        || disposition.cleanup_id() != key.plan().plan().cleanup_id()
        || disposition.plan_digest() != key.plan().plan_digest()
        || disposition.event_sequence() != 2
        || disposition.step_ordinal() != 0
        || disposition.event_kind() != "exact_handle_disposition_set"
        || disposition.object_digest() != expected_disposition.object_digest()
        || disposition.observed_identity_digest()
            != Some(expected_disposition.object().expected_identity_digest())
        || disposition.observed_parent_identity_digest()
            != expected_disposition
                .object()
                .expected_parent_identity_digest()
        || disposition.namespace_durability_kind().is_some()
        || disposition.namespace_durability_evidence_digest().is_some()
        || disposition.previous_event_digest() != key.intent_event().event_digest()
        || disposition.process_owner_epoch() != key.plan().plan().process_owner_epoch()
        || disposition.recorded_at_ms() <= key.intent_event().event().recorded_at_ms()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_RECOVERY_BINDING_CHANGED");
    }
    Ok(())
}

fn read_outcome(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupDispositionRecoveryAuthoritySession<'_>,
    key: &CandidateCleanupDispositionRecoveryKey,
) -> Result<ComputePluginCandidateCleanupDispositionRecoveryOutcome> {
    let stored_plan = read_exact_sealed_plan(transaction, key.plan(), key.candidate_token())?;
    if stored_plan.as_ref() != Some(key.plan())
        || super::super::recovery::count_completion(transaction, key.candidate_token())? != 0
        || read_exact_step_event(transaction, key.intent_event())?.as_ref()
            != Some(key.intent_event())
        || count_event_identity_matches(transaction, key.intent_event())? != 1
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_RECOVERY_AUTHORITY_CHANGED");
    }
    let authority = read_authority_plan_application_state_at_or_before_observation(
        transaction,
        &session.trusted_now,
    )?;
    if authority.installation_id_digest != session.installation_id_digest() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_RECOVERY_PROCESS_CHANGED");
    }
    validate_candidate_cleanup_continuation(
        transaction,
        &authority,
        key.candidate_token(),
        key.authorization_receipt(),
        key.owner(),
    )?;
    let stored = read_exact_step_event(transaction, key.disposition_event())?;
    let identity_matches = count_event_identity_matches(transaction, key.disposition_event())?;
    match stored {
        Some(event) => {
            if identity_matches != 1
                || count_events(transaction, event.event().cleanup_id())? != 2
                || count_authority_time(transaction, session, event.event().recorded_at_ms())? != 1
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_RECOVERY_RESULT_AMBIGUOUS");
            }
            Ok(ComputePluginCandidateCleanupDispositionRecoveryOutcome::Durable(event))
        }
        None => {
            if identity_matches != 0
                || count_events(transaction, key.plan().plan().cleanup_id())? != 1
                || count_authority_time(
                    transaction,
                    session,
                    key.intent_event().event().recorded_at_ms(),
                )? != 1
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_RECOVERY_ABSENCE_AMBIGUOUS");
            }
            Ok(ComputePluginCandidateCleanupDispositionRecoveryOutcome::NotCreated)
        }
    }
}

fn count_authority_time(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupDispositionRecoveryAuthoritySession<'_>,
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
                expected_ms
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_RECOVERY_TIME_READ")
}
