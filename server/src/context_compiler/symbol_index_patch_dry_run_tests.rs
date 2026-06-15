use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    symbol_index::{SymbolEdge, SymbolIndex, SymbolRecord},
    symbol_index_patch_check::PatchDiffCheckStatus,
    symbol_index_patch_dry_run::{PatchApplyGateStatus, dry_run_symbol_patch},
    symbol_index_patch_verification::PatchVerificationStatus,
    symbol_index_patch_verification_repair::{
        PatchVerificationCommandResultInput, PatchVerificationRepairStatus,
        build_patch_verification_repair_context,
    },
    symbol_index_patch_verification_run::run_symbol_patch_verification,
    symbol_index_patch_verification_run_types::PatchVerificationExecutionStatus,
    symbol_index_store::{SYMBOL_INDEX_DB_FILE, write_symbol_index_sqlite},
    symbol_index_task_pack::{SymbolTaskPackQuery, build_latest_symbol_task_pack},
};

#[test]
fn patch_dry_run_accepts_diff_that_git_can_apply() {
    let data_dir = temp_dir("elon_symbol_patch_dry_run_ok_data");
    let workspace = temp_dir("elon_symbol_patch_dry_run_ok_workspace");
    write_bundle(
        &data_dir,
        "20260614",
        "213016-trace-patch-dry-run-ok-user",
        sample_index(),
    );
    init_git_workspace(&workspace);
    write_workspace_file(
        &workspace,
        "server/src/context_compiler/context_pack.rs",
        "pub fn demo() {\n    let status = 500;\n}\n",
    );
    write_workspace_file(
        &workspace,
        "server/src/context_compiler/context_pack_tests.rs",
        "#[test]\nfn demo_test() {\n    assert_eq!(500, 500);\n}\n",
    );

    let response = patch_generation(&data_dir);
    let diff = r#"diff --git a/server/src/context_compiler/context_pack.rs b/server/src/context_compiler/context_pack.rs
--- a/server/src/context_compiler/context_pack.rs
+++ b/server/src/context_compiler/context_pack.rs
@@ -1,3 +1,3 @@
 pub fn demo() {
-    let status = 500;
+    let status = 401;
 }
"#;

    let dry_run = dry_run_symbol_patch(&response.patch_generation, diff, &workspace);

    assert_eq!(
        dry_run.contract_check.status,
        PatchDiffCheckStatus::AcceptedForApplyCheck
    );
    assert!(dry_run.accepted_for_apply_check);
    assert!(dry_run.apply_check.attempted);
    assert!(dry_run.apply_check.success);
    assert_eq!(dry_run.apply_gate.status, PatchApplyGateStatus::Blocked);
    assert!(!dry_run.apply_gate.ready_to_apply);
    assert!(
        dry_run
            .apply_gate
            .blockers
            .iter()
            .any(|blocker| blocker == "workspace_not_clean")
    );
    assert_eq!(dry_run.repair_context.failure_kind, "workspace_not_clean");
    assert!(!dry_run.repair_context.model_repair_required);
    assert!(dry_run.repair_context.retry_prompt.is_none());
    assert_eq!(
        dry_run.verification_plan.status,
        PatchVerificationStatus::Blocked
    );
    assert!(!dry_run.verification_plan.ready_to_verify_after_apply);
    assert!(
        dry_run
            .verification_plan
            .blocked_reasons
            .iter()
            .any(|reason| reason == "workspace_not_clean")
    );
    assert!(
        dry_run
            .verification_plan
            .commands
            .iter()
            .all(|command| !command.auto_runnable_after_apply)
    );
    assert_eq!(dry_run.apply_gate.patch_sha256.len(), 64);
    assert!(
        dry_run
            .apply_check
            .command
            .as_deref()
            .unwrap_or_default()
            .contains("git -C")
    );
    let patch_file = dry_run.apply_check.patch_file.as_deref().unwrap();
    assert!(
        !Path::new(patch_file).exists(),
        "temporary patch file should be removed"
    );

    fs::remove_dir_all(data_dir).unwrap();
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn patch_dry_run_reports_git_apply_check_failure_without_applying() {
    let data_dir = temp_dir("elon_symbol_patch_dry_run_fail_data");
    let workspace = temp_dir("elon_symbol_patch_dry_run_fail_workspace");
    write_bundle(
        &data_dir,
        "20260614",
        "213017-trace-patch-dry-run-fail-user",
        sample_index(),
    );
    init_git_workspace(&workspace);
    write_workspace_file(
        &workspace,
        "server/src/context_compiler/context_pack.rs",
        "pub fn demo() {\n    let status = 500;\n}\n",
    );

    let response = patch_generation(&data_dir);
    let diff = r#"diff --git a/server/src/context_compiler/context_pack.rs b/server/src/context_compiler/context_pack.rs
--- a/server/src/context_compiler/context_pack.rs
+++ b/server/src/context_compiler/context_pack.rs
@@ -1,3 +1,3 @@
 pub fn demo() {
-    let status = 404;
+    let status = 401;
 }
"#;

    let dry_run = dry_run_symbol_patch(&response.patch_generation, diff, &workspace);
    let actual =
        fs::read_to_string(workspace.join("server/src/context_compiler/context_pack.rs")).unwrap();

    assert!(dry_run.contract_check.accepted_for_apply_check);
    assert!(dry_run.apply_check.attempted);
    assert!(!dry_run.apply_check.success);
    assert!(!dry_run.apply_gate.ready_to_apply);
    assert!(
        dry_run
            .apply_gate
            .blockers
            .iter()
            .any(|blocker| blocker == "git_apply_check_failed")
    );
    assert!(
        dry_run.apply_check.stderr.contains("patch failed")
            || dry_run.apply_check.stderr.contains("does not apply")
    );
    assert_eq!(
        dry_run.repair_context.failure_kind,
        "git_apply_check_failed"
    );
    assert!(dry_run.repair_context.model_repair_required);
    assert!(
        dry_run
            .repair_context
            .retry_prompt
            .as_deref()
            .unwrap_or_default()
            .contains("<patch_repair_task>")
    );
    assert!(
        dry_run
            .repair_context
            .repair_instructions
            .iter()
            .any(|instruction| instruction.contains("current workspace file contents"))
    );
    assert!(actual.contains("let status = 500;"));

    fs::remove_dir_all(data_dir).unwrap();
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn patch_dry_run_does_not_run_git_when_contract_rejects_diff() {
    let data_dir = temp_dir("elon_symbol_patch_dry_run_reject_data");
    let workspace = temp_dir("elon_symbol_patch_dry_run_reject_workspace");
    write_bundle(
        &data_dir,
        "20260614",
        "213018-trace-patch-dry-run-reject-user",
        sample_index(),
    );
    init_git_workspace(&workspace);
    write_workspace_file(
        &workspace,
        "server/src/context_compiler/unrelated.rs",
        "pub fn unrelated() {}\n",
    );

    let response = patch_generation(&data_dir);
    let diff = r#"diff --git a/server/src/context_compiler/unrelated.rs b/server/src/context_compiler/unrelated.rs
--- a/server/src/context_compiler/unrelated.rs
+++ b/server/src/context_compiler/unrelated.rs
@@ -1,1 +1,1 @@
-pub fn unrelated() {}
+pub fn unrelated() { println!("changed"); }
"#;

    let dry_run = dry_run_symbol_patch(&response.patch_generation, diff, &workspace);

    assert_eq!(
        dry_run.contract_check.status,
        PatchDiffCheckStatus::Rejected
    );
    assert!(!dry_run.accepted_for_apply_check);
    assert!(!dry_run.apply_check.attempted);
    assert!(!dry_run.apply_gate.ready_to_apply);
    assert!(
        dry_run
            .apply_gate
            .blockers
            .iter()
            .any(|blocker| blocker == "diff_contract_not_accepted")
    );
    assert!(
        dry_run
            .contract_check
            .violations
            .iter()
            .any(|violation| violation.code == "file_not_allowed")
    );
    assert_eq!(
        dry_run.repair_context.failure_kind,
        "diff_contract_rejected"
    );
    assert!(dry_run.repair_context.model_repair_required);
    assert!(
        dry_run
            .repair_context
            .contract_violations
            .iter()
            .any(|violation| violation.code == "file_not_allowed")
    );

    fs::remove_dir_all(data_dir).unwrap();
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn patch_dry_run_marks_clean_workspace_ready_for_apply() {
    let data_dir = temp_dir("elon_symbol_patch_dry_run_ready_data");
    let workspace = temp_dir("elon_symbol_patch_dry_run_ready_workspace");
    write_bundle(
        &data_dir,
        "20260614",
        "213019-trace-patch-dry-run-ready-user",
        sample_index(),
    );
    init_git_workspace(&workspace);
    write_workspace_file(
        &workspace,
        "server/src/context_compiler/context_pack.rs",
        "pub fn demo() {\n    let status = 500;\n}\n",
    );
    write_workspace_file(
        &workspace,
        "server/src/context_compiler/context_pack_tests.rs",
        "#[test]\nfn demo_test() {\n    assert_eq!(500, 500);\n}\n",
    );
    commit_workspace(&workspace);

    let response = patch_generation(&data_dir);
    let diff = r#"diff --git a/server/src/context_compiler/context_pack.rs b/server/src/context_compiler/context_pack.rs
--- a/server/src/context_compiler/context_pack.rs
+++ b/server/src/context_compiler/context_pack.rs
@@ -1,3 +1,3 @@
 pub fn demo() {
-    let status = 500;
+    let status = 401;
 }
"#;

    let dry_run = dry_run_symbol_patch(&response.patch_generation, diff, &workspace);

    assert!(dry_run.workspace.clean);
    assert!(dry_run.apply_check.success);
    assert!(dry_run.apply_gate.ready_to_apply);
    assert_eq!(dry_run.apply_gate.status, PatchApplyGateStatus::Ready);
    assert!(dry_run.apply_gate.blockers.is_empty());
    assert!(
        dry_run
            .apply_gate
            .safe_apply_command
            .as_deref()
            .unwrap_or_default()
            .contains("git -C")
    );
    assert!(
        dry_run
            .apply_gate
            .verification_commands
            .iter()
            .any(|command| command.contains("git status --short"))
    );
    assert_eq!(dry_run.repair_context.failure_kind, "none");
    assert!(!dry_run.repair_context.model_repair_required);
    assert!(dry_run.repair_context.retry_prompt.is_none());
    assert_eq!(
        dry_run.verification_plan.status,
        PatchVerificationStatus::ReadyAfterApply
    );
    assert!(dry_run.verification_plan.ready_to_verify_after_apply);
    assert_eq!(
        dry_run.verification_plan.repair_policy.max_repair_attempts,
        2
    );
    assert!(
        dry_run
            .verification_plan
            .repair_policy
            .model_repair_on_failure
    );
    assert!(dry_run.verification_plan.commands.iter().any(
        |command| command.category == "test" && command.failure_kind == "targeted_tests_failed"
    ));
    assert!(
        dry_run
            .verification_plan
            .commands
            .iter()
            .any(|command| command.category == "diff_hygiene" && command.auto_runnable_after_apply)
    );
    assert!(
        dry_run
            .verification_plan
            .commands
            .iter()
            .any(|command| command.category == "workspace_status"
                && command.auto_runnable_after_apply)
    );

    fs::remove_dir_all(data_dir).unwrap();
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn verification_repair_context_prompts_for_failed_targeted_test() {
    let (data_dir, workspace, response, diff, dry_run) =
        ready_patch_fixture("elon_symbol_patch_verify_repair_fail", "213020");
    let failed_command = dry_run
        .verification_plan
        .commands
        .iter()
        .find(|command| command.category == "test")
        .unwrap()
        .command
        .clone();

    let context = build_patch_verification_repair_context(
        &response.patch_generation,
        &dry_run,
        &diff,
        &[PatchVerificationCommandResultInput {
            command: failed_command,
            exit_code: Some(101),
            stdout: Some("running 1 test\ncontext_pack_tests::demo_test FAILED".to_string()),
            stderr: Some("assertion failed: expected status 401, got 500".to_string()),
            elapsed_ms: Some(1_250),
        }],
    );

    assert_eq!(
        context.status,
        PatchVerificationRepairStatus::ReadyForRepair
    );
    assert!(context.model_repair_required);
    assert_eq!(context.failed_commands.len(), 1);
    assert_eq!(
        context.failed_commands[0].failure_kind,
        "targeted_tests_failed"
    );
    assert!(context.failed_commands[0].matched_plan);
    assert!(context.passed_commands.is_empty());
    assert!(
        context
            .allowed_files
            .iter()
            .any(|file| file == "server/src/context_compiler/context_pack.rs")
    );
    assert!(
        context
            .retry_prompt
            .as_deref()
            .unwrap_or_default()
            .contains("<patch_verification_repair_task>")
    );
    assert!(
        context
            .retry_prompt
            .as_deref()
            .unwrap_or_default()
            .contains("targeted_tests_failed")
    );

    fs::remove_dir_all(data_dir).unwrap();
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn verification_repair_context_does_not_prompt_when_all_commands_pass() {
    let (data_dir, workspace, response, diff, dry_run) =
        ready_patch_fixture("elon_symbol_patch_verify_repair_pass", "213021");
    let results = dry_run
        .verification_plan
        .commands
        .iter()
        .map(|command| PatchVerificationCommandResultInput {
            command: command.command.clone(),
            exit_code: Some(0),
            stdout: Some(String::new()),
            stderr: Some(String::new()),
            elapsed_ms: Some(10),
        })
        .collect::<Vec<_>>();

    let context = build_patch_verification_repair_context(
        &response.patch_generation,
        &dry_run,
        &diff,
        &results,
    );

    assert_eq!(
        context.status,
        PatchVerificationRepairStatus::VerificationPassed
    );
    assert!(!context.model_repair_required);
    assert!(context.failed_commands.is_empty());
    assert_eq!(context.passed_commands.len(), results.len());
    assert!(context.retry_prompt.is_none());

    fs::remove_dir_all(data_dir).unwrap();
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn verification_run_applies_patch_in_isolated_worktree_and_cleans_up() {
    let (data_dir, workspace, mut response, diff, _dry_run) =
        ready_patch_fixture("elon_symbol_patch_verify_run_pass", "213022");
    response.patch_generation.apply_readiness.post_apply_checks = vec![
        "git diff --check".to_string(),
        "git status --short".to_string(),
    ];
    let original =
        fs::read_to_string(workspace.join("server/src/context_compiler/context_pack.rs")).unwrap();

    let result = run_symbol_patch_verification(&response.patch_generation, &diff, &workspace);

    assert_eq!(
        result.execution.status,
        PatchVerificationExecutionStatus::Passed
    );
    assert!(result.execution.patch_applied);
    assert!(result.execution.run_workspace_removed);
    assert_eq!(result.execution.executed_commands.len(), 3);
    assert!(result.execution.skipped_commands.is_empty());
    assert_eq!(
        result.verification_repair_context.status,
        PatchVerificationRepairStatus::VerificationPassed
    );
    let run_workspace = result.execution.run_workspace.as_deref().unwrap();
    assert!(!Path::new(run_workspace).exists());
    let current =
        fs::read_to_string(workspace.join("server/src/context_compiler/context_pack.rs")).unwrap();
    assert_eq!(current, original, "source workspace must remain untouched");
    assert_git_clean(&workspace);

    fs::remove_dir_all(data_dir).unwrap();
    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn verification_run_builds_repair_context_when_auto_command_fails() {
    let (data_dir, workspace, mut response, diff, _dry_run) =
        ready_patch_fixture("elon_symbol_patch_verify_run_fail", "213023");
    response.patch_generation.apply_readiness.post_apply_checks =
        vec!["cargo test demo_target".to_string()];

    let result = run_symbol_patch_verification(&response.patch_generation, &diff, &workspace);

    assert_eq!(
        result.execution.status,
        PatchVerificationExecutionStatus::VerificationFailed
    );
    assert!(result.execution.patch_applied);
    assert!(result.execution.run_workspace_removed);
    assert!(
        result
            .execution
            .executed_commands
            .iter()
            .any(|command| command.command == "cargo test demo_target"
                && command.exit_code != Some(0))
    );
    assert_eq!(
        result.verification_repair_context.status,
        PatchVerificationRepairStatus::ReadyForRepair
    );
    assert!(result.verification_repair_context.model_repair_required);
    assert!(
        result
            .verification_repair_context
            .failed_commands
            .iter()
            .any(|command| command.failure_kind == "targeted_tests_failed")
    );
    assert!(
        result
            .verification_repair_context
            .retry_prompt
            .as_deref()
            .unwrap_or_default()
            .contains("<patch_verification_repair_task>")
    );

    fs::remove_dir_all(data_dir).unwrap();
    fs::remove_dir_all(workspace).unwrap();
}

fn patch_generation(data_dir: &Path) -> super::symbol_index_task_pack::SymbolTaskPackResponse {
    build_latest_symbol_task_pack(
        data_dir,
        &SymbolTaskPackQuery {
            text: Some("把 build_context_pack 报错 500 改成 401".to_string()),
            path: Some("context_pack.rs".to_string()),
            depth: 1,
            search_limit: 5,
            chunk_limit: 10,
            impact_limit: 20,
            max_chars: 24_000,
            ..Default::default()
        },
    )
    .unwrap()
}

fn ready_patch_fixture(
    prefix: &str,
    trace_suffix: &str,
) -> (
    PathBuf,
    PathBuf,
    super::symbol_index_task_pack::SymbolTaskPackResponse,
    String,
    super::symbol_index_patch_dry_run::SymbolPatchDryRunResponse,
) {
    let data_dir = temp_dir(&format!("{prefix}_data"));
    let workspace = temp_dir(&format!("{prefix}_workspace"));
    write_bundle(
        &data_dir,
        "20260614",
        &format!("{trace_suffix}-trace-patch-verify-repair-user"),
        sample_index(),
    );
    init_git_workspace(&workspace);
    write_workspace_file(
        &workspace,
        "server/src/context_compiler/context_pack.rs",
        "pub fn demo() {\n    let status = 500;\n}\n",
    );
    write_workspace_file(
        &workspace,
        "server/src/context_compiler/context_pack_tests.rs",
        "#[test]\nfn demo_test() {\n    assert_eq!(500, 500);\n}\n",
    );
    commit_workspace(&workspace);

    let response = patch_generation(&data_dir);
    let diff = r#"diff --git a/server/src/context_compiler/context_pack.rs b/server/src/context_compiler/context_pack.rs
--- a/server/src/context_compiler/context_pack.rs
+++ b/server/src/context_compiler/context_pack.rs
@@ -1,3 +1,3 @@
 pub fn demo() {
-    let status = 500;
+    let status = 401;
 }
"#
    .to_string();
    let dry_run = dry_run_symbol_patch(&response.patch_generation, &diff, &workspace);

    (data_dir, workspace, response, diff, dry_run)
}

fn write_bundle(data_dir: &Path, day: &str, stem: &str, index: SymbolIndex) -> PathBuf {
    let bundle = data_dir.join("context-compiler").join(day).join(stem);
    fs::create_dir_all(&bundle).unwrap();
    let db = bundle.join(SYMBOL_INDEX_DB_FILE);
    let mut files = Vec::new();
    write_symbol_index_sqlite(&db, &index, &mut files).unwrap();
    db
}

fn init_git_workspace(workspace: &Path) {
    fs::create_dir_all(workspace).unwrap();
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .arg("init")
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn commit_workspace(workspace: &Path) {
    run_git(workspace, &["config", "user.email", "test@example.com"]);
    run_git(workspace, &["config", "user.name", "Test User"]);
    run_git(workspace, &["add", "."]);
    run_git(workspace, &["commit", "-m", "seed"]);
}

fn assert_git_clean(workspace: &Path) {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(["status", "--short"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).trim().is_empty(),
        "source workspace should be clean after isolated verification"
    );
}

fn run_git(workspace: &Path, args: &[&str]) {
    let output = Command::new("git")
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

fn write_workspace_file(workspace: &Path, relative_path: &str, content: &str) {
    let path = workspace.join(relative_path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn sample_index() -> SymbolIndex {
    SymbolIndex {
        records: vec![
            symbol(
                "server/src/context_compiler/context_pack.rs::build_context_pack",
                "build_context_pack",
                "fn",
                "server/src/context_compiler/context_pack.rs",
                10,
                "pub(crate) fn build_context_pack(...) -> String",
                Some(8.2),
            ),
            symbol(
                "server/src/context_compiler/context_pack_tests.rs::build_context_pack_test",
                "build_context_pack_test",
                "fn",
                "server/src/context_compiler/context_pack_tests.rs",
                22,
                "#[test] fn build_context_pack_test()",
                Some(3.0),
            ),
        ],
        edges: vec![SymbolEdge {
            id: "edge-test".to_string(),
            source: "rust_analyzer_lsp",
            kind: "test_covers".to_string(),
            from_symbol_id: Some(
                "server/src/context_compiler/context_pack_tests.rs::build_context_pack_test"
                    .to_string(),
            ),
            from_path: "server/src/context_compiler/context_pack_tests.rs".to_string(),
            line: 24,
            to_symbol_id: Some(
                "server/src/context_compiler/context_pack.rs::build_context_pack".to_string(),
            ),
            to_symbol_name: Some("build_context_pack".to_string()),
            to_path: Some("server/src/context_compiler/context_pack.rs".to_string()),
            confidence: 0.9,
            reason: "test covers symbol".to_string(),
        }],
        ..Default::default()
    }
}

fn symbol(
    id: &str,
    name: &str,
    kind: &str,
    file_path: &str,
    start_line: usize,
    signature: &str,
    importance_score: Option<f64>,
) -> SymbolRecord {
    SymbolRecord {
        id: id.to_string(),
        name: name.to_string(),
        qualified_name: id.to_string(),
        kind: kind.to_string(),
        language: "rust",
        file_path: file_path.to_string(),
        start_line,
        end_line: start_line + 10,
        signature: signature.to_string(),
        visibility: "pub".to_string(),
        parent_symbol_id: None,
        module_path: file_path.replace('/', "::"),
        doc_summary: None,
        role: "definition",
        importance_score,
        signature_hash: format!("{name}-hash"),
        source_providers: vec!["rust_symbols".to_string()],
    }
}

fn temp_dir(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), nonce))
}
