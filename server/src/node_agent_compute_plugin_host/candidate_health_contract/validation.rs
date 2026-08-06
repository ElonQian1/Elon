use std::time::Instant;

use anyhow::{bail, Context, Result};
use chrono::{Duration, SecondsFormat};
use serde::Serialize;
use uuid::Uuid;

use super::{
    probe_state::CandidateHealthProbeState, CandidateHealthBinding, CandidateHealthEvaluation,
    CandidateHealthEvaluationStartFailure, CandidateHealthFinalizationFailure,
    CandidateHealthProbeObservation, CandidateHealthProgress,
    ComputePluginCandidateHealthObservation, HashedComputePluginCandidateHealthObservation,
    ValidatedCandidateHealthPublication, CANDIDATE_HEALTHY, CANDIDATE_HEALTH_CANONICALIZATION,
    CANDIDATE_HEALTH_DIGEST_ALGORITHM, CANDIDATE_HEALTH_OBSERVATION_SCHEMA,
    CANDIDATE_HEALTH_TRANSCRIPT_SCHEMA, HASHED_CANDIDATE_HEALTH_OBSERVATION_SCHEMA,
    MAX_CANDIDATE_HEALTH_INTERVAL_MS, MAX_CANDIDATE_HEALTH_LIFETIME_SECONDS,
    MAX_CANDIDATE_HEALTH_PROBES, MAX_CANDIDATE_HEALTH_TIMEOUT_MS,
};
use crate::node_agent_compute_plugin_host::{
    candidate_staging_contract::StagedComputePluginCandidateArchive,
    identity::ComputePluginInstallationIdentity,
    install_plan_admission_validation::is_identifier,
    lifecycle::SLOT_STAGED,
    manifest_validation::{is_sha256, ValidatedComputePluginManifest},
    signed_artifact_verification::jcs_sha256_hex,
    trusted_time::ComputePluginTrustedTimeObservation,
};

#[derive(Serialize)]
struct CandidateHealthTranscriptAnchor<'a> {
    schema: &'static str,
    evaluation_id: &'a str,
    installation_id_digest: &'a str,
    candidate_token_digest: &'a str,
    staging_id: &'a str,
    staging_receipt_digest: &'a str,
    staging_run_digest: &'a str,
    root_identity_digest: &'a str,
    extraction_plan_digest: &'a str,
    runner_digest: &'a str,
    protocol: &'a str,
}

pub(super) fn begin_evaluation<'root>(
    staged: StagedComputePluginCandidateArchive<'root>,
    manifest: &ValidatedComputePluginManifest,
    installation: &ComputePluginInstallationIdentity,
) -> std::result::Result<
    CandidateHealthEvaluation<'root>,
    CandidateHealthEvaluationStartFailure<'root>,
> {
    let binding = match validate_start_binding(&staged, manifest, installation) {
        Ok(binding) => binding,
        Err(error) => return Err(CandidateHealthEvaluationStartFailure { error, staged }),
    };
    let evaluation_id = format!("che_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let transcript_digest = match initial_transcript_digest(&evaluation_id, &binding) {
        Ok(digest) if is_identifier(&evaluation_id) => digest,
        Ok(_) => {
            return Err(CandidateHealthEvaluationStartFailure {
                error: anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_ID_INVALID"),
                staged,
            })
        }
        Err(error) => return Err(CandidateHealthEvaluationStartFailure { error, staged }),
    };

    Ok(CandidateHealthEvaluation {
        staged,
        evaluation_id,
        binding,
        probes: CandidateHealthProbeState::new(transcript_digest),
        started_at: Instant::now(),
        last_probe_at: None,
    })
}

pub(super) fn record_probe(
    evaluation: &mut CandidateHealthEvaluation<'_>,
    observation: CandidateHealthProbeObservation,
) -> Result<CandidateHealthProgress> {
    let progress = evaluation.probes.record(
        &evaluation.evaluation_id,
        evaluation.binding.timeout_ms,
        evaluation.binding.required_consecutive_successes,
        evaluation.binding.unhealthy_after_failures,
        observation,
    )?;
    evaluation.last_probe_at = Some(Instant::now());
    Ok(progress)
}

pub(super) fn finalize_evaluation<'root>(
    evaluation: CandidateHealthEvaluation<'root>,
    trusted_time: ComputePluginTrustedTimeObservation,
) -> std::result::Result<
    ValidatedCandidateHealthPublication<'root>,
    CandidateHealthFinalizationFailure<'root>,
> {
    match build_health_observation(&evaluation, &trusted_time) {
        Ok(observation) => Ok(ValidatedCandidateHealthPublication {
            staged: evaluation.staged,
            observation,
            trusted_time,
        }),
        Err(error) => Err(CandidateHealthFinalizationFailure { error, evaluation }),
    }
}

