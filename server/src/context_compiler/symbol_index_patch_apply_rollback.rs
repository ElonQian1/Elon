use std::{fs, path::Path, path::PathBuf};

use super::{
    symbol_index_patch_apply_git::{
        DEFAULT_TIMEOUT_SECS, dedupe, git_first_line, inspect_git_source, run_git, sha256_hex,
        temp_patch_file,
    },
    symbol_index_patch_apply_types::{PatchRollbackStatus, SymbolPatchRollbackResponse},
};

pub(crate) fn rollback_symbol_patch(
    workspace: &Path,
    generated_diff: Option<&str>,
    commit_sha: Option<&str>,
    confirm: bool,
) -> SymbolPatchRollbackResponse {
    let source = inspect_git_source(workspace);
    let mut response = SymbolPatchRollbackResponse {
        status: PatchRollbackStatus::Blocked,
        confirmed: confirm,
        source_workspace: workspace.display().to_string(),
        source_git_root: source
            .git_root
            .as_ref()
            .map(|path| path.display().to_string()),
        source_head: source.head,
        source_branch: source.branch,
        source_clean: source.clean,
        patch_sha256: generated_diff.map(sha256_hex),
        commit_sha: commit_sha.map(ToOwned::to_owned),
        reverse_check: None,
        rollback_command: None,
        revert_commit_sha: None,
        blocked_reasons: source.blockers,
        warnings: source.warnings,
        next_steps: Vec::new(),
    };

    if response.source_git_root.is_none() {
        response
            .blocked_reasons
            .push("workspace_git_root_unavailable".to_string());
    }
    if generated_diff.is_none()
        && commit_sha
            .filter(|value| !value.trim().is_empty())
            .is_none()
    {
        response
            .blocked_reasons
            .push("rollback_requires_patch_or_commit_sha".to_string());
    }
    response.blocked_reasons = dedupe(response.blocked_reasons);
    let Some(source_git_root) = response.source_git_root.as_ref().map(PathBuf::from) else {
        response.next_steps = vec!["Provide a valid git workspace before rollback.".to_string()];
        return response;
    };
    if !response.blocked_reasons.is_empty() {
        response.next_steps = vec!["Provide a valid git workspace before rollback.".to_string()];
        return response;
    }

    if let Some(commit_sha) = commit_sha.filter(|value| !value.trim().is_empty()) {
        return rollback_commit(response, &source_git_root, commit_sha, confirm);
    }

    let Some(generated_diff) = generated_diff else {
        response.next_steps = vec!["Provide a patch diff or commitSha for rollback.".to_string()];
        return response;
    };
    rollback_reverse_patch(response, &source_git_root, generated_diff, confirm)
}

fn rollback_commit(
    mut response: SymbolPatchRollbackResponse,
    source_git_root: &Path,
    commit_sha: &str,
    confirm: bool,
) -> SymbolPatchRollbackResponse {
    if !response.source_clean {
        response.status = PatchRollbackStatus::Blocked;
        response
            .blocked_reasons
            .push("workspace_not_clean_for_commit_revert".to_string());
        response.next_steps = vec![
            "Commit rollback requires a clean worktree; use reverse patch rollback for uncommitted apply."
                .to_string(),
        ];
        return response;
    }
    if !confirm {
        response.status = PatchRollbackStatus::DryRunReady;
        response.next_steps = vec![
            "Set confirm=true to run git revert --no-edit for this commit.".to_string(),
            "Run rollback from the branch/worktree that contains the commit.".to_string(),
        ];
        return response;
    }
    let revert = run_git(
        source_git_root,
        &[
            "revert".to_string(),
            "--no-edit".to_string(),
            commit_sha.to_string(),
        ],
        DEFAULT_TIMEOUT_SECS,
    );
    response.rollback_command = Some(revert.clone());
    if revert.success {
        response.status = PatchRollbackStatus::RevertCommitted;
        response.revert_commit_sha =
            git_first_line(source_git_root, &["rev-parse", "--short", "HEAD"]);
        response.next_steps =
            vec!["Run project verification before pushing the rollback commit.".to_string()];
    } else {
        response.status = PatchRollbackStatus::Failed;
        response
            .blocked_reasons
            .push("git_revert_failed".to_string());
        response.next_steps =
            vec!["Inspect git revert stderr and resolve conflicts manually if needed.".to_string()];
    }
    response
}

fn rollback_reverse_patch(
    mut response: SymbolPatchRollbackResponse,
    source_git_root: &Path,
    generated_diff: &str,
    confirm: bool,
) -> SymbolPatchRollbackResponse {
    let patch_file = temp_patch_file("rollback");
    if let Err(error) = fs::write(&patch_file, generated_diff) {
        response.status = PatchRollbackStatus::Failed;
        response
            .blocked_reasons
            .push("write_rollback_patch_failed".to_string());
        response.warnings.push(error.to_string());
        return response;
    }
    let reverse_check = run_git(
        source_git_root,
        &[
            "apply".to_string(),
            "-R".to_string(),
            "--check".to_string(),
            "--whitespace=nowarn".to_string(),
            patch_file.display().to_string(),
        ],
        DEFAULT_TIMEOUT_SECS,
    );
    response.reverse_check = Some(reverse_check.clone());
    if !reverse_check.success {
        response.status = PatchRollbackStatus::Blocked;
        response
            .blocked_reasons
            .push("reverse_patch_check_failed".to_string());
        response.next_steps = vec![
            "Regenerate rollback against the current file contents or use commitSha revert."
                .to_string(),
        ];
        let _ = fs::remove_file(patch_file);
        return response;
    }
    if !confirm {
        response.status = PatchRollbackStatus::DryRunReady;
        response.next_steps = vec![
            "Reverse patch check passed.".to_string(),
            "Set confirm=true to apply git apply -R.".to_string(),
        ];
        let _ = fs::remove_file(patch_file);
        return response;
    }
    let reverse_apply = run_git(
        source_git_root,
        &[
            "apply".to_string(),
            "-R".to_string(),
            "--whitespace=nowarn".to_string(),
            patch_file.display().to_string(),
        ],
        DEFAULT_TIMEOUT_SECS,
    );
    response.rollback_command = Some(reverse_apply.clone());
    let _ = fs::remove_file(patch_file);
    if reverse_apply.success {
        response.status = PatchRollbackStatus::ReversePatchApplied;
        response.next_steps =
            vec!["Run verification, then commit the rollback if it is correct.".to_string()];
    } else {
        response.status = PatchRollbackStatus::Failed;
        response
            .blocked_reasons
            .push("reverse_patch_apply_failed".to_string());
        response.next_steps = vec!["Inspect git apply -R stderr before retrying.".to_string()];
    }
    response
}
