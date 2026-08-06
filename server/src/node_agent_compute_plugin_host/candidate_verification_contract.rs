use std::{fmt, time::Instant};

use anyhow::{bail, Context, Result};
use serde::Serialize;
use uuid::Uuid;

use crate::node_agent_managed_fs::PinnedManagedFile;

use super::{
    fetch_contract::ComputePluginFetchCancellationGuard,
    fetch_file::{
        pin_existing_candidate_downloads, PinnedComputePluginCandidateDownloads,
        PinnedComputePluginRoot,
    },
    install_plan_admission::{
        validate_inventory, validate_live_binding, validate_plan_window,
        AdmittedComputePluginInstallPlan,
    },
    install_plan_admission_validation::is_identifier,
    lifecycle::SLOT_DOWNLOADING,
    local_authority::{
        ComputePluginCandidateHandle, ComputePluginCandidateVerificationAuthorityFacts,
        ComputePluginFetchAuthoritySession, ComputePluginPostPinVerificationAuthoritySession,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

mod begin;
mod hash;
mod hash_budget;
mod post_hash;
mod recovery;
mod resolution;

pub(in crate::node_agent_compute_plugin_host) use begin::{
    begin_candidate_verification, AuthorizedComputePluginCandidateArtifactSet,
    BeginCandidateVerificationFailure, CandidateVerificationBeginMutationPhase,
    CandidateVerificationBeginRecoveryCustody, UnclaimedCandidateArtifactSetCustody,
    ValidatedCandidateVerificationBeginPermit,
};
pub(in crate::node_agent_compute_plugin_host) use hash::{
    abandon_hashed_candidate_artifact_set, hash_authorized_candidate_artifact_set,
    CandidateArtifactSetHashDisposition, CandidateVerificationHashFailure,
    CandidateVerificationHashPhase, HashedComputePluginCandidateArtifactSet,
};
pub(in crate::node_agent_compute_plugin_host) use hash_budget::CandidateVerificationHashBudget;
pub(in crate::node_agent_compute_plugin_host) use post_hash::{
    abandon_trusted_hashed_candidate_artifact_set, bind_hashed_candidate_artifact_set,
    CandidateVerificationPostHashBindingFailure, CandidateVerificationPostHashBindingPhase,
    TrustedHashedComputePluginCandidateArtifactSet,
};
pub(in crate::node_agent_compute_plugin_host) use recovery::{
    abort_recovered_candidate_verification, inspect_candidate_verification_outcome,
    CandidateVerificationRecoveryAbortFailure, CandidateVerificationRecoveryAbortPhase,
    ComputePluginCandidateVerificationDigestMismatch,
    ComputePluginCandidateVerificationInitialAbsence, ComputePluginCandidateVerificationOutcome,
    ComputePluginCandidateVerificationOutcomeKind, ComputePluginCandidateVerificationRecoveryKey,
    ResolvedCandidateArtifactSetCustody, ValidatedCandidateVerificationRecoveryAbortPermit,
};
pub(in crate::node_agent_compute_plugin_host) use resolution::{
    adopt_recovered_candidate_verification_resolution,
    resolve_trusted_hashed_candidate_artifact_set, CandidateVerificationResolutionAdoptionFailure,
    CandidateVerificationResolutionAdoptionPhase, CandidateVerificationResolutionCustody,
    CandidateVerificationResolutionFailure, CandidateVerificationResolutionPhase,
    RejectedComputePluginCandidateArtifactSetCustody,
    ValidatedCandidateVerificationResolutionPermit, VerifiedComputePluginCandidateArtifactSet,
};

const FILE_SET_BINDING_SCHEMA: &str = "elon.compute_plugin.candidate_file_set_binding.v1";

/// Existing candidate files pinned before any verification Store mutation. This capability is
/// non-cloneable and keeps every share-none file handle alive; it is not yet authorized to hash or
/// create a verification run.
pub(in crate::node_agent_compute_plugin_host) struct PinnedComputePluginCandidateArtifactSet {
    verification_id: String,
    candidate_token: String,
    discovery_authority: ComputePluginCandidateVerificationAuthorityFacts,
    installation_binding_digest: String,
    root_identity_digest: String,
    file_set_binding_digest: String,
    pin_completed_at: Instant,
    cancellation_guard: ComputePluginFetchCancellationGuard,
    artifacts: Vec<PinnedComputePluginCandidateArtifact>,
    _candidate_directory: PinnedComputePluginCandidateDownloads,
}

struct PinnedComputePluginCandidateArtifact {
    ordinal: usize,
    item_index: usize,
    part_relative_path: String,
    expected_digest: String,
    expected_len: u64,
    file: PinnedManagedFile,
}

pub(in crate::node_agent_compute_plugin_host) struct PinnedComputePluginCleanupArtifact {
    pub(in crate::node_agent_compute_plugin_host) logical_path: String,
    pub(in crate::node_agent_compute_plugin_host) expected_digest: String,
    pub(in crate::node_agent_compute_plugin_host) file: PinnedManagedFile,
}

/// A new authenticated time observation has re-read the same durable projection after pinning.
/// The begin contract consumes this value and still performs a third read and CAS inside one
/// `BEGIN IMMEDIATE` transaction before returning a hash authorization.
pub(in crate::node_agent_compute_plugin_host) struct ObservedComputePluginCandidateArtifactSet<
    'authority,
> {
    pinned: PinnedComputePluginCandidateArtifactSet,
    current_authority: ComputePluginCandidateVerificationAuthorityFacts,
    authority_session: ComputePluginPostPinVerificationAuthoritySession<'authority>,
}

impl fmt::Debug for PinnedComputePluginCandidateArtifactSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedComputePluginCandidateArtifactSet")
            .field("verification_id", &"<redacted>")
            .field("candidate", &"<redacted>")
            .field("artifact_count", &self.artifacts.len())
            .field("file_set_binding_digest", &"<redacted>")
            .field("pin_completed_at", &"<monotonic>")
            .finish()
    }
}

