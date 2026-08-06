use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::node_agent_compute_plugin_host::signed_artifact_verification::jcs_sha256_hex;

const EVIDENCE_SCHEMA: &str = "elon.compute_plugin.candidate_cleanup_execution_evidence.v1";
const HASHED_EVIDENCE_SCHEMA: &str =
    "elon.compute_plugin.hashed_candidate_cleanup_execution_evidence.v1";
const EVIDENCE_CANONICALIZATION: &str = "RFC8785-JCS";
const EVIDENCE_DIGEST_ALGORITHM: &str = "sha256";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupStepEvidence {
    pub sequence: i64,
    pub object_kind: String,
    pub logical_path: String,
    pub content_digest: Option<String>,
    pub file_identity_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidateCleanupExecutionEvidence
{
    pub schema: String,
    pub cleanup_id: String,
    pub cleanup_authorization_receipt_digest: String,
    pub candidate_token_digest: String,
    pub quarantine_receipt_digest: String,
    pub staging_receipt_digest: String,
    pub extraction_evidence_digest: String,
    pub steps: Vec<ComputePluginCandidateCleanupStepEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct HashedComputePluginCandidateCleanupExecutionEvidence
{
    pub schema: String,
    pub evidence: ComputePluginCandidateCleanupExecutionEvidence,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub evidence_digest: String,
}

pub(super) fn build_hashed_execution_evidence(
    cleanup_id: String,
    cleanup_authorization_receipt_digest: String,
    candidate_token_digest: String,
    quarantine_receipt_digest: String,
    staging_receipt_digest: String,
    extraction_evidence_digest: String,
    steps: Vec<ComputePluginCandidateCleanupStepEvidence>,
) -> Result<HashedComputePluginCandidateCleanupExecutionEvidence> {
    let evidence = ComputePluginCandidateCleanupExecutionEvidence {
        schema: EVIDENCE_SCHEMA.to_string(),
        cleanup_id,
        cleanup_authorization_receipt_digest,
        candidate_token_digest,
        quarantine_receipt_digest,
        staging_receipt_digest,
        extraction_evidence_digest,
        steps,
    };
    let evidence_digest = jcs_sha256_hex(&evidence)?;
    Ok(HashedComputePluginCandidateCleanupExecutionEvidence {
        schema: HASHED_EVIDENCE_SCHEMA.to_string(),
        evidence,
        canonicalization: EVIDENCE_CANONICALIZATION.to_string(),
        digest_algorithm: EVIDENCE_DIGEST_ALGORITHM.to_string(),
        evidence_digest,
    })
}
