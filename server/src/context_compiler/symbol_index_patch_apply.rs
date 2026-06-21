use std::{fs, path::Path};

use super::{
    symbol_index_patch_apply_git::{
        cleanup_worktree, commit_message, default_branch_name, git_first_line,
        normalize_branch_name, run_git, temp_patch_file, temp_run_dir, DEFAULT_TIMEOUT_SECS,
    },
    symbol_index_patch_apply_policy::{
        base_apply_response, block_apply, dry_run_next_steps, policy_block, policy_next_steps,
        policy_reasons, rollback_info,
    },
    symbol_index_patch_apply_types::{
        PatchApplyCommandReport, PatchApplyMode, PatchApplyOptions, PatchApplyWorkflowStatus,
        SymbolPatchApplyWorkflowResponse,
    },
    symbol_index_patch_generation_types::SymbolPatchGeneration,
    symbol_index_patch_plan_types::SymbolPatchPlan,
    symbol_index_patch_review::build_symbol_patch_review,
    symbol_index_patch_verification_run::run_symbol_patch_verification,
};

pub(crate) use super::symbol_index_patch_apply_rollback::rollback_symbol_patch;

pub(crate) fn apply_reviewed_symbol_patch(
    plan: &SymbolPatchPlan,
    generation: &SymbolPatchGeneration,
    generated_diff: &str,
    workspace: &Path,
    options: PatchApplyOptions,
) -> SymbolPatchApplyWorkflowResponse {
    let verification = run_symbol_patch_verification(generation, generated_diff, workspace);
    let review = build_symbol_patch_review(plan, generation, generated_diff, Some(&verification));
    let patch_sha256 = verification.dry_run.apply_gate.patch_sha256.clone();
    let mut response = base_apply_response(
        generation,
        &verification,
        &review,
        options.clone(),
        patch_sha256,
    );

    let Some(source_git_root) = verification
        .dry_run
        .workspace
        .git_root
        .as_ref()
        .map(std::path::PathBuf::from)
    else {
        return block_apply(response, vec!["workspace_git_root_unavailable".to_string()]);
    };
    response.source_branch =
        git_first_line(&source_git_root, &["rev-parse", "--abbrev-ref", "HEAD"]);

    if !verification.dry_run.apply_gate.ready_to_apply {
        return block_apply(response, verification.dry_run.apply_gate.blockers.clone());
    }
    if let Some(policy_status) = policy_block(&review, &options) {
        response.status = policy_status;
        response.blocked_reasons = policy_reasons(policy_status);
        response.next_steps = policy_next_steps(policy_status);
        return response;
    }
    if options.mode == PatchApplyMode::DryRun || !options.confirm {
        response.status = PatchApplyWorkflowStatus::DryRunReady;
        response.next_steps = dry_run_next_steps(&response);
        return response;
    }

    match options.mode {
        PatchApplyMode::DryRun => response,
        PatchApplyMode::CurrentWorktree => {
            apply_to_existing_worktree(response, generated_diff, &source_git_root, None, options)
        }
        PatchApplyMode::NewBranch | PatchApplyMode::TemporaryWorktree => {
            apply_to_new_worktree(response, generated_diff, &source_git_root, options)
        }
    }
}

fn apply_to_new_worktree(
    mut response: SymbolPatchApplyWorkflowResponse,
    generated_diff: &str,
    source_git_root: &Path,
    options: PatchApplyOptions,
) -> SymbolPatchApplyWorkflowResponse {
    let branch_name = normalize_branch_name(options.branch_name.as_deref())
        .unwrap_or_else(|| default_branch_name(&response.task, &response.patch_sha256));
    let run_workspace = temp_run_dir("apply");
    response.branch_name = Some(branch_name.clone());
    response.run_workspace = Some(run_workspace.display().to_string());

    let setup = run_git(
        source_git_root,
        &[
            "worktree".to_string(),
            "add".to_string(),
            "-b".to_string(),
            branch_name,
            run_workspace.display().to_string(),
            "HEAD".to_string(),
        ],
        DEFAULT_TIMEOUT_SECS,
    );
    response.setup_command = Some(setup.clone());
    if !setup.success {
        response.status = PatchApplyWorkflowStatus::SetupFailed;
        response
            .blocked_reasons
            .push("apply_worktree_create_failed".to_string());
        response.next_steps = vec![
            "Choose a different branch name or remove the stale worktree before retrying."
                .to_string(),
        ];
        return response;
    }

    apply_to_existing_worktree(
        response,
        generated_diff,
        &run_workspace,
        Some(source_git_root),
        options,
    )
}

