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
        validate_hashed_execution_plan, CandidateCleanupParentAbsenceRecoveryKey,
        HashedComputePluginCandidateCleanupStepEvent,
    },
    local_authority::{
        cleanup_store::binding::validate_failed_candidate_inventory,
        cleanup_topology_store::read_exact_sealed_plan,
        plan_application::read_authority_plan_application_state_at_or_before_observation,
        AuthorizedCandidateCleanupDeletionGuard,
        CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_CANONICALIZATION,
        CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_DIGEST_ALGORITHM,
        CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA,
        HASHED_CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginCandidateCleanupParentAbsenceRecoveryOutcome
{
    NotCreated,
    Durable(HashedComputePluginCandidateCleanupStepEvent),
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_cleanup_parent_absence_recovery_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession<'authority>>
    {
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
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_SESSION_INVALID");
        }
        Ok(
            ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession {
                authority: self,
                process_fence,
                trusted_now,
                observed_at,
                clock_epoch_digest: observation.clock_epoch_digest().to_string(),
            },
        )
    }
}

impl ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession<'_> {
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

    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_cleanup_parent_absence_outcome(
        &self,
        key: &CandidateCleanupParentAbsenceRecoveryKey,
    ) -> Result<ComputePluginCandidateCleanupParentAbsenceRecoveryOutcome> {
        validate_recovery_provenance(self, key)?;
        self.authority
            .with_deferred(|transaction| read_outcome(transaction, self, key))
    }
}

impl ComputePluginCandidateCleanupParentAbsenceRecoveryOutcome {
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
    session: &ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession<'_>,
    key: &CandidateCleanupParentAbsenceRecoveryKey,
) -> Result<()> {
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.plan().plan().installation_id_digest() != session.installation_id_digest()
        || key.plan().plan().process_owner_epoch() != session.process_owner_epoch()
        || key.absence_event().event().recorded_at_ms() > session.trusted_now_ms()
        || key.parent_absence_observed_at() >= key.prepared_at()
        || session.observed_at <= session.process_fence.acquired_observed_at()
        || session.observed_at <= key.prepared_at()
        || session.observed_at <= key.parent_absence_observed_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_PROVENANCE_CHANGED");
    }
    validate_hashed_execution_plan(key.plan())?;
    validate_hashed_cleanup_step_event(key.intent_event())?;
    validate_hashed_cleanup_step_event(key.disposition_event())?;
    validate_hashed_cleanup_step_event(key.absence_event())?;
    validate_authorization_receipt(key)?;
    let expected_intent =
        build_initial_delete_intent(key.plan(), key.intent_event().event().recorded_at_ms())?;
    let object = key.plan().objects().first().ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_OBJECT_MISSING")
    })?;
    let disposition = key.disposition_event().event();
    let absence = key.absence_event().event();
    if expected_intent != *key.intent_event()
        || disposition.cleanup_id() != key.plan().plan().cleanup_id()
        || disposition.plan_digest() != key.plan().plan_digest()
        || disposition.event_sequence() != 2
        || disposition.step_ordinal() != 0
        || disposition.event_kind() != "exact_handle_disposition_set"
        || disposition.object_digest() != object.object_digest()
        || disposition.observed_identity_digest()
            != Some(object.object().expected_identity_digest())
        || disposition.observed_parent_identity_digest()
            != object.object().expected_parent_identity_digest()
        || disposition.namespace_durability_kind().is_some()
        || disposition.namespace_durability_evidence_digest().is_some()
        || disposition.previous_event_digest() != key.intent_event().event_digest()
        || disposition.process_owner_epoch() != key.plan().plan().process_owner_epoch()
        || disposition.recorded_at_ms() <= key.intent_event().event().recorded_at_ms()
        || absence.cleanup_id() != key.plan().plan().cleanup_id()
        || absence.plan_digest() != key.plan().plan_digest()
        || absence.event_sequence() != 3
        || absence.step_ordinal() != 0
        || absence.event_kind() != "parent_namespace_absence_observed"
        || absence.object_digest() != object.object_digest()
        || absence.observed_identity_digest().is_some()
        || absence.observed_parent_identity_digest()
            != object.object().expected_parent_identity_digest()
        || absence.namespace_durability_kind().is_some()
        || absence.namespace_durability_evidence_digest().is_some()
        || absence.previous_event_digest() != key.disposition_event().event_digest()
        || absence.process_owner_epoch() != key.plan().plan().process_owner_epoch()
        || absence.recorded_at_ms() <= disposition.recorded_at_ms()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_BINDING_CHANGED");
    }
    Ok(())
}

