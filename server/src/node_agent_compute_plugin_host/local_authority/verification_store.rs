use std::time::Instant;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::node_agent_compute_plugin_host::{
    fetch_contract::ComputePluginFetchCancellationGuard, identity::ComputePluginReleaseRef,
    install_plan::ComputePluginPlannedDownload,
    install_plan_admission::ComputePluginLiveAdmissionState,
    lifecycle::ComputePluginInventorySnapshot, manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

use super::ComputePluginFetchAuthoritySession;

mod read;

const EXPECTED_ARTIFACT_SET_SCHEMA: &str = "elon.compute_plugin.expected_artifact_set.v1";

/// A fresh trusted-time session observed strictly after all candidate file handles were pinned.
/// It carries no filesystem or Store-write capability; the next layer must still re-read durable
/// authority and later perform a separate purpose-specific `BEGIN IMMEDIATE` CAS.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPostPinVerificationAuthoritySession<
    'authority,
> {
    authority_session: ComputePluginFetchAuthoritySession<'authority>,
    observed_at: Instant,
    clock_epoch_digest: String,
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
        expected_artifact_set_digest(self)
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
            && self.candidate_release == current.candidate_release
            && self.candidate_permission_grant_digest == current.candidate_permission_grant_digest
            && self.next_verification_generation == current.next_verification_generation
            && self.artifact_bytes == current.artifact_bytes
            && self.expected_artifact_set_digest == current.expected_artifact_set_digest
            && self.artifacts == current.artifacts
    }
}

#[derive(Serialize)]
struct ExpectedArtifactSet<'facts> {
    schema: &'static str,
    release: &'facts ComputePluginReleaseRef,
    digest_algorithm: &'static str,
    artifact_count: usize,
    artifact_bytes: i64,
    artifacts: Vec<ExpectedArtifact<'facts>>,
}

#[derive(Serialize)]
struct ExpectedArtifact<'facts> {
    sequence: usize,
    artifact_kind: &'facts str,
    artifact_id: &'facts str,
    digest: &'facts str,
    size_bytes: i64,
}

fn expected_artifact_set_digest(
    facts: &ComputePluginCandidateVerificationAuthorityFacts,
) -> Result<String> {
    let artifacts = facts
        .artifacts
        .iter()
        .enumerate()
        .map(|(sequence, artifact)| ExpectedArtifact {
            sequence,
            artifact_kind: &artifact.planned_download.artifact_kind,
            artifact_id: &artifact.planned_download.artifact_id,
            digest: &artifact.planned_download.digest,
            size_bytes: artifact.planned_download.size_bytes,
        })
        .collect::<Vec<_>>();
    jcs_sha256_hex(&ExpectedArtifactSet {
        schema: EXPECTED_ARTIFACT_SET_SCHEMA,
        release: &facts.candidate_release,
        digest_algorithm: "sha256",
        artifact_count: facts.artifacts.len(),
        artifact_bytes: facts.artifact_bytes,
        artifacts,
    })
}

pub(super) use read::read_fresh_candidate_verification_authority;