fn validate_start_binding(
    staged: &StagedComputePluginCandidateArchive<'_>,
    manifest: &ValidatedComputePluginManifest,
    installation: &ComputePluginInstallationIdentity,
) -> Result<CandidateHealthBinding> {
    staged
        .archive()
        .snapshot_cancellation_guard()
        .ensure_current()?;
    let archive = staged.archive();
    let plan = archive.plan().envelope();
    let evidence = &archive.evidence().evidence;
    let receipt = staged.receipt().receipt();
    let release = manifest.release_ref();
    let entrypoint = &manifest.manifest().entrypoint;
    let health = &entrypoint.health_check;
    let package_file = manifest
        .manifest()
        .package
        .files
        .iter()
        .find(|file| file.relative_path == entrypoint.relative_path)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RUNNER_MISSING"))?;
    let planned_file = plan
        .plan
        .files
        .iter()
        .find(|file| file.relative_path == entrypoint.relative_path)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_PLAN_RUNNER_MISSING"))?;

    if release != plan.plan.release
        || plan.plan.publisher_key_fingerprint != manifest.verification_key_fingerprint()
        || plan.plan.package_digest != manifest.manifest().package.package_digest
        || installation.digest() != evidence.installation_id_digest
        || staged.recovery_key().installation_id_digest() != installation.digest()
        || staged.recovery_key().candidate_token_digest() != receipt.candidate_token_digest()
        || staged.recovery_key().staging_id() != receipt.staging_id()
        || staged.recovery_key().staging_run_digest() != receipt.staging_run_digest()
        || receipt.candidate_token_digest() != evidence.candidate_token_digest
        || receipt.staging_run_digest() != evidence.staging_run_digest
        || receipt.slot_phase_after() != SLOT_STAGED
        || !is_sha256(staged.receipt().receipt_digest())
        || !is_sha256(&evidence.root_identity_digest)
        || !is_sha256(&plan.plan_digest)
        || receipt.authority_state_revision_after() <= 0
        || receipt.inventory_revision_after() <= 0
        || !is_sha256(receipt.inventory_digest_after())
        || receipt.authority_epoch_after() <= 0
        || staged.recovery_key().process_owner_epoch() <= 0
        || receipt.staged_at_ms() <= 0
        || !package_file.executable
        || !planned_file.executable
        || package_file.digest != planned_file.expected_digest
        || package_file.size_bytes != planned_file.expected_size_bytes
        || health.timeout_ms <= 0
        || health.timeout_ms > MAX_CANDIDATE_HEALTH_TIMEOUT_MS
        || health.interval_ms <= 0
        || health.interval_ms > MAX_CANDIDATE_HEALTH_INTERVAL_MS
        || health.healthy_after_successes <= 0
        || health.healthy_after_successes > MAX_CANDIDATE_HEALTH_PROBES
        || health.unhealthy_after_failures <= 0
        || health.unhealthy_after_failures > MAX_CANDIDATE_HEALTH_PROBES
        || !is_identifier(&health.protocol)
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_BINDING_INVALID");
    }

    Ok(CandidateHealthBinding {
        installation_id_digest: installation.digest().to_string(),
        candidate_token_digest: receipt.candidate_token_digest().to_string(),
        staging_id: receipt.staging_id().to_string(),
        staging_receipt_digest: staged.receipt().receipt_digest().to_string(),
        staging_run_digest: receipt.staging_run_digest().to_string(),
        root_identity_digest: evidence.root_identity_digest.clone(),
        extraction_plan_digest: plan.plan_digest.clone(),
        release,
        entrypoint_relative_path: entrypoint.relative_path.clone(),
        runner_digest: package_file.digest.clone(),
        protocol: health.protocol.clone(),
        timeout_ms: health.timeout_ms,
        interval_ms: health.interval_ms,
        required_consecutive_successes: health.healthy_after_successes,
        unhealthy_after_failures: health.unhealthy_after_failures,
        clock_epoch_digest: staged.recovery_key().clock_epoch_digest().to_string(),
        process_owner_epoch: staged.recovery_key().process_owner_epoch(),
        authority_state_revision: receipt.authority_state_revision_after(),
        inventory_revision: receipt.inventory_revision_after(),
        inventory_digest: receipt.inventory_digest_after().to_string(),
        authority_epoch: receipt.authority_epoch_after(),
        staged_at_ms: receipt.staged_at_ms(),
    })
}

