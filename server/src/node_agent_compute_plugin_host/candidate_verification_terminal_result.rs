use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::{
    install_plan_admission_validation::is_identifier, manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

const ABORT_RESULT_SCHEMA: &str = "elon.compute_plugin.candidate_verification_abort.v1";
const REVOCATION_RESULT_SCHEMA: &str = "elon.compute_plugin.candidate_verification_revocation.v1";
const VERIFIED_RESULT_SCHEMA: &str = "elon.compute_plugin.candidate_verification_verified.v1";
const REJECTED_RESULT_SCHEMA: &str = "elon.compute_plugin.candidate_verification_rejected.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateVerificationResolutionKind {
    Verified,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct CandidateVerificationDigestMismatch {
    pub ordinal: i64,
    pub expected_digest: String,
    pub observed_digest: String,
}

/// Primitive-only input for the resolution Store. It deliberately carries no filesystem handle or
/// private hash capability, but binds the immutable run, complete hash evidence and exact
/// inventory/authority transition written by the same transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct CandidateVerificationResolutionInput {
    pub kind: CandidateVerificationResolutionKind,
    pub verification_id: String,
    pub candidate_token_digest: String,
    pub owner_plan_id: String,
    pub owner_plan_digest: String,
    pub verification_generation: i64,
    pub candidate_generation: i64,
    pub prepared_at_ms: i64,
    pub resolved_at_ms: i64,
    pub artifact_count: i64,
    pub artifact_bytes: i64,
    pub expected_artifact_set_digest: String,
    pub observed_artifact_set_digest: String,
    pub file_set_binding_digest: String,
    pub mismatch: Option<CandidateVerificationDigestMismatch>,
    pub authority_state_revision_before: i64,
    pub authority_state_revision_after: i64,
    pub inventory_revision_before: i64,
    pub inventory_revision_after: i64,
    pub inventory_digest_before: String,
    pub inventory_digest_after: String,
    pub authority_epoch_before: i64,
    pub authority_epoch_after: i64,
    pub slot_phase_after: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CandidateVerificationResolutionPayload {
    schema: String,
    state: String,
    reason: String,
    verification_id: String,
    candidate_token_digest: String,
    owner_plan_id: String,
    owner_plan_digest: String,
    verification_generation: i64,
    candidate_generation: i64,
    prepared_at_ms: i64,
    resolved_at_ms: i64,
    artifact_count: i64,
    artifact_bytes: i64,
    expected_artifact_set_digest: String,
    observed_artifact_set_digest: String,
    file_set_binding_digest: String,
    mismatch: Option<CandidateVerificationDigestMismatch>,
    authority_state_revision_before: i64,
    authority_state_revision_after: i64,
    inventory_revision_before: i64,
    inventory_revision_after: i64,
    inventory_digest_before: String,
    inventory_digest_after: String,
    authority_epoch_before: i64,
    authority_epoch_after: i64,
    slot_phase_after: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateVerificationTerminalKind {
    Aborted,
    Revoked,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CandidateVerificationTerminalResult {
    schema: String,
    state: String,
    reason: String,
    resolved_at_ms: i64,
}

impl CandidateVerificationResolutionKind {
    pub(in crate::node_agent_compute_plugin_host) fn state(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Rejected => "rejected",
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn reason(self) -> &'static str {
        match self {
            Self::Verified => "artifact_set_verified",
            Self::Rejected => "artifact_digest_mismatch",
        }
    }

    fn schema(self) -> &'static str {
        match self {
            Self::Verified => VERIFIED_RESULT_SCHEMA,
            Self::Rejected => REJECTED_RESULT_SCHEMA,
        }
    }
}

impl CandidateVerificationResolutionInput {
    pub(in crate::node_agent_compute_plugin_host) fn state(&self) -> &'static str {
        self.kind.state()
    }

    pub(in crate::node_agent_compute_plugin_host) fn reason(&self) -> &'static str {
        self.kind.reason()
    }

    pub(in crate::node_agent_compute_plugin_host) fn mismatch_ordinal(&self) -> Option<i64> {
        self.mismatch.as_ref().map(|mismatch| mismatch.ordinal)
    }

    pub(in crate::node_agent_compute_plugin_host) fn mismatch_observed_digest(
        &self,
    ) -> Option<&str> {
        self.mismatch
            .as_ref()
            .map(|mismatch| mismatch.observed_digest.as_str())
    }
}