fn apply_to_existing_worktree(
    mut response: SymbolPatchApplyWorkflowResponse,
    generated_diff: &str,
    target_worktree: &Path,
    cleanup_source_git_root: Option<&Path>,
    options: PatchApplyOptions,
) -> SymbolPatchApplyWorkflowResponse {
    let patch_file = temp_patch_file("apply");
    if let Err(error) = fs::write(&patch_file, generated_diff) {
        response.status = PatchApplyWorkflowStatus::SetupFailed;
        response
            .blocked_reasons
            .push("write_apply_patch_failed".to_string());
        response.warnings.push(error.to_string());
        return response;
    }

    let apply_check = run_git(
        target_worktree,
        &[
            "apply".to_string(),
            "--check".to_string(),
            "--whitespace=nowarn".to_string(),
            patch_file.display().to_string(),
        ],
        DEFAULT_TIMEOUT_SECS,
    );
    if !apply_check.success {
        response.status = PatchApplyWorkflowStatus::ApplyFailed;
        response.apply_command = Some(apply_check);
        response
            .blocked_reasons
            .push("target_git_apply_check_failed".to_string());
        response.next_steps = vec![
            "Inspect target apply-check stderr and regenerate the diff for the target branch."
                .to_string(),
        ];
        cleanup_after_failed_apply(
            &mut response,
            cleanup_source_git_root,
            target_worktree,
            &patch_file,
        );
        return response;
    }

    let apply = run_git(
        target_worktree,
        &[
            "apply".to_string(),
            "--whitespace=nowarn".to_string(),
            patch_file.display().to_string(),
        ],
        DEFAULT_TIMEOUT_SECS,
    );
    response.apply_command = Some(apply.clone());
    if !apply.success {
        response.status = PatchApplyWorkflowStatus::ApplyFailed;
        response
            .blocked_reasons
            .push("target_git_apply_failed".to_string());
        response.next_steps = vec![
            "Use rollback.reversePatchCommand only if the patch was partially applied.".to_string(),
        ];
        cleanup_after_failed_apply(
            &mut response,
            cleanup_source_git_root,
            target_worktree,
            &patch_file,
        );
        return response;
    }

    response.rollback = rollback_info(
        &response,
        target_worktree,
        Some(&patch_file),
        response.commit_sha.as_deref(),
        cleanup_source_git_root.is_some(),
    );

    if options.commit {
        let message = commit_message(&options, &response.task);
        let commit_result =
            commit_applied_patch(target_worktree, &response.touched_files, &message);
        response.commit_message = Some(message);
        response.commit_command = Some(commit_result.clone());
        if !commit_result.success {
            response.status = PatchApplyWorkflowStatus::CommitFailed;
            response
                .blocked_reasons
                .push("git_commit_failed".to_string());
            response.next_steps = vec![
                "Inspect git commit stderr; the patch remains applied in runWorkspace.".to_string(),
                "Use rollback.reversePatchCommand to undo uncommitted changes if needed."
                    .to_string(),
            ];
            return response;
        }
        response.commit_sha = git_first_line(target_worktree, &["rev-parse", "--short", "HEAD"]);
        response.status = PatchApplyWorkflowStatus::CommittedToBranch;
        response.rollback = rollback_info(
            &response,
            target_worktree,
            Some(&patch_file),
            response.commit_sha.as_deref(),
            cleanup_source_git_root.is_some(),
        );
        response.next_steps = vec![
            "Run the same verification commands on the apply branch before merging.".to_string(),
            "Use patch-rollback with commitSha from this response if the branch must be reverted."
                .to_string(),
        ];
    } else {
        response.status = if cleanup_source_git_root.is_some() {
            PatchApplyWorkflowStatus::AppliedToTemporaryWorktree
        } else {
            PatchApplyWorkflowStatus::AppliedToCurrentWorktree
        };
        response.next_steps = vec![
            "Inspect the applied diff and run verification commands before committing.".to_string(),
            "Use rollback.reversePatchCommand to undo this uncommitted apply.".to_string(),
        ];
    }

    if cleanup_source_git_root.is_some() && !options.keep_worktree {
        let cleanup = cleanup_worktree(cleanup_source_git_root.unwrap(), target_worktree);
        response.run_workspace_removed = cleanup.success;
        response.cleanup_command = Some(cleanup.clone());
        if !cleanup.success {
            response
                .warnings
                .push("apply_worktree_cleanup_failed".to_string());
        }
    }
    if response.run_workspace_removed {
        let _ = fs::remove_file(&patch_file);
    }
    response
}

fn commit_applied_patch(
    worktree: &Path,
    touched_files: &[String],
    message: &str,
) -> PatchApplyCommandReport {
    if touched_files.is_empty() {
        return PatchApplyCommandReport::not_attempted(
            "git add --all -- <touched-files>",
            "no touched files to commit",
        );
    }
    let mut add_args = vec!["add".to_string(), "--all".to_string(), "--".to_string()];
    add_args.extend(touched_files.iter().cloned());
    let add = run_git(worktree, &add_args, DEFAULT_TIMEOUT_SECS);
    if !add.success {
        return add;
    }
    run_git(
        worktree,
        &["commit".to_string(), "-m".to_string(), message.to_string()],
        DEFAULT_TIMEOUT_SECS,
    )
}

fn cleanup_after_failed_apply(
    response: &mut SymbolPatchApplyWorkflowResponse,
    cleanup_source_git_root: Option<&Path>,
    target_worktree: &Path,
    patch_file: &Path,
) {
    let _ = fs::remove_file(patch_file);
    if let Some(source_git_root) = cleanup_source_git_root {
        let cleanup = cleanup_worktree(source_git_root, target_worktree);
        response.run_workspace_removed = cleanup.success;
        response.cleanup_command = Some(cleanup);
    }
}
