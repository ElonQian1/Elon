use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::{manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex};

const ABORT_RESULT_SCHEMA: &str = "elon.compute_plugin.candidate_verification_abort.v1";
const REVOCATION_RESULT_SCHEMA: &str = "elon.compute_plugin.candidate_verification_revocation.v1";

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
                | "process_owner_epoch_advanced"
                | "candidate_released_by_plan"
        ),
    }
}