fn validate_authorization_receipt(key: &CandidateCleanupParentAbsenceRecoveryKey) -> Result<()> {
    let hashed = key.authorization_receipt();
    let receipt = hashed.receipt();
    if hashed.schema() != HASHED_CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA
        || receipt.schema() != CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA
        || hashed.canonicalization() != CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_CANONICALIZATION
        || hashed.digest_algorithm() != CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_DIGEST_ALGORITHM
        || receipt.cleanup_id() != key.plan().plan().cleanup_id()
        || receipt.candidate_token_digest() != key.plan().plan().candidate_token_digest()
        || hashed.receipt_digest() != key.plan().plan().authorization_receipt_digest()
        || receipt.process_owner_epoch() != key.plan().plan().process_owner_epoch()
        || key.plan().plan().planned_at_ms() <= receipt.authorized_at_ms()
        || receipt.slot_phase_before() != "failed"
        || !is_sha256(hashed.receipt_digest())
        || jcs_sha256_hex(receipt)? != hashed.receipt_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_AUTHORIZATION_CHANGED");
    }
    Ok(())
}

fn read_outcome(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession<'_>,
    key: &CandidateCleanupParentAbsenceRecoveryKey,
) -> Result<ComputePluginCandidateCleanupParentAbsenceRecoveryOutcome> {
    let stored_plan = read_exact_sealed_plan(transaction, key.plan(), key.candidate_token())?;
    if stored_plan.as_ref() != Some(key.plan())
        || count_exact_authorization(transaction, key)? != 1
        || count_exact_pending_owner(transaction, key)? != 1
        || super::super::recovery::count_completion(transaction, key.candidate_token())? != 0
        || read_exact_step_event(transaction, key.intent_event())?.as_ref()
            != Some(key.intent_event())
        || count_event_identity_matches(transaction, key.intent_event())? != 1
        || read_exact_step_event(transaction, key.disposition_event())?.as_ref()
            != Some(key.disposition_event())
        || count_event_identity_matches(transaction, key.disposition_event())? != 1
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_AUTHORITY_CHANGED");
    }
    let stored = read_exact_step_event(transaction, key.absence_event())?;
    let identity_matches = count_event_identity_matches(transaction, key.absence_event())?;
    match stored {
        Some(event) => {
            if identity_matches != 1 || count_events(transaction, event.event().cleanup_id())? != 3
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_RESULT_AMBIGUOUS");
            }
            validate_authority_snapshot(transaction, session, key, event.event().recorded_at_ms())?;
            Ok(ComputePluginCandidateCleanupParentAbsenceRecoveryOutcome::Durable(event))
        }
        None => {
            if identity_matches != 0
                || count_events(transaction, key.plan().plan().cleanup_id())? != 2
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_ABSENCE_AMBIGUOUS");
            }
            validate_authority_snapshot(
                transaction,
                session,
                key,
                key.disposition_event().event().recorded_at_ms(),
            )?;
            Ok(ComputePluginCandidateCleanupParentAbsenceRecoveryOutcome::NotCreated)
        }
    }
}

fn validate_authority_snapshot(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession<'_>,
    key: &CandidateCleanupParentAbsenceRecoveryKey,
    expected_high_water_ms: i64,
) -> Result<()> {
    let receipt = key.authorization_receipt().receipt();
    let authority = read_authority_plan_application_state_at_or_before_observation(
        transaction,
        &session.trusted_now,
    )?;
    validate_failed_candidate_inventory(
        &authority.inventory,
        key.owner_plugin_id(),
        key.owner_slot_ref(),
        key.owner_release(),
    )?;
    if authority.installation_id_digest != session.installation_id_digest()
        || authority.process_owner_epoch != session.process_owner_epoch()
        || authority.state_revision < receipt.authority_state_revision_after()
        || authority.inventory.inventory_revision < receipt.inventory_revision()
        || authority.authority_epoch < receipt.authority_epoch_after()
        || authority.trusted_time_high_water_ms < expected_high_water_ms
        || count_authority_time(transaction, session, expected_high_water_ms)? != 1
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_SNAPSHOT_CHANGED");
    }
    Ok(())
}

fn count_exact_authorization(
    transaction: &Transaction<'_>,
    key: &CandidateCleanupParentAbsenceRecoveryKey,
) -> Result<i64> {
    let receipt = key.authorization_receipt().receipt();
    let receipt_json = serde_json::to_string(receipt).context(
        "COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_AUTHORIZATION_SERIALIZE",
    )?;
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
                key.candidate_token(),
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
                key.authorization_receipt().receipt_digest(),
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_AUTHORIZATION_READ")
}

fn count_exact_pending_owner(
    transaction: &Transaction<'_>,
    key: &CandidateCleanupParentAbsenceRecoveryKey,
) -> Result<i64> {
    let release_json = serde_json::to_string(key.owner_release()).context(
        "COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_OWNER_RELEASE_SERIALIZE",
    )?;
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
                key.candidate_token(),
                key.owner_plugin_id(),
                key.owner_slot_ref(),
                key.owner_candidate_generation(),
                release_json,
                key.owner_plan_id(),
                key.owner_plan_digest(),
                key.owner_application_inventory_revision(),
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_OWNER_READ")
}

fn count_authority_time(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession<'_>,
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
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PARENT_ABSENCE_RECOVERY_TIME_READ")
}