pub(in crate::node_agent_compute_plugin_host) fn encode_candidate_verification_resolution(
    input: &CandidateVerificationResolutionInput,
) -> Result<(String, String)> {
    validate_resolution_input(input)?;
    let payload = resolution_payload(input);
    Ok((serde_json::to_string(&payload)?, jcs_sha256_hex(&payload)?))
}

pub(in crate::node_agent_compute_plugin_host) fn parse_candidate_verification_resolution(
    stored_json: &str,
    stored_digest: &str,
) -> Result<CandidateVerificationResolutionInput> {
    if !is_sha256(stored_digest) {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_RESULT_DIGEST_INVALID");
    }
    let payload: CandidateVerificationResolutionPayload = serde_json::from_str(stored_json)
        .map_err(|_| {
            anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_RESULT_JSON_INVALID")
        })?;
    let kind = parse_resolution_kind(&payload)?;
    let input = CandidateVerificationResolutionInput {
        kind,
        verification_id: payload.verification_id.clone(),
        candidate_token_digest: payload.candidate_token_digest.clone(),
        owner_plan_id: payload.owner_plan_id.clone(),
        owner_plan_digest: payload.owner_plan_digest.clone(),
        verification_generation: payload.verification_generation,
        candidate_generation: payload.candidate_generation,
        prepared_at_ms: payload.prepared_at_ms,
        resolved_at_ms: payload.resolved_at_ms,
        artifact_count: payload.artifact_count,
        artifact_bytes: payload.artifact_bytes,
        expected_artifact_set_digest: payload.expected_artifact_set_digest.clone(),
        observed_artifact_set_digest: payload.observed_artifact_set_digest.clone(),
        file_set_binding_digest: payload.file_set_binding_digest.clone(),
        mismatch: payload.mismatch.clone(),
        authority_state_revision_before: payload.authority_state_revision_before,
        authority_state_revision_after: payload.authority_state_revision_after,
        inventory_revision_before: payload.inventory_revision_before,
        inventory_revision_after: payload.inventory_revision_after,
        inventory_digest_before: payload.inventory_digest_before.clone(),
        inventory_digest_after: payload.inventory_digest_after.clone(),
        authority_epoch_before: payload.authority_epoch_before,
        authority_epoch_after: payload.authority_epoch_after,
        slot_phase_after: payload.slot_phase_after.clone(),
    };
    validate_resolution_input(&input)?;
    if resolution_payload(&input) != payload
        || serde_json::to_string(&payload)? != stored_json
        || jcs_sha256_hex(&payload)? != stored_digest
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_RESULT_BINDING_CHANGED");
    }
    Ok(input)
}

pub(in crate::node_agent_compute_plugin_host) fn validate_candidate_verification_resolution(
    expected: &CandidateVerificationResolutionInput,
    stored_json: &str,
    stored_digest: &str,
) -> Result<()> {
    if parse_candidate_verification_resolution(stored_json, stored_digest)? != *expected {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_RESULT_CHANGED");
    }
    Ok(())
}