fn initial_transcript_digest(
    evaluation_id: &str,
    binding: &CandidateHealthBinding,
) -> Result<String> {
    jcs_sha256_hex(&CandidateHealthTranscriptAnchor {
        schema: CANDIDATE_HEALTH_TRANSCRIPT_SCHEMA,
        evaluation_id,
        installation_id_digest: &binding.installation_id_digest,
        candidate_token_digest: &binding.candidate_token_digest,
        staging_id: &binding.staging_id,
        staging_receipt_digest: &binding.staging_receipt_digest,
        staging_run_digest: &binding.staging_run_digest,
        root_identity_digest: &binding.root_identity_digest,
        extraction_plan_digest: &binding.extraction_plan_digest,
        runner_digest: &binding.runner_digest,
        protocol: &binding.protocol,
    })
}

fn build_health_observation(
    evaluation: &CandidateHealthEvaluation<'_>,
    trusted_time: &ComputePluginTrustedTimeObservation,
) -> Result<HashedComputePluginCandidateHealthObservation> {
    let last_probe_at = evaluation
        .last_probe_at
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_PROBE_MISSING"))?;
    let progress = evaluation.probes.progress();
    if !progress.healthy
        || progress.terminal_unhealthy
        || progress.consecutive_successes < evaluation.binding.required_consecutive_successes
        || trusted_time.installation_id_digest() != evaluation.binding.installation_id_digest
        || trusted_time.clock_epoch_digest() != evaluation.binding.clock_epoch_digest
        || trusted_time.trusted_now().timestamp_millis() <= evaluation.binding.staged_at_ms
        || trusted_time.observed_at() <= last_probe_at
        || last_probe_at < evaluation.started_at
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_NOT_FINALIZABLE");
    }
    evaluation
        .staged
        .archive()
        .snapshot_cancellation_guard()
        .ensure_current()?;

    let observed_at = trusted_time
        .trusted_now()
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let expires_at = (trusted_time.trusted_now().to_owned()
        + Duration::seconds(MAX_CANDIDATE_HEALTH_LIFETIME_SECONDS))
    .to_rfc3339_opts(SecondsFormat::Millis, true);
    let observation = ComputePluginCandidateHealthObservation {
        schema: CANDIDATE_HEALTH_OBSERVATION_SCHEMA.to_string(),
        evaluation_id: evaluation.evaluation_id.clone(),
        installation_id_digest: evaluation.binding.installation_id_digest.clone(),
        candidate_token_digest: evaluation.binding.candidate_token_digest.clone(),
        staging_id: evaluation.binding.staging_id.clone(),
        staging_receipt_digest: evaluation.binding.staging_receipt_digest.clone(),
        staging_run_digest: evaluation.binding.staging_run_digest.clone(),
        root_identity_digest: evaluation.binding.root_identity_digest.clone(),
        extraction_plan_digest: evaluation.binding.extraction_plan_digest.clone(),
        release: evaluation.binding.release.clone(),
        entrypoint_relative_path: evaluation.binding.entrypoint_relative_path.clone(),
        runner_digest: evaluation.binding.runner_digest.clone(),
        protocol: evaluation.binding.protocol.clone(),
        timeout_ms: evaluation.binding.timeout_ms,
        interval_ms: evaluation.binding.interval_ms,
        required_consecutive_successes: evaluation.binding.required_consecutive_successes,
        unhealthy_after_failures: evaluation.binding.unhealthy_after_failures,
        attempted_probes: progress.attempted_probes,
        successful_probes: progress.successful_probes,
        consecutive_successes: progress.consecutive_successes,
        probe_transcript_digest: evaluation.probes.transcript_digest().to_string(),
        reason_codes: evaluation.probes.reason_codes(),
        status: CANDIDATE_HEALTHY.to_string(),
        observed_at,
        expires_at,
        clock_epoch_digest: trusted_time.clock_epoch_digest().to_string(),
        process_owner_epoch: evaluation.binding.process_owner_epoch,
        authority_state_revision: evaluation.binding.authority_state_revision,
        inventory_revision: evaluation.binding.inventory_revision,
        inventory_digest: evaluation.binding.inventory_digest.clone(),
        authority_epoch: evaluation.binding.authority_epoch,
        time_authority_id: trusted_time.time_authority_id().to_string(),
        time_attestation_digest: trusted_time.attestation_digest().to_string(),
        time_attestation_sequence: trusted_time.attestation_sequence(),
        time_signing_key_fingerprint: trusted_time.signing_key_fingerprint().to_string(),
    };
    let observation_digest = jcs_sha256_hex(&observation)?;
    Ok(HashedComputePluginCandidateHealthObservation {
        schema: HASHED_CANDIDATE_HEALTH_OBSERVATION_SCHEMA.to_string(),
        observation,
        canonicalization: CANDIDATE_HEALTH_CANONICALIZATION.to_string(),
        digest_algorithm: CANDIDATE_HEALTH_DIGEST_ALGORITHM.to_string(),
        observation_digest,
    })
}
