use std::fmt;

use anyhow::{bail, Result};

use super::CandidateArtifactSetHashDisposition;
use crate::node_agent_compute_plugin_host::manifest_validation::is_sha256;

pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) struct CandidateArtifactSetHashEvidence
{
    disposition: CandidateArtifactSetHashDisposition,
    hashed_artifact_count: usize,
    hashed_artifact_bytes: u64,
    observed_artifact_set_digest: String,
    mismatch: Option<CandidateArtifactDigestMismatch>,
}

pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) struct CandidateArtifactDigestMismatch
{
    ordinal: usize,
    expected_digest: String,
    observed_digest: String,
}

impl CandidateArtifactSetHashEvidence {
    pub(super) fn new(
        disposition: CandidateArtifactSetHashDisposition,
        hashed_artifact_count: usize,
        hashed_artifact_bytes: u64,
        observed_artifact_set_digest: String,
        mismatch: Option<CandidateArtifactDigestMismatch>,
    ) -> Self {
        Self {
            disposition,
            hashed_artifact_count,
            hashed_artifact_bytes,
            observed_artifact_set_digest,
            mismatch,
        }
    }

    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) fn disposition(
        &self,
    ) -> CandidateArtifactSetHashDisposition {
        self.disposition
    }

    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) fn observed_artifact_set_digest(
        &self,
    ) -> &str {
        &self.observed_artifact_set_digest
    }

    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) fn mismatch_ordinal(
        &self,
    ) -> Option<usize> {
        self.mismatch.as_ref().map(|mismatch| mismatch.ordinal)
    }

    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) fn mismatch_observed_digest(
        &self,
    ) -> Option<&str> {
        self.mismatch
            .as_ref()
            .map(|mismatch| mismatch.observed_digest.as_str())
    }

    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) fn mismatch_expected_digest(
        &self,
    ) -> Option<&str> {
        self.mismatch
            .as_ref()
            .map(|mismatch| mismatch.expected_digest.as_str())
    }

    pub(in crate::node_agent_compute_plugin_host::candidate_verification_contract) fn validate(
        &self,
        key: &super::super::ComputePluginCandidateVerificationRecoveryKey,
        pinned: &super::super::PinnedComputePluginCandidateArtifactSet,
    ) -> Result<()> {
        if self.hashed_artifact_count != key.artifact_count()
            || i64::try_from(self.hashed_artifact_bytes).ok() != Some(key.artifact_bytes())
            || !is_sha256(&self.observed_artifact_set_digest)
        {
            bail!("COMPUTE_PLUGIN_VERIFICATION_HASH_EVIDENCE_INVALID");
        }
        match (self.disposition, self.mismatch.as_ref()) {
            (CandidateArtifactSetHashDisposition::Matched, None) => Ok(()),
            (CandidateArtifactSetHashDisposition::DigestMismatch, Some(mismatch))
                if pinned.artifacts.iter().any(|artifact| {
                    artifact.ordinal == mismatch.ordinal
                        && artifact.expected_digest == mismatch.expected_digest
                }) && is_sha256(&mismatch.expected_digest)
                    && is_sha256(&mismatch.observed_digest)
                    && mismatch.expected_digest != mismatch.observed_digest =>
            {
                Ok(())
            }
            _ => bail!("COMPUTE_PLUGIN_VERIFICATION_HASH_DISPOSITION_INVALID"),
        }
    }
}

impl CandidateArtifactDigestMismatch {
    pub(super) fn new(ordinal: usize, expected_digest: String, observed_digest: String) -> Self {
        Self {
            ordinal,
            expected_digest,
            observed_digest,
        }
    }
}

impl fmt::Debug for CandidateArtifactSetHashEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateArtifactSetHashEvidence")
            .field("disposition", &self.disposition)
            .field("hashed_artifact_count", &self.hashed_artifact_count)
            .field("hashed_artifact_bytes", &self.hashed_artifact_bytes)
            .field("observed_artifact_set_digest", &"<redacted>")
            .field("mismatch", &self.mismatch)
            .finish()
    }
}

impl fmt::Debug for CandidateArtifactDigestMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateArtifactDigestMismatch")
            .field("ordinal", &self.ordinal)
            .field("expected_digest", &"<redacted>")
            .field("observed_digest", &"<redacted>")
            .field("expected_digest_len", &self.expected_digest.len())
            .field("observed_digest_len", &self.observed_digest.len())
            .finish()
    }
}