fn resolution_payload(
    input: &CandidateVerificationResolutionInput,
) -> CandidateVerificationResolutionPayload {
    CandidateVerificationResolutionPayload {
        schema: input.kind.schema().to_string(),
        state: input.kind.state().to_string(),
        reason: input.kind.reason().to_string(),
        verification_id: input.verification_id.clone(),
        candidate_token_digest: input.candidate_token_digest.clone(),
        owner_plan_id: input.owner_plan_id.clone(),
        owner_plan_digest: input.owner_plan_digest.clone(),
        verification_generation: input.verification_generation,
        candidate_generation: input.candidate_generation,
        prepared_at_ms: input.prepared_at_ms,
        resolved_at_ms: input.resolved_at_ms,
        artifact_count: input.artifact_count,
        artifact_bytes: input.artifact_bytes,
        expected_artifact_set_digest: input.expected_artifact_set_digest.clone(),
        observed_artifact_set_digest: input.observed_artifact_set_digest.clone(),
        file_set_binding_digest: input.file_set_binding_digest.clone(),
        mismatch: input.mismatch.clone(),
        authority_state_revision_before: input.authority_state_revision_before,
        authority_state_revision_after: input.authority_state_revision_after,
        inventory_revision_before: input.inventory_revision_before,
        inventory_revision_after: input.inventory_revision_after,
        inventory_digest_before: input.inventory_digest_before.clone(),
        inventory_digest_after: input.inventory_digest_after.clone(),
        authority_epoch_before: input.authority_epoch_before,
        authority_epoch_after: input.authority_epoch_after,
        slot_phase_after: input.slot_phase_after.clone(),
    }
}

fn parse_resolution_kind(
    payload: &CandidateVerificationResolutionPayload,
) -> Result<CandidateVerificationResolutionKind> {
    match (
        payload.schema.as_str(),
        payload.state.as_str(),
        payload.reason.as_str(),
    ) {
        (VERIFIED_RESULT_SCHEMA, "verified", "artifact_set_verified") => {
            Ok(CandidateVerificationResolutionKind::Verified)
        }
        (REJECTED_RESULT_SCHEMA, "rejected", "artifact_digest_mismatch") => {
            Ok(CandidateVerificationResolutionKind::Rejected)
        }
        _ => bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_RESULT_KIND_INVALID"),
    }
}

fn validate_resolution_input(input: &CandidateVerificationResolutionInput) -> Result<()> {
    let expected_state_after = input
        .authority_state_revision_before
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_FENCE_EXHAUSTED"))?;
    let expected_inventory_after = input
        .inventory_revision_before
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_FENCE_EXHAUSTED"))?;
    let expected_epoch_after = input
        .authority_epoch_before
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_FENCE_EXHAUSTED"))?;
    if !is_identifier(&input.verification_id)
        || !is_sha256(&input.candidate_token_digest)
        || !is_identifier(&input.owner_plan_id)
        || !is_sha256(&input.owner_plan_digest)
        || input.verification_generation <= 0
        || input.candidate_generation <= 0
        || input.prepared_at_ms < 0
        || input.resolved_at_ms <= input.prepared_at_ms
        || input.artifact_count <= 0
        || input.artifact_count > 4_096
        || input.artifact_bytes <= 0
        || !is_sha256(&input.expected_artifact_set_digest)
        || !is_sha256(&input.observed_artifact_set_digest)
        || !is_sha256(&input.file_set_binding_digest)
        || input.authority_state_revision_before <= 0
        || input.authority_state_revision_after != expected_state_after
        || input.inventory_revision_before <= 0
        || input.inventory_revision_after != expected_inventory_after
        || !is_sha256(&input.inventory_digest_before)
        || !is_sha256(&input.inventory_digest_after)
        || input.inventory_digest_before == input.inventory_digest_after
        || input.authority_epoch_before <= 0
        || input.authority_epoch_after != expected_epoch_after
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_RESULT_INVALID");
    }
    match (input.kind, input.mismatch.as_ref()) {
        (CandidateVerificationResolutionKind::Verified, None)
            if input.slot_phase_after == "verifying" =>
        {
            Ok(())
        }
        (CandidateVerificationResolutionKind::Rejected, Some(mismatch))
            if input.slot_phase_after == "failed"
                && mismatch.ordinal >= 0
                && is_sha256(&mismatch.expected_digest)
                && is_sha256(&mismatch.observed_digest)
                && mismatch.expected_digest != mismatch.observed_digest =>
        {
            Ok(())
        }
        _ => bail!("COMPUTE_PLUGIN_VERIFICATION_RESOLUTION_DISPOSITION_INVALID"),
    }
}