impl PinnedComputePluginCandidateArtifactSet {
    fn into_cleanup_parts(
        self,
    ) -> (
        Vec<PinnedComputePluginCleanupArtifact>,
        PinnedComputePluginCandidateDownloads,
    ) {
        let artifacts = self
            .artifacts
            .into_iter()
            .map(|artifact| PinnedComputePluginCleanupArtifact {
                logical_path: artifact.part_relative_path,
                expected_digest: artifact.expected_digest,
                file: artifact.file,
            })
            .collect();
        (artifacts, self._candidate_directory)
    }
}

impl fmt::Debug for ObservedComputePluginCandidateArtifactSet<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ObservedComputePluginCandidateArtifactSet")
            .field("pinned", &self.pinned)
            .field("authority", &"<fresh-durable-projection>")
            .field("session", &"<post-pin-trusted-time>")
            .finish()
    }
}

pub(in crate::node_agent_compute_plugin_host) fn pin_candidate_artifact_set(
    authority_session: &ComputePluginFetchAuthoritySession<'_>,
    admitted: &AdmittedComputePluginInstallPlan,
    candidate: &ComputePluginCandidateHandle,
    root: &PinnedComputePluginRoot,
    cancellation_guard: ComputePluginFetchCancellationGuard,
) -> Result<PinnedComputePluginCandidateArtifactSet> {
    authority_session.validate_fetch_cancellation_guard(&cancellation_guard)?;
    cancellation_guard.ensure_current()?;
    let facts = authority_session.read_fresh_candidate_verification_authority(
        &admitted.plan().plan_id,
        admitted.plan_digest(),
        candidate.candidate_token(),
    )?;
    cancellation_guard.ensure_current()?;
    validate_candidate_authority(admitted, candidate, &facts)?;

    let candidate_directory = pin_existing_candidate_downloads(root, &facts.candidate_token_digest)
        .context("COMPUTE_PLUGIN_VERIFICATION_DIRECTORY_PIN")?;
    if candidate_directory.installation_id_digest() != authority_session.installation_id_digest()
        || !is_sha256(candidate_directory.root_identity_digest())
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_MANAGED_ROOT_CHANGED");
    }
    let mut pinned_artifacts = Vec::with_capacity(facts.artifacts.len());
    for artifact in &facts.artifacts {
        cancellation_guard.ensure_current()?;
        let file = candidate_directory
            .open_existing_artifact(&artifact.part_relative_path)
            .with_context(|| {
                format!(
                    "COMPUTE_PLUGIN_VERIFICATION_FILE_OPEN ordinal={}",
                    artifact.ordinal
                )
            })?;
        let expected_len = u64::try_from(artifact.planned_download.size_bytes)
            .context("COMPUTE_PLUGIN_VERIFICATION_FILE_SIZE_RANGE")?;
        if file.len_bytes() != expected_len || !is_sha256(file.identity_digest()) {
            bail!("COMPUTE_PLUGIN_VERIFICATION_FILE_IDENTITY_CHANGED");
        }
        pinned_artifacts.push(PinnedComputePluginCandidateArtifact {
            ordinal: artifact.ordinal,
            item_index: artifact.item_index,
            part_relative_path: artifact.part_relative_path.clone(),
            expected_digest: artifact.planned_download.digest.clone(),
            expected_len,
            file,
        });
    }
    cancellation_guard.ensure_current()?;
    let verification_id = format!("cvf_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    if !is_identifier(&verification_id) {
        bail!("COMPUTE_PLUGIN_VERIFICATION_ID_GENERATION_FAILED");
    }
    let installation_binding_digest = candidate_directory.installation_id_digest().to_string();
    let root_identity_digest = candidate_directory.root_identity_digest().to_string();
    let file_set_binding_digest = compute_file_set_binding_digest(
        &verification_id,
        &facts,
        &installation_binding_digest,
        &root_identity_digest,
        &pinned_artifacts,
    )?;
    Ok(PinnedComputePluginCandidateArtifactSet {
        verification_id,
        candidate_token: candidate.candidate_token().to_string(),
        discovery_authority: facts,
        installation_binding_digest,
        root_identity_digest,
        file_set_binding_digest,
        pin_completed_at: Instant::now(),
        cancellation_guard,
        artifacts: pinned_artifacts,
        _candidate_directory: candidate_directory,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn observe_pinned_candidate_artifact_set<
    'authority,
>(
    mut pinned: PinnedComputePluginCandidateArtifactSet,
    authority_session: ComputePluginPostPinVerificationAuthoritySession<'authority>,
) -> Result<ObservedComputePluginCandidateArtifactSet<'authority>> {
    let current = authority_session.read_fresh_after_pin(
        pinned.pin_completed_at,
        &pinned.cancellation_guard,
        &pinned.discovery_authority.applied_plan_id,
        &pinned.discovery_authority.applied_plan_digest,
        &pinned.candidate_token,
    )?;
    if !pinned.discovery_authority.same_durable_projection(&current) {
        bail!("COMPUTE_PLUGIN_VERIFICATION_AUTHORITY_CHANGED_AFTER_PIN");
    }
    for artifact in &mut pinned.artifacts {
        pinned.cancellation_guard.ensure_current()?;
        artifact.file.revalidate_exact_len(artifact.expected_len)?;
    }
    pinned.cancellation_guard.ensure_current()?;
    let rebound = compute_file_set_binding_digest(
        &pinned.verification_id,
        &current,
        &pinned.installation_binding_digest,
        &pinned.root_identity_digest,
        &pinned.artifacts,
    )?;
    if rebound != pinned.file_set_binding_digest {
        bail!("COMPUTE_PLUGIN_VERIFICATION_FILE_SET_BINDING_CHANGED");
    }
    Ok(ObservedComputePluginCandidateArtifactSet {
        pinned,
        current_authority: current,
        authority_session,
    })
}

fn validate_candidate_authority(
    admitted: &AdmittedComputePluginInstallPlan,
    candidate: &ComputePluginCandidateHandle,
    facts: &ComputePluginCandidateVerificationAuthorityFacts,
) -> Result<()> {
    validate_plan_window(admitted.plan(), facts.trusted_now.clone(), false)?;
    validate_live_binding(admitted.plan(), &facts.live)?;
    validate_inventory(&facts.inventory, facts.trusted_now.clone())?;
    let expected_application_revision = admitted
        .plan()
        .expected_inventory_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_REVISION_OVERFLOW"))?;
    let expected_downloads = admitted
        .downloads()
        .iter()
        .filter(|download| download.release == facts.candidate_release)
        .collect::<Vec<_>>();
    let artifacts_match = facts.artifacts.len() == expected_downloads.len()
        && facts
            .artifacts
            .iter()
            .zip(expected_downloads)
            .all(|(artifact, expected)| {
                artifact.ordinal == expected.ordinal
                    && artifact.item_index == expected.item_index
                    && artifact.planned_download == expected.download
            });
    let candidate_record_matches = facts.inventory.plugins.iter().any(|record| {
        record.plugin_id == facts.candidate_plugin_id
            && record.candidate_slot_ref.as_deref() == Some(facts.candidate_slot_ref.as_str())
            && record.slots.iter().any(|slot| {
                slot.slot_ref == facts.candidate_slot_ref
                    && slot.release == facts.candidate_release
                    && slot.phase == SLOT_DOWNLOADING
            })
            && facts.candidate_generation > record.install_generation
    });
    if facts.applied_plan_id != admitted.plan().plan_id
        || facts.applied_plan_digest != admitted.plan_digest()
        || facts.application_inventory_revision != expected_application_revision
        || facts.execution_inventory_revision != facts.inventory.inventory_revision
        || facts.execution_inventory_revision < facts.application_inventory_revision
        || facts.authority_state_revision <= 0
        || facts.inventory_digest != jcs_sha256_hex(&facts.inventory)?
        || !is_sha256(&facts.installation_id_digest)
        || facts.authority_epoch <= 0
        || facts.process_owner_epoch <= 0
        || facts.observed_trusted_time_high_water_ms < 0
        || facts.observed_trusted_time_high_water_ms > facts.trusted_now.timestamp_millis()
        || facts.candidate_token_digest != candidate.candidate_token_digest()
        || facts.candidate_generation != candidate.candidate_generation()
        || facts.candidate_owner_plan_id != admitted.plan().plan_id
        || facts.candidate_owner_plan_digest != admitted.plan_digest()
        || facts.candidate_application_inventory_revision != expected_application_revision
        || facts.candidate_state != "owned"
        || facts.candidate_plugin_id != candidate.plugin_id()
        || facts.candidate_slot_ref != candidate.slot_ref()
        || facts.candidate_created_at_ms < 0
        || facts.candidate_release.plugin_id != facts.candidate_plugin_id
        || facts.next_verification_generation <= 0
        || facts.artifacts.is_empty()
        || facts.artifact_bytes <= 0
        || !artifacts_match
        || !candidate_record_matches
        || facts.expected_artifact_set_digest != facts.recompute_expected_artifact_set_digest()?
        || !is_sha256(&facts.expected_artifact_set_digest)
        || jcs_sha256_hex(&candidate.candidate_token())? != facts.candidate_token_digest
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_AUTHORITY_INVALID");
    }
    Ok(())
}

