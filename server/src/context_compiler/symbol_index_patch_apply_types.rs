use serde::Serialize;

use super::{
    symbol_index_patch_dry_run::PatchApplyCheckResult,
    symbol_index_patch_review_types::{
        PatchReviewDecision, PatchReviewSeverity, SymbolPatchReviewResponse,
    },
    symbol_index_patch_verification_run_types::SymbolPatchVerificationRunResponse,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchApplyMode {
    DryRun,
    CurrentWorktree,
    NewBranch,
    TemporaryWorktree,
}

impl PatchApplyMode {
    pub(crate) fn parse(value: Option<&str>) -> Result<Self, String> {
        let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(Self::DryRun);
        };
        match normalize_mode(value).as_str() {
            "dry_run" | "dryrun" | "check" => Ok(Self::DryRun),
            "current_worktree" | "current" | "current_branch" => Ok(Self::CurrentWorktree),
            "new_branch" | "branch" => Ok(Self::NewBranch),
            "temporary_worktree" | "temp_worktree" | "worktree" | "isolated_worktree" => {
                Ok(Self::TemporaryWorktree)
            }
            _ => Err(format!("unsupported apply mode: {value}")),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PatchApplyOptions {
    pub(crate) mode: PatchApplyMode,
    pub(crate) confirm: bool,
    pub(crate) commit: bool,
    pub(crate) keep_worktree: bool,
    pub(crate) branch_name: Option<String>,
    pub(crate) commit_message: Option<String>,
    pub(crate) require_review_approval: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchApplyWorkflowStatus {
    Blocked,
    DryRunReady,
    RejectedByReview,
    NeedsHumanReview,
    SetupFailed,
    ApplyFailed,
    AppliedToCurrentWorktree,
    AppliedToTemporaryWorktree,
    CommitFailed,
    CommittedToBranch,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolPatchApplyWorkflowResponse {
    pub(crate) task: String,
    pub(crate) status: PatchApplyWorkflowStatus,
    pub(crate) mode: PatchApplyMode,
    pub(crate) confirmed: bool,
    pub(crate) commit_requested: bool,
    pub(crate) require_review_approval: bool,
    pub(crate) patch_sha256: String,
    pub(crate) touched_files: Vec<String>,
    pub(crate) source_workspace: String,
    pub(crate) source_git_root: Option<String>,
    pub(crate) source_head: Option<String>,
    pub(crate) source_branch: Option<String>,
    pub(crate) source_clean: bool,
    pub(crate) source_status_lines: Vec<String>,
    pub(crate) run_workspace: Option<String>,
    pub(crate) run_workspace_removed: bool,
    pub(crate) branch_name: Option<String>,
    pub(crate) commit_message: Option<String>,
    pub(crate) commit_sha: Option<String>,
    pub(crate) review_decision: PatchReviewDecision,
    pub(crate) highest_finding_severity: Option<PatchReviewSeverity>,
    pub(crate) apply_check: PatchApplyCheckResult,
    pub(crate) setup_command: Option<PatchApplyCommandReport>,
    pub(crate) apply_command: Option<PatchApplyCommandReport>,
    pub(crate) commit_command: Option<PatchApplyCommandReport>,
    pub(crate) cleanup_command: Option<PatchApplyCommandReport>,
    pub(crate) rollback: PatchRollbackInfo,
    pub(crate) verification: SymbolPatchVerificationRunResponse,
    pub(crate) review: SymbolPatchReviewResponse,
    pub(crate) blocked_reasons: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchRollbackInfo {
    pub(crate) available: bool,
    pub(crate) strategy: String,
    pub(crate) reverse_patch_command: Option<String>,
    pub(crate) revert_commit_command: Option<String>,
    pub(crate) cleanup_worktree_command: Option<String>,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchApplyCommandReport {
    pub(crate) attempted: bool,
    pub(crate) command: String,
    pub(crate) success: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) elapsed_ms: u64,
    pub(crate) timed_out: bool,
    pub(crate) error: Option<String>,
}

impl PatchApplyCommandReport {
    pub(crate) fn not_attempted(command: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            attempted: false,
            command: command.into(),
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            elapsed_ms: 0,
            timed_out: false,
            error: Some(error.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolPatchRollbackResponse {
    pub(crate) status: PatchRollbackStatus,
    pub(crate) confirmed: bool,
    pub(crate) source_workspace: String,
    pub(crate) source_git_root: Option<String>,
    pub(crate) source_head: Option<String>,
    pub(crate) source_branch: Option<String>,
    pub(crate) source_clean: bool,
    pub(crate) patch_sha256: Option<String>,
    pub(crate) commit_sha: Option<String>,
    pub(crate) reverse_check: Option<PatchApplyCommandReport>,
    pub(crate) rollback_command: Option<PatchApplyCommandReport>,
    pub(crate) revert_commit_sha: Option<String>,
    pub(crate) blocked_reasons: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) next_steps: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchRollbackStatus {
    Blocked,
    DryRunReady,
    ReversePatchApplied,
    RevertCommitted,
    Failed,
}

fn normalize_mode(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| match ch {
            '-' | ' ' => '_',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}
