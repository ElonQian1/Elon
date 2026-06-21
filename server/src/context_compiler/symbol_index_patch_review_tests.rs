use super::{
    symbol_index_compression_types::CompressionLevel,
    symbol_index_patch_generation_types::{
        PatchApplyReadiness, PatchApplyReadinessLevel, PatchDiffContract, PatchGenerationMode,
        SymbolPatchGeneration,
    },
    symbol_index_patch_plan_types::{
        PatchEditPriority, PatchEditTarget, PatchEditType, PatchPlanSummary, PatchPlanningDecision,
        PatchPlanningTrace, PatchTestPlan, SymbolPatchPlan,
    },
    symbol_index_patch_review::build_symbol_patch_review,
    symbol_index_patch_review_types::{PatchReviewDecision, PatchReviewSeverity},
    symbol_index_ranker::RerankDecision,
    symbol_index_retrieval_plan::QueryIntent,
};

#[test]
fn review_rejects_patch_that_violates_allowed_files() {
    let plan = sample_plan(true);
    let generation = sample_generation(vec![
        "src/auth.rs".to_string(),
        "tests/auth_login_test.rs".to_string(),
    ]);
    let diff = r#"diff --git a/src/token.rs b/src/token.rs
--- a/src/token.rs
+++ b/src/token.rs
@@ -1,3 +1,3 @@
-let status = 500;
+let status = 401;
"#;

    let review = build_symbol_patch_review(&plan, &generation, diff, None);

    assert_eq!(review.decision, PatchReviewDecision::Reject);
    assert!(review
        .findings
        .iter()
        .any(|finding| finding.severity == PatchReviewSeverity::Critical));
}

#[test]
fn review_requires_human_when_required_test_file_is_missing() {
    let plan = sample_plan(true);
    let generation = sample_generation(vec![
        "src/auth.rs".to_string(),
        "tests/auth_login_test.rs".to_string(),
    ]);
    let diff = r#"diff --git a/src/auth.rs b/src/auth.rs
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,3 +1,3 @@
-let status = 500;
+let status = 401;
"#;

    let review = build_symbol_patch_review(&plan, &generation, diff, None);

    assert_eq!(review.decision, PatchReviewDecision::NeedsHumanReview);
    assert!(review
        .findings
        .iter()
        .any(|finding| finding.code == "required_file_missing"));
}

#[test]
fn review_approves_with_notes_for_small_patch_without_verification_report() {
    let plan = sample_plan(true);
    let generation = sample_generation(vec![
        "src/auth.rs".to_string(),
        "tests/auth_login_test.rs".to_string(),
    ]);
    let diff = r#"diff --git a/src/auth.rs b/src/auth.rs
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,3 +1,3 @@
-let status = 500;
+let status = 401;
diff --git a/tests/auth_login_test.rs b/tests/auth_login_test.rs
--- a/tests/auth_login_test.rs
+++ b/tests/auth_login_test.rs
@@ -1,3 +1,3 @@
-assert_eq!(status, 500);
+assert_eq!(status, 401);
"#;

    let review = build_symbol_patch_review(&plan, &generation, diff, None);

    assert_eq!(review.decision, PatchReviewDecision::ApproveWithNotes);
    assert_eq!(review.plan_compliance.required_files_missing.len(), 0);
    assert!(review.review_report_markdown.contains("<patch_review>"));
    assert!(review
        .affected_symbols
        .iter()
        .any(|symbol| symbol == "auth"));
}

#[test]
fn review_requires_human_when_patch_adds_unsafe_code() {
    let plan = sample_plan(false);
    let generation = sample_generation(vec!["src/auth.rs".to_string()]);
    let diff = r#"diff --git a/src/auth.rs b/src/auth.rs
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,3 +1,4 @@
 let status = 401;
+let ptr = unsafe { raw_ptr.read() };
"#;

    let review = build_symbol_patch_review(&plan, &generation, diff, None);

    assert_eq!(review.decision, PatchReviewDecision::NeedsHumanReview);
    assert!(review
        .findings
        .iter()
        .any(|finding| finding.code == "unsafe_added"));
}

fn sample_plan(include_test: bool) -> SymbolPatchPlan {
    let mut must_edit = vec![target("src/auth.rs", "auth", PatchEditType::ModifyBehavior)];
    if include_test {
        must_edit.push(target(
            "tests/auth_login_test.rs",
            "auth_login_test",
            PatchEditType::UpdateTest,
        ));
    }

    SymbolPatchPlan {
        task: "把登录失败时的 500 改成 401".to_string(),
        intent: QueryIntent::ModifyBehavior,
        plan_kind: "code_patch".to_string(),
        patch_required: true,
        summary: PatchPlanSummary {
            must_edit_count: must_edit.len(),
            should_inspect_count: 0,
            maybe_edit_count: 0,
            test_target_count: usize::from(include_test),
            risk_count: 0,
        },
        must_edit,
        should_inspect: Vec::new(),
        maybe_edit: Vec::new(),
        proposed_changes: Vec::new(),
        test_plan: PatchTestPlan {
            commands: vec!["cargo test auth_login".to_string()],
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

fn sample_generation(allowed_files: Vec<String>) -> SymbolPatchGeneration {
    SymbolPatchGeneration {
        task: "把登录失败时的 500 改成 401".to_string(),
        mode: PatchGenerationMode::GenerateDiff,
        ready_to_generate: true,
        edit_sequence: Vec::new(),
        diff_contract: PatchDiffContract {
            output_format: "unified_diff".to_string(),
            apply_strategy: "git_apply_check".to_string(),
            allowed_files,
            inspect_only_files: Vec::new(),
            forbidden_patterns: Vec::new(),
            required_tests: vec!["cargo test auth_login".to_string()],
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
