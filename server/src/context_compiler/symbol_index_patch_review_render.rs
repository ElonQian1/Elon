use super::symbol_index_patch_review_types::{
    PatchReviewDecision, PatchReviewFinding, PatchReviewPlanCompliance, PatchReviewScope,
    PatchReviewTestAdequacy, PatchReviewVerificationSummary,
};

pub(crate) fn render_review_report(
    decision: PatchReviewDecision,
    summary: &str,
    compliance: &PatchReviewPlanCompliance,
    scope: &PatchReviewScope,
    test: &PatchReviewTestAdequacy,
    verification: &PatchReviewVerificationSummary,
    affected_symbols: &[String],
    findings: &[PatchReviewFinding],
    next_steps: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("<patch_review>\n\n");
    out.push_str("# Decision\n");
    out.push_str(decision.as_str());
    out.push_str("\n\n# Summary\n");
    out.push_str(summary);
    out.push_str("\n\n# Plan Compliance\n");
    out.push_str(&format!(
        "- must_edit_coverage: {:.2}\n- required_files_touched: {}\n- required_files_missing: {}\n- unexpected_files_touched: {}\n",
        compliance.must_edit_coverage,
        list_or_none(&compliance.required_files_touched),
        list_or_none(&compliance.required_files_missing),
        list_or_none(&compliance.unexpected_files_touched)
    ));
    out.push_str("\n# Scope\n");
    out.push_str(&format!(
        "- touched_files: {}\n- changed_lines: +{} / -{}\n- hunk_count: {}\n",
        list_or_none(&scope.touched_files),
        scope.added_lines,
        scope.removed_lines,
        scope.hunk_count
    ));
    out.push_str("\n# Test Adequacy\n");
    out.push_str(&format!(
        "- status: {}\n- touched_test_files: {}\n- required_commands: {}\n",
        test.status,
        list_or_none(&test.touched_test_files),
        list_or_none(&test.required_commands)
    ));
    out.push_str("\n# Verification\n");
    out.push_str(&format!(
        "- provided: {}\n- status: {}\n- failed_commands: {}\n",
        verification.provided,
        verification
            .status
            .map(|status| format!("{status:?}"))
            .unwrap_or_else(|| "not_provided".to_string()),
        list_or_none(&verification.failed_commands)
    ));
    out.push_str("\n# Affected Symbols\n");
    if affected_symbols.is_empty() {
        out.push_str("- none\n");
    } else {
        for symbol in affected_symbols {
            out.push_str(&format!("- `{symbol}`\n"));
        }
    }
    out.push_str("\n# Findings\n");
    if findings.is_empty() {
        out.push_str("- none\n");
    } else {
        for finding in findings {
            out.push_str(&format!(
                "- {} / {} / {}: {}{}\n  recommendation: {}\n",
                finding.severity.as_str(),
                finding.category.as_str(),
                finding.code,
                finding.message,
                finding
                    .file_path
                    .as_deref()
                    .map(|file| format!(" ({file})"))
                    .unwrap_or_default(),
                finding.recommendation
            ));
        }
    }
    out.push_str("\n# Next Steps\n");
    for step in next_steps {
        out.push_str(&format!("- {step}\n"));
    }
    out.push_str("\n</patch_review>\n");
    out
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}
