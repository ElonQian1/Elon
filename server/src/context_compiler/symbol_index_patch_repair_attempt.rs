use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{
    symbol_index_patch_check::{check_symbol_patch_diff, SymbolPatchDiffCheck},
    symbol_index_patch_generation_types::SymbolPatchGeneration,
    symbol_index_patch_verification_repair::PatchVerificationRepairStatus,
    symbol_index_patch_verification_run::run_symbol_patch_verification,
    symbol_index_patch_verification_run_types::{
        PatchVerificationExecutionStatus, SymbolPatchVerificationRunResponse,
    },
};

const MAX_REPAIR_ATTEMPTS: usize = 2;
const DIFF_EXCERPT_LIMIT: usize = 4_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolPatchRepairAttemptResponse {
    pub(crate) task: String,
    pub(crate) status: PatchRepairAttemptStatus,
    pub(crate) attempt: usize,
    pub(crate) max_attempts: usize,
    pub(crate) original_patch_sha256: String,
    pub(crate) repair_patch_sha256: String,
    pub(crate) combined_patch_sha256: Option<String>,
    pub(crate) original_verification: SymbolPatchVerificationRunResponse,
    pub(crate) repair_diff_check: SymbolPatchDiffCheck,
    pub(crate) repaired_verification: Option<SymbolPatchVerificationRunResponse>,
    pub(crate) repair_request: PatchRepairRequestSummary,
    pub(crate) repair_result: PatchRepairResultSummary,
    pub(crate) next_steps: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchRepairAttemptStatus {
    RepairedPatchPassed,
    ManualVerificationRequired,
    RepairStillFailing,
    AttemptsExhausted,
    AttemptRejected,
    OriginalPatchNotReadyForRepair,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchRepairRequestSummary {
    pub(crate) task: String,
    pub(crate) attempt: usize,
    pub(crate) max_attempts: usize,
    pub(crate) original_patch_sha256: String,
    pub(crate) verification_status: PatchVerificationExecutionStatus,
    pub(crate) failure_kind: Option<String>,
    pub(crate) failed_command: Option<String>,
    pub(crate) repair_context_status: PatchVerificationRepairStatus,
    pub(crate) repair_context_excerpt: Option<String>,
    pub(crate) allowed_files: Vec<String>,
    pub(crate) forbidden_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchRepairResultSummary {
    pub(crate) repair_patch_sha256: String,
    pub(crate) repair_patch_excerpt: String,
    pub(crate) combined_patch_sha256: Option<String>,
    pub(crate) combined_patch_excerpt: Option<String>,
    pub(crate) verification_status: Option<PatchVerificationExecutionStatus>,
    pub(crate) success: bool,
}

pub(crate) fn build_symbol_patch_repair_attempt_response(
    generation: &SymbolPatchGeneration,
    original_patch: &str,
    repair_patch: &str,
    workspace: &Path,
    attempt: Option<usize>,
    max_attempts: Option<usize>,
) -> SymbolPatchRepairAttemptResponse {
    let attempt = attempt.unwrap_or(1).max(1);
    let max_attempts = max_attempts
        .unwrap_or(MAX_REPAIR_ATTEMPTS)
        .clamp(1, MAX_REPAIR_ATTEMPTS);
    let original_patch_sha256 = sha256_hex(original_patch);
    let repair_patch_sha256 = sha256_hex(repair_patch);
    let original_verification =
        run_symbol_patch_verification(generation, original_patch, workspace);
    let repair_diff_check = check_symbol_patch_diff(generation, repair_patch);
    let repair_request = repair_request_summary(
        generation,
        &original_patch_sha256,
        attempt,
        max_attempts,
        &original_verification,
    );

    let blocked = preflight_rejection(
        attempt,
        max_attempts,
        &original_verification,
        &repair_diff_check,
    );
    if let Some((status, next_steps)) = blocked {
        return response_without_repaired_run(
            generation,
            status,
            attempt,
            max_attempts,
            original_patch_sha256,
            repair_patch_sha256,
            original_verification,
            repair_diff_check,
            repair_request,
            repair_patch,
            next_steps,
        );
    }

    let combined_patch = combine_patch_streams(original_patch, repair_patch);
    let combined_patch_sha256 = sha256_hex(&combined_patch);
    let repaired_verification =
        run_symbol_patch_verification(generation, &combined_patch, workspace);
    let repaired_execution_status = repaired_verification.execution.status;
    let (status, next_steps) =
        repaired_status_and_next_steps(attempt, max_attempts, &repaired_verification);
    let success = status == PatchRepairAttemptStatus::RepairedPatchPassed;

    SymbolPatchRepairAttemptResponse {
        task: generation.task.clone(),
        status,
        attempt,
        max_attempts,
        original_patch_sha256,
        repair_patch_sha256: repair_patch_sha256.clone(),
        combined_patch_sha256: Some(combined_patch_sha256.clone()),
        original_verification,
        repair_diff_check,
        repaired_verification: Some(repaired_verification),
        repair_request,
        repair_result: PatchRepairResultSummary {
            repair_patch_sha256,
            repair_patch_excerpt: truncate_text(repair_patch, DIFF_EXCERPT_LIMIT),
            combined_patch_sha256: Some(combined_patch_sha256),
            combined_patch_excerpt: Some(truncate_text(&combined_patch, DIFF_EXCERPT_LIMIT)),
            verification_status: Some(repaired_execution_status),
            success,
        },
        next_steps,
    }
}

fn response_without_repaired_run(
    generation: &SymbolPatchGeneration,
    status: PatchRepairAttemptStatus,
    attempt: usize,
    max_attempts: usize,
    original_patch_sha256: String,
    repair_patch_sha256: String,
    original_verification: SymbolPatchVerificationRunResponse,
    repair_diff_check: SymbolPatchDiffCheck,
    repair_request: PatchRepairRequestSummary,
    repair_patch: &str,
    next_steps: Vec<String>,
) -> SymbolPatchRepairAttemptResponse {
    SymbolPatchRepairAttemptResponse {
        task: generation.task.clone(),
        status,
        attempt,
        max_attempts,
        original_patch_sha256,
        repair_patch_sha256: repair_patch_sha256.clone(),
        combined_patch_sha256: None,
        original_verification,
        repair_diff_check,
        repaired_verification: None,
        repair_request,
        repair_result: PatchRepairResultSummary {
            repair_patch_sha256,
            repair_patch_excerpt: truncate_text(repair_patch, DIFF_EXCERPT_LIMIT),
            combined_patch_sha256: None,
            combined_patch_excerpt: None,
            verification_status: None,
            success: false,
        },
        next_steps,
    }
}

fn preflight_rejection(
    attempt: usize,
    max_attempts: usize,
    original_verification: &SymbolPatchVerificationRunResponse,
    repair_diff_check: &SymbolPatchDiffCheck,
) -> Option<(PatchRepairAttemptStatus, Vec<String>)> {
    if attempt > max_attempts {
        return Some((
            PatchRepairAttemptStatus::AttemptsExhausted,
            vec![
                "Repair attempt exceeds max_attempts; stop the automatic repair loop.".to_string(),
                "Return the final verification report to an operator.".to_string(),
            ],
        ));
    }

    if !original_verification
        .verification_repair_context
        .model_repair_required
    {
        return Some((
            PatchRepairAttemptStatus::OriginalPatchNotReadyForRepair,
            vec![
                "Original patch verification did not request model repair.".to_string(),
                "Use patch-verify-run output directly before attempting a repair patch."
                    .to_string(),
            ],
        ));
    }

    if !repair_diff_check.accepted_for_apply_check {
        return Some((
            PatchRepairAttemptStatus::AttemptRejected,
            vec![
                "Repair diff violates the patch contract or touches no allowed files.".to_string(),
                "Regenerate an incremental unified diff that only edits allowed_files.".to_string(),
            ],
        ));
    }

    None
}

fn repaired_status_and_next_steps(
    attempt: usize,
    max_attempts: usize,
    repaired_verification: &SymbolPatchVerificationRunResponse,
) -> (PatchRepairAttemptStatus, Vec<String>) {
    match repaired_verification.execution.status {
        PatchVerificationExecutionStatus::Passed => (
            PatchRepairAttemptStatus::RepairedPatchPassed,
            vec![
                "Original patch plus repair patch passed isolated verification.".to_string(),
                "The source workspace was not modified by the repair attempt.".to_string(),
            ],
        ),
        PatchVerificationExecutionStatus::ManualVerificationRequired => (
            PatchRepairAttemptStatus::ManualVerificationRequired,
            vec![
                "Auto-runnable verification passed, but required manual checks remain."
                    .to_string(),
                "Run skipped manual commands before applying the repaired patch.".to_string(),
            ],
        ),
        _ if attempt >= max_attempts => (
            PatchRepairAttemptStatus::AttemptsExhausted,
            vec![
                "Repair verification still failed and max_attempts has been reached."
                    .to_string(),
                "Return repaired_verification and repair_request to an operator.".to_string(),
            ],
        ),
        _ => (
            PatchRepairAttemptStatus::RepairStillFailing,
            vec![
                "Repair verification still failed.".to_string(),
                "Use repaired_verification.verificationRepairContext.retryPrompt for the next repair attempt.".to_string(),
            ],
        ),
    }
}

fn repair_request_summary(
    generation: &SymbolPatchGeneration,
    original_patch_sha256: &str,
    attempt: usize,
    max_attempts: usize,
    original_verification: &SymbolPatchVerificationRunResponse,
) -> PatchRepairRequestSummary {
    let first_failure = original_verification
        .verification_repair_context
        .failed_commands
        .first();
    PatchRepairRequestSummary {
        task: generation.task.clone(),
        attempt,
        max_attempts,
        original_patch_sha256: original_patch_sha256.to_string(),
        verification_status: original_verification.execution.status,
        failure_kind: first_failure.map(|failure| failure.failure_kind.clone()),
        failed_command: first_failure.map(|failure| failure.command.clone()),
        repair_context_status: original_verification.verification_repair_context.status,
        repair_context_excerpt: original_verification
            .verification_repair_context
            .retry_prompt
            .as_deref()
            .map(|text| truncate_text(text, DIFF_EXCERPT_LIMIT)),
        allowed_files: generation.diff_contract.allowed_files.clone(),
        forbidden_patterns: generation.diff_contract.forbidden_patterns.clone(),
    }
}

fn combine_patch_streams(original_patch: &str, repair_patch: &str) -> String {
    let mut combined = original_patch.trim_end().to_string();
    combined.push_str("\n\n");
    combined.push_str(repair_patch.trim_start());
    if !combined.ends_with('\n') {
        combined.push('\n');
    }
    combined
}

fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn truncate_text(text: &str, limit: usize) -> String {
    let mut out = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= limit {
            out.push_str("...<truncated>");
            return out;
        }
        out.push(ch);
    }
    out
}
