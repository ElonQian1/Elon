use std::time::Instant;
use std::{cell::Cell, fmt};

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::node_agent_compute_plugin_host::{
    fetch_contract::ComputePluginFetchCancellationGuard, identity::ComputePluginReleaseRef,
    install_plan::ComputePluginPlannedDownload,
    install_plan_admission::ComputePluginLiveAdmissionState,
    lifecycle::ComputePluginInventorySnapshot, manifest_validation::is_sha256,
};

use super::{
    ComputePluginFetchAuthoritySession, ComputePluginFetchProcessFence, ComputePluginLocalAuthority,
};

mod abort;
mod begin;
mod closure;
mod outcome;
mod post_hash;
mod read;
mod resolution;
mod staging_read;

pub(super) use staging_read::{
    read_verified_candidate_staging_snapshot, VerifiedCandidateStagingSnapshot,
};

pub(in crate::node_agent_compute_plugin_host) use post_hash::{
    ComputePluginPostHashVerificationAuthoritySession,
    ComputePluginPostHashVerificationBindingFacts,
};

/// A fresh trusted-time session observed strictly after all candidate file handles were pinned.
/// It carries no filesystem or generic Store-write capability. Its only mutation seam accepts the
/// private linear begin permit and performs the third durable read inside `BEGIN IMMEDIATE`.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPostPinVerificationAuthoritySession<
    'authority,
> {
    authority_session: ComputePluginFetchAuthoritySession<'authority>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

/// Recovery reads and cleanup do not replay publisher material. They are bound only to the same
/// local authority instance, current process-owner fence and a fresh trusted-time observation.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateVerificationRecoveryAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    clock_epoch_digest: String,
}

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateVerificationOutcomeReadFailure
{
    error: anyhow::Error,
    run_observed: bool,
}

impl fmt::Debug for ComputePluginCandidateVerificationOutcomeReadFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginCandidateVerificationOutcomeReadFailure")
            .field("error", &self.error)
            .field("run_observed", &self.run_observed)
            .finish()
    }
}

impl ComputePluginCandidateVerificationOutcomeReadFailure {
    pub(in crate::node_agent_compute_plugin_host) fn run_observed(&self) -> bool {
        self.run_observed
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_error(self) -> anyhow::Error {
        self.error
    }
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn candidate_verification_recovery_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: crate::node_agent_compute_plugin_host::trusted_time::ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginCandidateVerificationRecoveryAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let clock_epoch_digest = observation.clock_epoch_digest().to_string();
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || !is_sha256(observation.installation_id_digest())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || !is_sha256(observation.clock_epoch_digest())
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
            || observation.observed_at() <= process_fence.acquired_observed_at()
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            anyhow::bail!("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_SESSION_INVALID");
        }
        Ok(ComputePluginCandidateVerificationRecoveryAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            clock_epoch_digest,
        })
    }
}

impl ComputePluginCandidateVerificationRecoveryAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &super::ComputePluginAuthorityInstanceBinding {
        self.process_fence.authority_instance_binding()
    }

    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_verification_outcome(
        &self,
        key: &crate::node_agent_compute_plugin_host::candidate_verification_contract::ComputePluginCandidateVerificationRecoveryKey,
    ) -> std::result::Result<
        crate::node_agent_compute_plugin_host::candidate_verification_contract::ComputePluginCandidateVerificationOutcome,
        ComputePluginCandidateVerificationOutcomeReadFailure,
    >{
        if !key
            .authority_instance_binding()
            .matches(self.process_fence.authority_instance_binding())
            || key.clock_epoch_digest() != self.clock_epoch_digest.as_str()
        {
            return Err(ComputePluginCandidateVerificationOutcomeReadFailure {
                error: anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_PROVENANCE_CHANGED"),
                run_observed: false,
            });
        }
        let run_observed = Cell::new(false);
        let result = self.authority.with_deferred(|transaction| {
            if outcome::exact_verification_id_exists(transaction, key)? {
                run_observed.set(true);
            }
            Ok(outcome::read_outcome_snapshot(transaction, self.process_fence, key)?.outcome)
        });
        result.map_err(
            |error| ComputePluginCandidateVerificationOutcomeReadFailure {
                error,
                run_observed: run_observed.get(),
            },
        )
    }

    pub(in crate::node_agent_compute_plugin_host) fn abort_recovered_candidate_verification(
        &self,
        permit: crate::node_agent_compute_plugin_host::candidate_verification_contract::ValidatedCandidateVerificationRecoveryAbortPermit<'_>,
    ) -> Result<crate::node_agent_compute_plugin_host::candidate_verification_contract::ComputePluginCandidateVerificationOutcome>{
        let key = permit.key();
        if !key
            .authority_instance_binding()
            .matches(self.process_fence.authority_instance_binding())
            || key.clock_epoch_digest() != self.clock_epoch_digest.as_str()
        {
            anyhow::bail!("COMPUTE_PLUGIN_VERIFICATION_RECOVERY_PROVENANCE_CHANGED");
        }
        self.authority.with_immediate(|transaction| {
            abort::abort_recovered_candidate_verification(
                transaction,
                self.process_fence,
                self.trusted_now.timestamp_millis(),
                permit,
            )
        })
    }
}

