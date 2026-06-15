use std::collections::BTreeSet;

use super::{
    symbol_index_patch_check::SymbolPatchDiffCheck,
    symbol_index_patch_generation_types::SymbolPatchGeneration,
    symbol_index_patch_plan_types::{PatchEditTarget, PatchEditType, SymbolPatchPlan},
    symbol_index_patch_review_types::{
        PatchReviewPlanCompliance, PatchReviewScope, PatchReviewTestAdequacy,
        PatchReviewVerificationSummary,
    },
    symbol_index_patch_verification_run_types::{
        PatchVerificationExecutionStatus, SymbolPatchVerificationRunResponse,
    },
};

pub(crate) fn build_plan_compliance(
    plan: &SymbolPatchPlan,
    generation: &SymbolPatchGeneration,
    scope: &PatchReviewScope,
    contract_check: &SymbolPatchDiffCheck,
) -> PatchReviewPlanCompliance {
    let touched = scope.touched_files.iter().cloned().collect::<BTreeSet<_>>();
    let required_files = edit_files(&plan.must_edit);
    let expected_files = plan
        .must_edit
        .iter()
        .chain(plan.maybe_edit.iter())
        .map(|target| target.file_path.clone())
        .collect::<BTreeSet<_>>();
    let required_files_touched = required_files
        .iter()
        .filter(|file| touched.contains(*file))
        .cloned()
        .collect::<Vec<_>>();
    let required_files_missing = required_files
        .iter()
        .filter(|file| !touched.contains(*file))
        .cloned()
        .collect::<Vec<_>>();
    let unexpected_files_touched = scope
        .touched_files
        .iter()
        .filter(|file| !expected_files.contains(*file))
        .cloned()
        .collect::<Vec<_>>();
    let mut forbidden_files_touched = contract_check
        .touched_files
        .iter()
        .filter(|file| !file.allowed || file.inspect_only)
        .map(|file| file.file_path.clone())
        .collect::<Vec<_>>();
    forbidden_files_touched.extend(
        scope
            .touched_files
            .iter()
            .filter(|file| !generation.diff_contract.allowed_files.contains(*file))
            .cloned(),
    );
    forbidden_files_touched = dedupe(forbidden_files_touched);

    let must_edit_coverage = if required_files.is_empty() {
        1.0
    } else {
        required_files_touched.len() as f32 / required_files.len() as f32
    };

    PatchReviewPlanCompliance {
        required_files,
        required_files_touched,
        required_files_missing,
        unexpected_files_touched,
        forbidden_files_touched,
        must_edit_coverage,
    }
}

pub(crate) fn build_scope(
    contract_check: &SymbolPatchDiffCheck,
    generated_diff: &str,
) -> PatchReviewScope {
    let touched_files = contract_check
        .touched_files
        .iter()
        .map(|file| file.file_path.clone())
        .collect::<Vec<_>>();
    let added_files = contract_check
        .touched_files
        .iter()
        .filter(|file| file.change_kind == "added")
        .map(|file| file.file_path.clone())
        .collect::<Vec<_>>();
    let deleted_files = contract_check
        .touched_files
        .iter()
        .filter(|file| file.change_kind == "deleted")
        .map(|file| file.file_path.clone())
        .collect::<Vec<_>>();
    let hunk_count = contract_check
        .touched_files
        .iter()
        .map(|file| file.hunk_count)
        .sum();
    let test_files_touched = touched_files
        .iter()
        .filter(|file| looks_like_test_file(file))
        .cloned()
        .collect::<Vec<_>>();
    let (added_lines, removed_lines) = line_delta(generated_diff);

    PatchReviewScope {
        touched_file_count: touched_files.len(),
        touched_files,
        added_lines,
        removed_lines,
        hunk_count,
        added_files,
        deleted_files,
        test_files_touched,
    }
}