#[derive(Serialize)]
struct FileSetBinding<'binding> {
    schema: &'static str,
    verification_id: &'binding str,
    verification_generation: i64,
    installation_binding_digest: &'binding str,
    root_identity_digest: &'binding str,
    owner_plan_id: &'binding str,
    owner_plan_digest: &'binding str,
    application_inventory_revision: i64,
    execution_inventory_revision: i64,
    authority_state_revision: i64,
    inventory_digest: &'binding str,
    authority_epoch: i64,
    process_owner_epoch: i64,
    candidate_token_digest: &'binding str,
    candidate_generation: i64,
    slot_ref: &'binding str,
    expected_artifact_set_digest: &'binding str,
    artifact_count: usize,
    artifact_bytes: i64,
    artifacts: Vec<FileBinding<'binding>>,
}

#[derive(Serialize)]
struct FileBinding<'binding> {
    ordinal: usize,
    item_index: usize,
    part_relative_path: &'binding str,
    expected_digest: &'binding str,
    size_bytes: u64,
    file_identity_digest: &'binding str,
}

fn compute_file_set_binding_digest(
    verification_id: &str,
    facts: &ComputePluginCandidateVerificationAuthorityFacts,
    installation_binding_digest: &str,
    root_identity_digest: &str,
    artifacts: &[PinnedComputePluginCandidateArtifact],
) -> Result<String> {
    let bindings = artifacts
        .iter()
        .map(|artifact| FileBinding {
            ordinal: artifact.ordinal,
            item_index: artifact.item_index,
            part_relative_path: &artifact.part_relative_path,
            expected_digest: &artifact.expected_digest,
            size_bytes: artifact.expected_len,
            file_identity_digest: artifact.file.identity_digest(),
        })
        .collect();
    jcs_sha256_hex(&FileSetBinding {
        schema: FILE_SET_BINDING_SCHEMA,
        verification_id,
        verification_generation: facts.next_verification_generation,
        installation_binding_digest,
        root_identity_digest,
        owner_plan_id: &facts.candidate_owner_plan_id,
        owner_plan_digest: &facts.candidate_owner_plan_digest,
        application_inventory_revision: facts.candidate_application_inventory_revision,
        execution_inventory_revision: facts.execution_inventory_revision,
        authority_state_revision: facts.authority_state_revision,
        inventory_digest: &facts.inventory_digest,
        authority_epoch: facts.authority_epoch,
        process_owner_epoch: facts.process_owner_epoch,
        candidate_token_digest: &facts.candidate_token_digest,
        candidate_generation: facts.candidate_generation,
        slot_ref: &facts.candidate_slot_ref,
        expected_artifact_set_digest: &facts.expected_artifact_set_digest,
        artifact_count: artifacts.len(),
        artifact_bytes: facts.artifact_bytes,
        artifacts: bindings,
    })
}