impl<'authority> ComputePluginPostPinVerificationAuthoritySession<'authority> {
    pub(super) fn new(
        authority_session: ComputePluginFetchAuthoritySession<'authority>,
        observed_at: Instant,
        clock_epoch_digest: String,
    ) -> Self {
        Self {
            authority_session,
            observed_at,
            clock_epoch_digest,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn read_fresh_after_pin(
        &self,
        barrier: Instant,
        cancellation_guard: &ComputePluginFetchCancellationGuard,
        plan_id: &str,
        plan_digest: &str,
        candidate_token: &str,
    ) -> Result<ComputePluginCandidateVerificationAuthorityFacts> {
        if self.observed_at <= barrier || !is_sha256(&self.clock_epoch_digest) {
            anyhow::bail!("COMPUTE_PLUGIN_VERIFICATION_POST_PIN_TIME_INVALID");
        }
        self.authority_session
            .validate_fetch_cancellation_guard(cancellation_guard)?;
        cancellation_guard.ensure_current()?;
        self.authority_session
            .read_fresh_candidate_verification_authority(plan_id, plan_digest, candidate_token)
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate_begin_source(
        &self,
        cancellation_guard: &ComputePluginFetchCancellationGuard,
    ) -> Result<()> {
        if !is_sha256(&self.clock_epoch_digest) {
            anyhow::bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_CLOCK_BINDING_INVALID");
        }
        self.authority_session
            .validate_fetch_cancellation_guard(cancellation_guard)?;
        cancellation_guard.ensure_current()
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &super::ComputePluginAuthorityInstanceBinding {
        self.authority_session.recovery_authority_instance_binding()
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.authority_session.recovery_installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.authority_session.recovery_process_owner_epoch()
    }

    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn begin_validated_candidate_verification(
        &self,
        permit: crate::node_agent_compute_plugin_host::candidate_verification_contract::ValidatedCandidateVerificationBeginPermit<'_>,
    ) -> Result<ComputePluginPreparedCandidateVerificationFacts> {
        self.authority_session
            .begin_validated_candidate_verification(permit)
    }
}

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPreparedCandidateVerificationFacts
{
    verification_id: String,
    candidate_token_digest: String,
    owner_plan_id: String,
    owner_plan_digest: String,
    verification_generation: i64,
    candidate_generation: i64,
    application_inventory_revision: i64,
    authority_state_revision: i64,
    authority_epoch: i64,
    process_owner_epoch: i64,
    artifact_count: usize,
    artifact_bytes: i64,
    expected_artifact_set_digest: String,
    file_set_binding_digest: String,
    prepared_at_ms: i64,
}

impl fmt::Debug for ComputePluginPreparedCandidateVerificationFacts {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginPreparedCandidateVerificationFacts")
            .field("verification_id", &"<redacted>")
            .field("owner_plan_id", &self.owner_plan_id)
            .field("verification_generation", &self.verification_generation)
            .field("candidate_generation", &self.candidate_generation)
            .field("artifact_count", &self.artifact_count)
            .field("prepared_at_ms", &self.prepared_at_ms)
            .finish()
    }
}

impl ComputePluginPreparedCandidateVerificationFacts {
    pub(in crate::node_agent_compute_plugin_host) fn matches_recovery_key(
        &self,
        key: &crate::node_agent_compute_plugin_host::candidate_verification_contract::ComputePluginCandidateVerificationRecoveryKey,
    ) -> bool {
        self.verification_id == key.verification_id()
            && self.candidate_token_digest == key.candidate_token_digest()
            && self.owner_plan_id == key.owner_plan_id()
            && self.owner_plan_digest == key.owner_plan_digest()
            && self.verification_generation == key.verification_generation()
            && self.candidate_generation == key.candidate_generation()
            && self.application_inventory_revision == key.application_inventory_revision()
            && self.authority_state_revision == key.authority_state_revision()
            && self.authority_epoch == key.authority_epoch()
            && self.process_owner_epoch == key.process_owner_epoch()
            && self.artifact_count == key.artifact_count()
            && self.artifact_bytes == key.artifact_bytes()
            && self.expected_artifact_set_digest == key.expected_artifact_set_digest()
            && self.file_set_binding_digest == key.file_set_binding_digest()
            && self.prepared_at_ms == key.prepared_at_ms()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateArtifactAuthorityFacts {
    pub ordinal: usize,
    pub item_index: usize,
    pub planned_download: ComputePluginPlannedDownload,
    pub part_relative_path: String,
    pub committed_offset: i64,
    pub cursor_generation: i64,
    pub download_state: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateVerificationAuthorityFacts
{
    pub inventory: ComputePluginInventorySnapshot,
    pub live: ComputePluginLiveAdmissionState,
    pub trusted_now: DateTime<Utc>,
    pub observed_trusted_time_high_water_ms: i64,
    pub installation_id_digest: String,
    pub applied_plan_id: String,
    pub applied_plan_digest: String,
    pub application_inventory_revision: i64,
    pub execution_inventory_revision: i64,
    pub authority_state_revision: i64,
    pub inventory_digest: String,
    pub authority_epoch: i64,
    pub process_owner_epoch: i64,
    pub candidate_token_digest: String,
    pub candidate_generation: i64,
    pub candidate_owner_plan_id: String,
    pub candidate_owner_plan_digest: String,
    pub candidate_application_inventory_revision: i64,
    pub candidate_state: String,
    pub candidate_plugin_id: String,
    pub candidate_slot_ref: String,
    pub candidate_created_at_ms: i64,
    pub candidate_release: ComputePluginReleaseRef,
    pub candidate_permission_grant_digest: String,
    pub next_verification_generation: i64,
    pub artifact_bytes: i64,
    pub expected_artifact_set_digest: String,
    pub artifacts: Vec<ComputePluginCandidateArtifactAuthorityFacts>,
}

impl ComputePluginCandidateVerificationAuthorityFacts {
    pub(in crate::node_agent_compute_plugin_host) fn recompute_expected_artifact_set_digest(
        &self,
    ) -> Result<String> {
        closure::candidate_closure_snapshot_from_facts(self)
            .map(|snapshot| snapshot.expected_artifact_set_digest)
    }

    pub(in crate::node_agent_compute_plugin_host) fn recompute_durable_candidate_closure_digest(
        &self,
    ) -> Result<String> {
        closure::candidate_closure_snapshot_from_facts(self)
            .map(|snapshot| snapshot.durable_closure_digest)
    }

    pub(in crate::node_agent_compute_plugin_host) fn same_durable_projection(
        &self,
        current: &Self,
    ) -> bool {
        current.trusted_now >= self.trusted_now
            && current.observed_trusted_time_high_water_ms
                >= self.observed_trusted_time_high_water_ms
            && self.inventory == current.inventory
            && self.live == current.live
            && self.installation_id_digest == current.installation_id_digest
            && self.applied_plan_id == current.applied_plan_id
            && self.applied_plan_digest == current.applied_plan_digest
            && self.application_inventory_revision == current.application_inventory_revision
            && self.execution_inventory_revision == current.execution_inventory_revision
            && self.authority_state_revision == current.authority_state_revision
            && self.inventory_digest == current.inventory_digest
            && self.authority_epoch == current.authority_epoch
            && self.process_owner_epoch == current.process_owner_epoch
            && self.candidate_token_digest == current.candidate_token_digest
            && self.candidate_generation == current.candidate_generation
            && self.candidate_owner_plan_id == current.candidate_owner_plan_id
            && self.candidate_owner_plan_digest == current.candidate_owner_plan_digest
            && self.candidate_application_inventory_revision
                == current.candidate_application_inventory_revision
            && self.candidate_state == current.candidate_state
            && self.candidate_plugin_id == current.candidate_plugin_id
            && self.candidate_slot_ref == current.candidate_slot_ref
            && self.candidate_created_at_ms == current.candidate_created_at_ms
            && self.candidate_release == current.candidate_release
            && self.candidate_permission_grant_digest == current.candidate_permission_grant_digest
            && self.next_verification_generation == current.next_verification_generation
            && self.artifact_bytes == current.artifact_bytes
            && self.expected_artifact_set_digest == current.expected_artifact_set_digest
            && self.artifacts == current.artifacts
    }
}

pub(super) use begin::begin_candidate_verification;
pub(super) use read::read_fresh_candidate_verification_authority;