pub(crate) fn build_test_adequacy(
    plan: &SymbolPatchPlan,
    scope: &PatchReviewScope,
) -> PatchReviewTestAdequacy {
    let required_test_files = plan
        .must_edit
        .iter()
        .filter(|target| {
            looks_like_test_file(&target.file_path)
                || matches!(
                    target.edit_type,
                    PatchEditType::AddTest | PatchEditType::UpdateTest
                )
        })
        .map(|target| target.file_path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let touched = scope
        .test_files_touched
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let missing_test_files = required_test_files
        .iter()
        .filter(|file| !touched.contains(*file))
        .cloned()
        .collect::<Vec<_>>();
    let expected = !required_test_files.is_empty()
        || !plan.test_plan.commands.is_empty()
        || !plan.test_plan.target_tests.is_empty()
        || plan
            .must_edit
            .iter()
            .any(|target| behavior_change_needs_test(target.edit_type));
    let mut warnings = Vec::new();
    if expected && scope.test_files_touched.is_empty() {
        warnings.push("expected_test_context_but_no_test_file_touched".to_string());
    }
    if !missing_test_files.is_empty() {
        warnings.push("required_test_file_not_touched".to_string());
    }
    let status = if !expected {
        "not_required"
    } else if !missing_test_files.is_empty() {
        "missing_required_test_file"
    } else if scope.test_files_touched.is_empty() {
        "weak"
    } else {
        "covered"
    }
    .to_string();

    PatchReviewTestAdequacy {
        expected,
        status,
        required_test_files,
        touched_test_files: scope.test_files_touched.clone(),
        missing_test_files,
        required_commands: plan.test_plan.commands.clone(),
        warnings,
    }
}

pub(crate) fn build_verification_summary(
    verification: Option<&SymbolPatchVerificationRunResponse>,
) -> PatchReviewVerificationSummary {
    let Some(verification) = verification else {
        return PatchReviewVerificationSummary {
            provided: false,
            status: None,
            success: false,
            executed_command_count: 0,
            failed_commands: Vec::new(),
            skipped_required_commands: Vec::new(),
            blocked_reasons: Vec::new(),
        };
    };
    let failed_commands = verification
        .execution
        .executed_commands
        .iter()
        .filter(|command| !matches!(command.exit_code, Some(0)) || command.timed_out)
        .map(|command| command.command.clone())
        .collect::<Vec<_>>();
    let skipped_required_commands = verification
        .execution
        .skipped_commands
        .iter()
        .filter(|command| command.required)
        .map(|command| command.command.clone())
        .collect::<Vec<_>>();
    let success = matches!(
        verification.execution.status,
        PatchVerificationExecutionStatus::Passed
    );

    PatchReviewVerificationSummary {
        provided: true,
        status: Some(verification.execution.status),
        success,
        executed_command_count: verification.execution.executed_commands.len(),
        failed_commands,
        skipped_required_commands,
        blocked_reasons: verification.execution.blocked_reasons.clone(),
    }
}

pub(crate) fn affected_symbols(plan: &SymbolPatchPlan, touched_files: &[String]) -> Vec<String> {
    let touched = touched_files.iter().cloned().collect::<BTreeSet<_>>();
    plan.must_edit
        .iter()
        .chain(plan.maybe_edit.iter())
        .chain(plan.should_inspect.iter())
        .filter(|target| touched.contains(&target.file_path))
        .filter_map(|target| {
            target
                .qualified_name
                .clone()
                .or_else(|| target.symbol_id.clone())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn diff_added_line(line: &str) -> Option<&str> {
    line.strip_prefix('+').filter(|_| !line.starts_with("+++ "))
}

pub(crate) fn diff_removed_line(line: &str) -> Option<&str> {
    line.strip_prefix('-').filter(|_| !line.starts_with("--- "))
}

fn edit_files(targets: &[PatchEditTarget]) -> Vec<String> {
    targets
        .iter()
        .filter(|target| target.edit_type != PatchEditType::InspectOnly)
        .map(|target| target.file_path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn line_delta(diff: &str) -> (usize, usize) {
    let mut added = 0;
    let mut removed = 0;
    for line in diff.lines() {
        if diff_added_line(line).is_some() {
            added += 1;
        } else if diff_removed_line(line).is_some() {
            removed += 1;
        }
    }
    (added, removed)
}

fn behavior_change_needs_test(edit_type: PatchEditType) -> bool {
    matches!(
        edit_type,
        PatchEditType::ModifyBehavior
            | PatchEditType::ModifyErrorMapping
            | PatchEditType::AddErrorVariant
            | PatchEditType::AddRoute
            | PatchEditType::AddServiceMethod
            | PatchEditType::AddRepositoryMethod
            | PatchEditType::AddConfig
    )
}

fn looks_like_test_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with("tests/")
        || lower.contains("/tests/")
        || lower.contains("/test/")
        || lower.ends_with("_test.rs")
        || lower.ends_with("_tests.rs")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".spec.ts")
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
