use std::time::Instant;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use super::types::{
    ExtractedComputePluginCandidateArchive, EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA,
    HASHED_EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA, STAGING_EVIDENCE_CANONICALIZATION,
    STAGING_EVIDENCE_DIGEST_ALGORITHM, STAGING_SEAL_EVIDENCE_SCHEMA, STAGING_SEAL_PAYLOAD_SCHEMA,
};
use crate::node_agent_compute_plugin_host::{
    candidate_extraction::{
        COMPUTE_PLUGIN_EXTRACTION_PLAN_SCHEMA, HASHED_COMPUTE_PLUGIN_EXTRACTION_PLAN_SCHEMA,
    },
    manifest_validation::is_sha256,
    plugin_manifest::{COMPUTE_PLUGIN_DIGEST_ALGORITHM, COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION},
    signed_artifact_verification::jcs_sha256_hex,
};

impl ExtractedComputePluginCandidateArchive<'_> {
    /// Re-hashes the exact retained file handles immediately before the caller acquires a fresh
    /// trusted-time observation. Success is only a monotonic barrier, not a durable staged fact.
    pub(in crate::node_agent_compute_plugin_host) fn revalidate_for_staging_store(
        &mut self,
    ) -> Result<Instant> {
        let cancellation = self.verified.snapshot_cancellation_guard();
        cancellation.ensure_current()?;
        self.validate_evidence_closure()?;

        let planned_files = &self.plan.envelope().plan.files;
        let evidence_files = &self.evidence.evidence.files;
        if planned_files.len() != self.files.len() || evidence_files.len() != self.files.len() {
            bail!("COMPUTE_PLUGIN_STAGING_REVALIDATION_FILE_COUNT_CHANGED");
        }

        let mut completed_at = self.completed_at;
        for ((planned, evidence), file) in planned_files
            .iter()
            .zip(evidence_files)
            .zip(self.files.iter_mut())
        {
            let expected_len = u64::try_from(planned.expected_size_bytes)
                .context("COMPUTE_PLUGIN_STAGING_REVALIDATION_FILE_SIZE")?;
            if planned.relative_path != evidence.relative_path
                || planned.expected_digest != evidence.digest
                || planned.expected_size_bytes != evidence.size_bytes
                || file.identity_digest() != evidence.file_identity_digest
                || file.len_bytes() != expected_len
            {
                bail!("COMPUTE_PLUGIN_STAGING_REVALIDATION_FILE_BINDING_CHANGED");
            }
            let hashed = file
                .hash_sha256_and_revalidate(expected_len, || cancellation.ensure_current())
                .map_err(|failure| {
                    failure
                        .into_error()
                        .context("COMPUTE_PLUGIN_STAGING_REVALIDATION_FILE_HASH")
                })?;
            if hashed.digest() != evidence.digest {
                bail!("COMPUTE_PLUGIN_STAGING_REVALIDATION_FILE_DIGEST_CHANGED");
            }
            completed_at = completed_at.max(hashed.completed_at());
        }

        let seal_len = u64::try_from(self.seal_evidence.size_bytes)
            .context("COMPUTE_PLUGIN_STAGING_REVALIDATION_SEAL_SIZE")?;
        if self.seal.identity_digest() != self.seal_evidence.file_identity_digest
            || self.seal.len_bytes() != seal_len
        {
            bail!("COMPUTE_PLUGIN_STAGING_REVALIDATION_SEAL_BINDING_CHANGED");
        }
        let hashed_seal = self
            .seal
            .hash_sha256_and_revalidate(seal_len, || cancellation.ensure_current())
            .map_err(|failure| {
                failure
                    .into_error()
                    .context("COMPUTE_PLUGIN_STAGING_REVALIDATION_SEAL_HASH")
            })?;
        if hashed_seal.digest() != self.seal_evidence.file_digest {
            bail!("COMPUTE_PLUGIN_STAGING_REVALIDATION_SEAL_DIGEST_CHANGED");
        }
        cancellation.ensure_current()?;
        Ok(completed_at.max(hashed_seal.completed_at()))
    }

    fn validate_evidence_closure(&self) -> Result<()> {
        let plan = self.plan.envelope();
        let evidence = &self.evidence;
        let seal = &self.seal_evidence;
        let payload = &seal.payload;
        let seal_payload_bytes =
            serde_json::to_vec(payload).context("COMPUTE_PLUGIN_STAGING_REVALIDATION_SEAL_JSON")?;
        let expected_seal_file_digest = hex::encode(Sha256::digest(&seal_payload_bytes));
        if plan.schema != HASHED_COMPUTE_PLUGIN_EXTRACTION_PLAN_SCHEMA
            || plan.plan.schema != COMPUTE_PLUGIN_EXTRACTION_PLAN_SCHEMA
            || plan.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
            || plan.plan_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
            || !is_sha256(&plan.plan_digest)
            || jcs_sha256_hex(&plan.plan)? != plan.plan_digest
            || evidence.schema != HASHED_EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA
            || evidence.evidence.schema != EXTRACTED_ARCHIVE_EVIDENCE_SCHEMA
            || evidence.canonicalization != STAGING_EVIDENCE_CANONICALIZATION
            || evidence.digest_algorithm != STAGING_EVIDENCE_DIGEST_ALGORITHM
            || !is_sha256(&evidence.evidence_digest)
            || jcs_sha256_hex(&evidence.evidence)? != evidence.evidence_digest
            || seal.schema != STAGING_SEAL_EVIDENCE_SCHEMA
            || payload.schema != STAGING_SEAL_PAYLOAD_SCHEMA
            || seal.canonicalization != STAGING_EVIDENCE_CANONICALIZATION
            || seal.digest_algorithm != STAGING_EVIDENCE_DIGEST_ALGORITHM
            || !is_sha256(&seal.payload_digest)
            || !is_sha256(&seal.file_digest)
            || !is_sha256(&seal.file_identity_digest)
            || jcs_sha256_hex(payload)? != seal.payload_digest
            || seal.file_digest != expected_seal_file_digest
            || usize::try_from(seal.size_bytes).ok() != Some(seal_payload_bytes.len())
        {
            bail!("COMPUTE_PLUGIN_STAGING_REVALIDATION_ENVELOPE_CHANGED");
        }
        if evidence.evidence.installation_id_digest != self.verified.installation_id_digest()
            || evidence.evidence.root_identity_digest != self.staging.root_identity_digest()
            || evidence.evidence.candidate_token_digest != self.verified.candidate_token_digest()
            || evidence.evidence.staging_run_digest != self.staging.staging_run_digest()
            || evidence.evidence.extraction_plan_digest != plan.plan_digest
            || evidence.evidence.extracted_file_count
                != i64::try_from(self.files.len())
                    .context("COMPUTE_PLUGIN_STAGING_REVALIDATION_FILE_COUNT")?
            || evidence.evidence.extracted_file_count
                != i64::try_from(plan.plan.files.len())
                    .context("COMPUTE_PLUGIN_STAGING_REVALIDATION_PLAN_FILE_COUNT")?
            || evidence.evidence.extracted_bytes != plan.plan.unpacked_size_bytes
            || payload.installation_id_digest != evidence.evidence.installation_id_digest
            || payload.root_identity_digest != evidence.evidence.root_identity_digest
            || payload.candidate_token_digest != evidence.evidence.candidate_token_digest
            || payload.staging_run_digest != evidence.evidence.staging_run_digest
            || payload.extraction_plan_digest != plan.plan_digest
            || payload.extraction_evidence_digest != evidence.evidence_digest
            || payload.extracted_file_count != evidence.evidence.extracted_file_count
            || payload.extracted_bytes != evidence.evidence.extracted_bytes
        {
            bail!("COMPUTE_PLUGIN_STAGING_REVALIDATION_CLOSURE_CHANGED");
        }
        Ok(())
    }
}
