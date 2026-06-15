use std::path::Path;

use super::{
    symbol_index_patch_apply_git::{normalize_branch_name, quote_path},
    symbol_index_patch_apply_types::{
        PatchApplyMode, PatchApplyOptions, PatchApplyWorkflowStatus, PatchRollbackInfo,
        SymbolPatchApplyWorkflowResponse,
    },
    symbol_index_patch_generation_types::SymbolPatchGeneration,
    symbol_index_patch_review_types::{
        PatchReviewDecision, PatchReviewSeverity, SymbolPatchReviewResponse,
    },
    symbol_index_patch_verification_run_types::SymbolPatchVerificationRunResponse,
};

pub(crate) fn base_apply_response(
    generation: &SymbolPatchGeneration,
    verification: &SymbolPatchVerificationRunResponse,
    review: &SymbolPatchReviewResponse,
    options: PatchApplyOptions,
    patch_sha256: String,
) -> SymbolPatchApplyWorkflowResponse {
    let source = &verification.dry_run.workspace;
    SymbolPatchApplyWorkflowResponse {
        task: generation.task.clone(),
        status: PatchApplyWorkflowStatus::Blocked,
        mode: options.mode,
        confirmed: options.confirm,
        commit_requested: options.commit,
        require_review_approval: options.require_review_approval,
        patch_sha256,
        touched_files: verification.dry_run.apply_gate.touched_files.clone(),
        source_workspace: source.requested_path.clone(),
        source_git_root: source.git_root.clone(),
        source_head: source.head.clone(),
        source_branch: None,
        source_clean: source.clean,
        source_status_lines: source.status_lines.clone(),
        run_workspace: None,
        run_workspace_removed: false,
        branch_name: normalize_branch_name(options.branch_name.as_deref()),
        commit_message: options.commit_message.clone(),
        commit_sha: None,
        review_decision: review.decision,
        highest_finding_severity: highest_finding_severity(review),
        apply_check: verification.dry_run.apply_check.clone(),
        setup_command: None,
        apply_command: None,
        commit_command: None,
        cleanup_command: None,
        rollback: PatchRollbackInfo {
            available: false,
            strategy: generation.apply_readiness.rollback_strategy.clone(),
            reverse_patch_command: None,
            revert_commit_command: None,
            cleanup_worktree_command: None,
            notes: Vec::new(),
        },
        verification: verification.clone(),
        review: review.clone(),
        blocked_reasons: Vec::new(),
        warnings: verification.dry_run.apply_gate.warnings.clone(),
        next_steps: Vec::new(),
    }
}

pub(crate) fn block_apply(
    mut response: SymbolPatchApplyWorkflowResponse,
    reasons: Vec<String>,
) -> SymbolPatchApplyWorkflowResponse {
    response.status = PatchApplyWorkflowStatus::Blocked;
    response.blocked_reasons = super::symbol_index_patch_apply_git::dedupe(reasons);
    response.next_steps =
        vec!["Resolve apply gate blockers, then rerun patch dry-run and review.".to_string()];
    response
}

pub(crate) fn policy_block(
    review: &SymbolPatchReviewResponse,
    options: &PatchApplyOptions,
) -> Option<PatchApplyWorkflowStatus> {
    if !options.require_review_approval {
        return None;
    }
    match review.decision {
        PatchReviewDecision::Reject => Some(PatchApplyWorkflowStatus::RejectedByReview),
        PatchReviewDecision::NeedsHumanReview if !options.confirm => {
            Some(PatchApplyWorkflowStatus::NeedsHumanReview)
        }
        PatchReviewDecision::NeedsHumanReview
            if options.mode == PatchApplyMode::CurrentWorktree =>
        {
            Some(PatchApplyWorkflowStatus::NeedsHumanReview)
        }
        _ => None,
    }
}

pub(crate) fn policy_reasons(status: PatchApplyWorkflowStatus) -> Vec<String> {
    match status {
        PatchApplyWorkflowStatus::RejectedByReview => vec!["review_decision_reject".to_string()],
        PatchApplyWorkflowStatus::NeedsHumanReview => {
            vec!["review_decision_needs_human_review".to_string()]
        }
        _ => Vec::new(),
    }
}

pub(crate) fn policy_next_steps(status: PatchApplyWorkflowStatus) -> Vec<String> {
    match status {
        PatchApplyWorkflowStatus::RejectedByReview => vec![
            "Do not apply this patch; generate a narrower repair patch first.".to_string(),
        ],
        PatchApplyWorkflowStatus::NeedsHumanReview => vec![
            "Inspect the review findings and use an isolated new branch/worktree after human approval."
                .to_string(),
        ],
        _ => Vec::new(),
    }
}

pub(crate) fn dry_run_next_steps(response: &SymbolPatchApplyWorkflowResponse) -> Vec<String> {
    vec![
        format!(
            "Apply check passed for patch sha256={}.",
            response.patch_sha256
        ),
        "Set confirm=true and mode=new_branch or mode=temporary_worktree to apply safely."
            .to_string(),
        "Set commit=true only after review approval and verification are acceptable.".to_string(),
    ]
}

pub(crate) fn rollback_info(
    response: &SymbolPatchApplyWorkflowResponse,
    target_worktree: &Path,
    patch_file: Option<&Path>,
    commit_sha: Option<&str>,
    temporary_worktree: bool,
) -> PatchRollbackInfo {
    let reverse_patch_command = patch_file.map(|path| {
        format!(
            "git -C {} apply -R --whitespace=nowarn {}",
            quote_path(target_worktree),
            quote_path(path)
        )
    });
    let revert_commit_command = commit_sha.map(|sha| {
        format!(
            "git -C {} revert --no-edit {}",
            quote_path(target_worktree),
            sha
        )
    });
    let cleanup_worktree_command = temporary_worktree.then(|| {
        format!(
            "git -C {} worktree remove --force {}",
            quote_path(Path::new(
                response.source_git_root.as_deref().unwrap_or_default()
            )),
            quote_path(target_worktree)
        )
    });
    let mut notes = Vec::new();
    if commit_sha.is_some() {
        notes.push(
            "Committed patches should be rolled back with git revert, not reset --hard."
                .to_string(),
        );
    } else {
        notes.push(
            "Uncommitted apply can be reversed with git apply -R using the same patch.".to_string(),
        );
    }

    PatchRollbackInfo {
        available: reverse_patch_command.is_some() || revert_commit_command.is_some(),
        strategy: response.rollback.strategy.clone(),
        reverse_patch_command,
        revert_commit_command,
        cleanup_worktree_command,
        notes,
    }
}

fn highest_finding_severity(review: &SymbolPatchReviewResponse) -> Option<PatchReviewSeverity> {
    review.findings.iter().map(|finding| finding.severity).max()
}