pub(in crate::node_agent_compute_plugin_host) fn encode_candidate_verification_abort(
    reason: &str,
    resolved_at_ms: i64,
) -> Result<(String, String)> {
    encode_terminal_result(
        CandidateVerificationTerminalKind::Aborted,
        reason,
        resolved_at_ms,
    )
}

pub(in crate::node_agent_compute_plugin_host) fn encode_candidate_verification_revocation(
    reason: &str,
    resolved_at_ms: i64,
) -> Result<(String, String)> {
    encode_terminal_result(
        CandidateVerificationTerminalKind::Revoked,
        reason,
        resolved_at_ms,
    )
}

pub(in crate::node_agent_compute_plugin_host) fn validate_candidate_verification_terminal_result(
    expected_kind: CandidateVerificationTerminalKind,
    expected_reason: &str,
    expected_resolved_at_ms: i64,
    stored_json: &str,
    stored_digest: &str,
) -> Result<()> {
    if !is_sha256(stored_digest) {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RESULT_DIGEST_INVALID");
    }
    let parsed: CandidateVerificationTerminalResult = serde_json::from_str(stored_json)
        .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RESULT_JSON_INVALID"))?;
    validate_payload(
        &parsed,
        expected_kind,
        expected_reason,
        expected_resolved_at_ms,
    )?;
    if serde_json::to_string(&parsed)? != stored_json || jcs_sha256_hex(&parsed)? != stored_digest {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RESULT_BINDING_CHANGED");
    }
    Ok(())
}

fn encode_terminal_result(
    kind: CandidateVerificationTerminalKind,
    reason: &str,
    resolved_at_ms: i64,
) -> Result<(String, String)> {
    let payload = CandidateVerificationTerminalResult {
        schema: schema(kind).to_string(),
        state: state(kind).to_string(),
        reason: reason.to_string(),
        resolved_at_ms,
    };
    validate_payload(&payload, kind, reason, resolved_at_ms)?;
    Ok((serde_json::to_string(&payload)?, jcs_sha256_hex(&payload)?))
}

fn validate_payload(
    payload: &CandidateVerificationTerminalResult,
    kind: CandidateVerificationTerminalKind,
    reason: &str,
    resolved_at_ms: i64,
) -> Result<()> {
    if resolved_at_ms < 0
        || payload.schema != schema(kind)
        || payload.state != state(kind)
        || payload.reason != reason
        || payload.resolved_at_ms != resolved_at_ms
        || !reason_is_valid(kind, reason)
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_TERMINAL_RESULT_INVALID");
    }
    Ok(())
}

fn schema(kind: CandidateVerificationTerminalKind) -> &'static str {
    match kind {
        CandidateVerificationTerminalKind::Aborted => ABORT_RESULT_SCHEMA,
        CandidateVerificationTerminalKind::Revoked => REVOCATION_RESULT_SCHEMA,
    }
}

fn state(kind: CandidateVerificationTerminalKind) -> &'static str {
    match kind {
        CandidateVerificationTerminalKind::Aborted => "aborted",
        CandidateVerificationTerminalKind::Revoked => "revoked",
    }
}

fn reason_is_valid(kind: CandidateVerificationTerminalKind, reason: &str) -> bool {
    match kind {
        CandidateVerificationTerminalKind::Aborted => {
            matches!(reason, "verification_aborted" | "authority_recovery")
        }
        CandidateVerificationTerminalKind::Revoked => matches!(
            reason,
            "authority_epoch_advanced_by_keyring"
                | "authority_epoch_advanced_by_plan"
                | "authority_epoch_advanced_by_verification"
                | "process_owner_epoch_advanced"
                | "candidate_released_by_plan"
        ),
    }
}
