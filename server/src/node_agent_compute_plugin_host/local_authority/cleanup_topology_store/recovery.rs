use std::time::Instant;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Transaction};

use super::{
    validation::{count_objects, count_plan_identity_matches, count_seals, read_exact_sealed_plan},
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        validate_hashed_execution_plan, CandidateCleanupTopologyRecoveryKey,
        HashedComputePluginCandidateCleanupExecutionPlan,
    },
    local_authority::{
        cleanup_store::binding::validate_candidate_cleanup_continuation,
        plan_application::read_authority_plan_application_state_at_or_before_observation,
        AuthorizedCandidateCleanupDeletionGuard,
    },
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupTopologyRecoveryAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginCandidateCleanupTopologyRecoveryOutcome
{
    NotCreated,
    Sealed(HashedComputePluginCandidateCleanupExecutionPlan),
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_cleanup_topology_recovery_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginCandidateCleanupTopologyRecoveryAuthoritySession<'authority>> {
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
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_RECOVERY_SESSION_INVALID");
        }
        Ok(
            ComputePluginCandidateCleanupTopologyRecoveryAuthoritySession {
                authority: self,
                process_fence,
                trusted_now,
                observed_at,
                clock_epoch_digest: observation.clock_epoch_digest().to_string(),
            },
        )
    }
}

impl ComputePluginCandidateCleanupTopologyRecoveryAuthoritySession<'_> {
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
    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_cleanup_topology_outcome(
        &self,
        key: &CandidateCleanupTopologyRecoveryKey,
    ) -> Result<ComputePluginCandidateCleanupTopologyRecoveryOutcome> {
        validate_recovery_provenance(self, key)?;
        self.authority
            .with_deferred(|transaction| read_outcome(transaction, self, key))
    }
}

impl ComputePluginCandidateCleanupTopologyRecoveryOutcome {
    pub(in crate::node_agent_compute_plugin_host) fn is_not_created(&self) -> bool {
        matches!(self, Self::NotCreated)
    }
    pub(in crate::node_agent_compute_plugin_host) fn plan(
        &self,
    ) -> Option<&HashedComputePluginCandidateCleanupExecutionPlan> {
        match self {
            Self::NotCreated => None,
            Self::Sealed(plan) => Some(plan),
        }
    }
}

fn validate_recovery_provenance(
    session: &ComputePluginCandidateCleanupTopologyRecoveryAuthoritySession<'_>,
    key: &CandidateCleanupTopologyRecoveryKey,
) -> Result<()> {
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.plan().plan().process_owner_epoch() != session.process_owner_epoch()
        || key.plan().plan().planned_at_ms() > session.trusted_now_ms()
        || session.observed_at <= session.process_fence.acquired_observed_at()
        || session.observed_at <= key.prepared_at()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_RECOVERY_PROVENANCE_CHANGED");
    }
    let plan = key.plan().plan();
    let authorization = key.authorization_receipt();
    let receipt = authorization.receipt();
    if authorization.receipt_digest() != plan.authorization_receipt_digest()
        || receipt.cleanup_id() != plan.cleanup_id()
        || receipt.candidate_token_digest() != plan.candidate_token_digest()
        || receipt.process_owner_epoch() != plan.process_owner_epoch()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_RECOVERY_ANCHOR_CHANGED");
    }
    validate_hashed_execution_plan(key.plan())
}

fn read_outcome(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupTopologyRecoveryAuthoritySession<'_>,
    key: &CandidateCleanupTopologyRecoveryKey,
) -> Result<ComputePluginCandidateCleanupTopologyRecoveryOutcome> {
    let stored = read_exact_sealed_plan(transaction, key.plan(), key.candidate_token())?;
    let identity_matches =
        count_plan_identity_matches(transaction, key.candidate_token(), key.plan())?;
    if count_completion(transaction, key.candidate_token())? != 0 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_RECOVERY_AUTHORITY_CHANGED");
    }
    let authority = read_authority_plan_application_state_at_or_before_observation(
        transaction,
        &session.trusted_now,
    )?;
    if authority.installation_id_digest != session.installation_id_digest() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_RECOVERY_PROCESS_CHANGED");
    }
    validate_candidate_cleanup_continuation(
        transaction,
        &authority,
        key.candidate_token(),
        key.authorization_receipt(),
        key.owner(),
    )?;
    match stored {
        Some(plan) => {
            if identity_matches != 1
                || count_objects(transaction, plan.plan().cleanup_id())?
                    != plan.plan().object_count()
                || count_seals(transaction, plan.plan().cleanup_id())? != 1
                || count_authority_time(transaction, session, plan.plan().planned_at_ms())? != 1
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_RECOVERY_RESULT_AMBIGUOUS");
            }
            Ok(ComputePluginCandidateCleanupTopologyRecoveryOutcome::Sealed(plan))
        }
        None => {
            if identity_matches != 0
                || count_objects(transaction, key.plan().plan().cleanup_id())? != 0
                || count_seals(transaction, key.plan().plan().cleanup_id())? != 0
                || count_authority_time(transaction, session, key.authorized_at_ms())? != 1
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_RECOVERY_ABSENCE_AMBIGUOUS");
            }
            Ok(ComputePluginCandidateCleanupTopologyRecoveryOutcome::NotCreated)
        }
    }
}

fn count_completion(transaction: &Transaction<'_>, candidate_token: &str) -> Result<i64> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM candidate_cleanup_completions WHERE candidate_token = ?1",
            params![candidate_token],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_RECOVERY_COMPLETION_READ")
}

fn count_authority_time(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupTopologyRecoveryAuthoritySession<'_>,
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
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_RECOVERY_TIME_READ")
}
