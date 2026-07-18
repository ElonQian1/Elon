use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    symbol_index_compression_types::CompressionLevel,
    symbol_index_patch_apply::{apply_reviewed_symbol_patch, rollback_symbol_patch},
    symbol_index_patch_apply_types::{
        PatchApplyMode, PatchApplyOptions, PatchApplyWorkflowStatus, PatchRollbackStatus,
    },
    symbol_index_patch_generation_types::{
        PatchApplyReadiness, PatchApplyReadinessLevel, PatchDiffContract, PatchGenerationMode,
        SymbolPatchGeneration,
    },
    symbol_index_patch_plan_types::{
        PatchEditPriority, PatchEditTarget, PatchEditType, PatchPlanSummary, PatchPlanningDecision,
        PatchPlanningTrace, PatchTestPlan, SymbolPatchPlan,
    },
    symbol_index_patch_review_types::PatchReviewDecision,
    symbol_index_patch_verification_run_types::PatchVerificationExecutionStatus,
    symbol_index_ranker::RerankDecision,
    symbol_index_retrieval_plan::QueryIntent,
};

#[test]
fn apply_workflow_defaults_to_dry_run_without_mutating_source_workspace() {
    let workspace = ready_workspace("elon_symbol_patch_apply_dry_run");
    let plan = sample_plan();
    let generation = sample_generation();
    let diff = sample_diff();

    let result = apply_reviewed_symbol_patch(
        &plan,
        &generation,
        diff,
        &workspace,
        PatchApplyOptions {
            mode: PatchApplyMode::DryRun,
            confirm: false,
            commit: false,
            keep_worktree: true,
            branch_name: None,
            commit_message: None,
            require_review_approval: true,
        },
    );

    assert_eq!(result.status, PatchApplyWorkflowStatus::DryRunReady);
    assert_eq!(result.review_decision, PatchReviewDecision::Approve);
    assert_eq!(
        result.verification.execution.status,
        PatchVerificationExecutionStatus::Passed
    );
    assert!(result.run_workspace.is_none());
    assert!(read_file(&workspace, "src/auth.rs").contains("status = 500"));
    assert_git_clean(&workspace);

    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn apply_workflow_commits_patch_on_isolated_branch_and_removes_worktree() {
    let workspace = ready_workspace("elon_symbol_patch_apply_commit");
    let plan = sample_plan();
    let generation = sample_generation();
    let diff = sample_diff();
    let branch = format!("repoctx/test-{}", nonce());

    let result = apply_reviewed_symbol_patch(
        &plan,
        &generation,
        diff,
        &workspace,
        PatchApplyOptions {
            mode: PatchApplyMode::TemporaryWorktree,
            confirm: true,
            commit: true,
            keep_worktree: false,
            branch_name: Some(branch.clone()),
            commit_message: Some("fix(auth): return 401 for invalid login".to_string()),
            require_review_approval: true,
        },
    );

    assert_eq!(result.status, PatchApplyWorkflowStatus::CommittedToBranch);
    assert_eq!(result.branch_name.as_deref(), Some(branch.as_str()));
    assert!(result.commit_sha.is_some());
    assert!(result.run_workspace_removed);
    assert!(result
        .rollback
        .revert_commit_command
        .as_deref()
        .unwrap_or_default()
        .contains("git -C"));
    assert!(read_file(&workspace, "src/auth.rs").contains("status = 500"));
    assert_git_clean(&workspace);

    run_git(&workspace, &["branch", "-D", &branch]);
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn rollback_reverse_patch_allows_dirty_uncommitted_apply() {
    let workspace = ready_workspace("elon_symbol_patch_rollback_reverse");
    let diff = sample_diff();
    let patch_file = workspace.join("change.patch");
    fs::write(&patch_file, diff).unwrap();
    run_git(
        &workspace,
        &["apply", "--whitespace=nowarn", patch_file.to_str().unwrap()],
    );
    assert!(read_file(&workspace, "src/auth.rs").contains("status = 401"));

    let dry_run = rollback_symbol_patch(&workspace, Some(diff), None, false);
    assert_eq!(dry_run.status, PatchRollbackStatus::DryRunReady);
    assert!(dry_run
        .reverse_check
        .as_ref()
        .is_some_and(|check| check.success));

    let rollback = rollback_symbol_patch(&workspace, Some(diff), None, true);
    assert_eq!(rollback.status, PatchRollbackStatus::ReversePatchApplied);
    assert!(read_file(&workspace, "src/auth.rs").contains("status = 500"));
    fs::remove_file(patch_file).unwrap();
    assert_git_clean(&workspace);

    fs::remove_dir_all(workspace).unwrap();
}

fn ready_workspace(prefix: &str) -> PathBuf {
    let workspace = temp_dir(prefix);
    fs::create_dir_all(&workspace).unwrap();
    run_git(&workspace, &["init"]);
    run_git(&workspace, &["config", "user.email", "test@example.com"]);
    run_git(&workspace, &["config", "user.name", "Test User"]);
    run_git(&workspace, &["config", "core.autocrlf", "false"]);
    write_file(
        &workspace,
        "src/auth.rs",
        "pub fn login() -> i32 {\n    let status = 500;\n    status\n}\n",
    );
    write_file(
        &workspace,
        "tests/auth_login_test.rs",
        "#[test]\nfn rejects_wrong_password() {\n    assert_eq!(500, 500);\n}\n",
    );
    run_git(&workspace, &["add", "."]);
    run_git(&workspace, &["commit", "-m", "seed"]);
    workspace
}

fn sample_plan() -> SymbolPatchPlan {
    SymbolPatchPlan {
        task: "把登录失败时的 500 改成 401".to_string(),
        intent: QueryIntent::ModifyBehavior,
        plan_kind: "code_patch".to_string(),
        patch_required: true,
        summary: PatchPlanSummary {
            must_edit_count: 2,
            should_inspect_count: 0,
            maybe_edit_count: 0,
            test_target_count: 1,
            risk_count: 0,
        },
        must_edit: vec![
            target("src/auth.rs", "auth", PatchEditType::ModifyBehavior),
            target(
                "tests/auth_login_test.rs",
                "auth_login_test",
                PatchEditType::UpdateTest,
            ),
        ],
        should_inspect: Vec::new(),
        maybe_edit: Vec::new(),
        proposed_changes: Vec::new(),
        test_plan: PatchTestPlan {
            commands: Vec::new(),
            target_tests: Vec::new(),
            expected_behavior: Vec::new(),
        },
        risk_notes: Vec::new(),
        open_questions: Vec::new(),
        trace: vec![PatchPlanningTrace {
            rank: 1,
            file_path: "src/auth.rs".to_string(),
            label: "auth".to_string(),
            decision: PatchPlanningDecision::MustEdit,
            reasons: vec!["direct behavior owner".to_string()],
        }],
    }
}

fn target(file_path: &str, symbol: &str, edit_type: PatchEditType) -> PatchEditTarget {
    PatchEditTarget {
        file_path: file_path.to_string(),
        symbol_id: Some(symbol.to_string()),
        qualified_name: Some(symbol.to_string()),
        start_line: Some(1),
        end_line: Some(10),
        edit_type,
        priority: PatchEditPriority::Required,
        reason: "required by task".to_string(),
        source_rank: 1,
        source_decision: RerankDecision::MustInclude,
        compression_level: CompressionLevel::FocusedSnippet,
        sources: vec!["symbol".to_string()],
    }
}

fn sample_generation() -> SymbolPatchGeneration {
    SymbolPatchGeneration {
        task: "把登录失败时的 500 改成 401".to_string(),
        mode: PatchGenerationMode::GenerateDiff,
        ready_to_generate: true,
        edit_sequence: Vec::new(),
        diff_contract: PatchDiffContract {
            output_format: "unified_diff".to_string(),
            apply_strategy: "git_apply_check".to_string(),
            allowed_files: vec![
                "src/auth.rs".to_string(),
                "tests/auth_login_test.rs".to_string(),
            ],
            inspect_only_files: Vec::new(),
            forbidden_patterns: Vec::new(),
            required_tests: Vec::new(),
            verification_commands: Vec::new(),
            safety_checks: Vec::new(),
        },
        apply_readiness: PatchApplyReadiness {
            level: PatchApplyReadinessLevel::ReadyAfterDiff,
            apply_check_status: "ready".to_string(),
            can_run_apply_check: true,
            requires_generated_diff: true,
            source_requirements: Vec::new(),
            pre_apply_checks: Vec::new(),
            post_apply_checks: Vec::new(),
            rollback_strategy: "git apply -R".to_string(),
            risk_level: "low".to_string(),
            notes: Vec::new(),
        },
        prompt: "Return a diff.".to_string(),
        blocked_reasons: Vec::new(),
        trace: Vec::new(),
    }
}

fn sample_diff() -> &'static str {
    r#"diff --git a/src/auth.rs b/src/auth.rs
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,4 +1,4 @@
 pub fn login() -> i32 {
-    let status = 500;
+    let status = 401;
     status
 }
diff --git a/tests/auth_login_test.rs b/tests/auth_login_test.rs
--- a/tests/auth_login_test.rs
+++ b/tests/auth_login_test.rs
@@ -1,4 +1,4 @@
 #[test]
 fn rejects_wrong_password() {
-    assert_eq!(500, 500);
+    assert_eq!(401, 401);
 }
"#
}

fn write_file(workspace: &Path, relative_path: &str, content: &str) {
    let path = workspace.join(relative_path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn read_file(workspace: &Path, relative_path: &str) -> String {
    fs::read_to_string(workspace.join(relative_path)).unwrap()
}

fn assert_git_clean(workspace: &Path) {
    let output = crate::git_command_error::git_command()
        .arg("-C")
        .arg(workspace)
        .args(["status", "--short"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).trim().is_empty(),
        "git workspace should be clean"
    );
}

fn run_git(workspace: &Path, args: &[&str]) {
    let output = crate::git_command_error::git_command()
        .arg("-C")
        .arg(workspace)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn temp_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), nonce()))
}

fn nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
