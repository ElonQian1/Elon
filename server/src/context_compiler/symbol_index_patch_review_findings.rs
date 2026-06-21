use super::{
    symbol_index_patch_check::SymbolPatchDiffCheck,
    symbol_index_patch_review_analysis::{diff_added_line, diff_removed_line},
    symbol_index_patch_review_types::{
        finding, PatchReviewCategory, PatchReviewDecision, PatchReviewFinding,
        PatchReviewPlanCompliance, PatchReviewScope, PatchReviewSeverity, PatchReviewTestAdequacy,
        PatchReviewVerificationSummary,
    },
    symbol_index_patch_verification_run_types::PatchVerificationExecutionStatus,
};

const LARGE_DIFF_FILE_THRESHOLD: usize = 4;
const MEDIUM_DIFF_LINE_THRESHOLD: usize = 150;
const LARGE_DIFF_LINE_THRESHOLD: usize = 400;
const LARGE_DIFF_HUNK_THRESHOLD: usize = 12;

pub(crate) fn build_findings(
    contract_check: &SymbolPatchDiffCheck,
    compliance: &PatchReviewPlanCompliance,
    scope: &PatchReviewScope,
    test: &PatchReviewTestAdequacy,
    verification: &PatchReviewVerificationSummary,
    generated_diff: &str,
) -> Vec<PatchReviewFinding> {
    let mut findings = Vec::new();
    findings.extend(contract_findings(contract_check));
    findings.extend(plan_compliance_findings(compliance));
    findings.extend(scope_findings(scope));
    findings.extend(test_adequacy_findings(test));
    findings.extend(safety_findings(generated_diff));
    findings.extend(verification_findings(verification));
    findings
}

pub(crate) fn decide(findings: &[PatchReviewFinding]) -> PatchReviewDecision {
    let max = findings.iter().map(|finding| finding.severity).max();
    match max {
        Some(PatchReviewSeverity::Critical) => PatchReviewDecision::Reject,
        Some(PatchReviewSeverity::High) => PatchReviewDecision::NeedsHumanReview,
        Some(_) => PatchReviewDecision::ApproveWithNotes,
        None => PatchReviewDecision::Approve,
    }
}

pub(crate) fn summary(
    decision: PatchReviewDecision,
    findings: &[PatchReviewFinding],
    scope: &PatchReviewScope,
    verification: &PatchReviewVerificationSummary,
) -> String {
    format!(
        "Patch review decision is {} with {} findings. It touches {} files, changes +{}/-{} lines, and verification status is {}.",
        decision.as_str(),
        findings.len(),
        scope.touched_file_count,
        scope.added_lines,
        scope.removed_lines,
        verification
            .status
            .map(|status| format!("{status:?}"))
            .unwrap_or_else(|| "not_provided".to_string())
    )
}

pub(crate) fn next_steps(decision: PatchReviewDecision) -> Vec<String> {
    match decision {
        PatchReviewDecision::Approve => vec![
            "Patch passed review; it may proceed to explicit apply approval.".to_string(),
            "Keep the review report with the patch run record.".to_string(),
        ],
        PatchReviewDecision::ApproveWithNotes => vec![
            "Patch is acceptable only after reading the review notes.".to_string(),
            "Prefer explicit user confirmation before applying.".to_string(),
        ],
        PatchReviewDecision::NeedsHumanReview => vec![
            "Do not auto-apply this patch.".to_string(),
            "Ask a human reviewer to inspect high-severity findings or generate a narrower repair patch.".to_string(),
        ],
        PatchReviewDecision::Reject => vec![
            "Reject this patch for automatic apply.".to_string(),
            "Return it to patch generation or repair with the review findings as constraints."
                .to_string(),
        ],
    }
}

fn contract_findings(contract_check: &SymbolPatchDiffCheck) -> Vec<PatchReviewFinding> {
    contract_check
        .violations
        .iter()
        .map(|violation| {
            finding(
                PatchReviewSeverity::Critical,
                PatchReviewCategory::ScopeControl,
                &format!("contract_{}", violation.code),
                violation.file_path.clone(),
                violation.message.clone(),
                vec![violation.code.clone()],
                "Reject this patch and regenerate a unified diff that satisfies the patch contract.",
            )
        })
        .collect()
}

fn plan_compliance_findings(compliance: &PatchReviewPlanCompliance) -> Vec<PatchReviewFinding> {
    let mut findings = Vec::new();
    for file in &compliance.required_files_missing {
        findings.push(finding(
            PatchReviewSeverity::High,
            PatchReviewCategory::PlanCompliance,
            "required_file_missing",
            Some(file.clone()),
            "Patch did not touch a required must_edit file.",
            vec![file.clone()],
            "Regenerate or repair the patch so every required edit target is addressed.",
        ));
    }
    for file in &compliance.unexpected_files_touched {
        findings.push(finding(
            PatchReviewSeverity::Medium,
            PatchReviewCategory::PlanCompliance,
            "unexpected_file_touched",
            Some(file.clone()),
            "Patch touched a file outside must_edit/maybe_edit targets.",
            vec![file.clone()],
            "Confirm the file is necessary or rerun retrieval to update the patch plan.",
        ));
    }
    findings
}

fn scope_findings(scope: &PatchReviewScope) -> Vec<PatchReviewFinding> {
    let mut findings = Vec::new();
    let changed_lines = scope.added_lines + scope.removed_lines;
    if scope.touched_file_count > LARGE_DIFF_FILE_THRESHOLD {
        findings.push(finding(
            PatchReviewSeverity::High,
            PatchReviewCategory::DiffSize,
            "many_files_touched",
            None,
            "Patch touches more files than expected for an automatic apply candidate.",
            vec![format!("touched_file_count={}", scope.touched_file_count)],
            "Split the patch or request human review before applying.",
        ));
    }
    if changed_lines > LARGE_DIFF_LINE_THRESHOLD {
        findings.push(finding(
            PatchReviewSeverity::High,
            PatchReviewCategory::DiffSize,
            "large_diff",
            None,
            "Patch changes a large number of lines.",
            vec![format!("changed_lines={changed_lines}")],
            "Review manually or split into smaller patches.",
        ));
    } else if changed_lines > MEDIUM_DIFF_LINE_THRESHOLD {
        findings.push(finding(
            PatchReviewSeverity::Medium,
            PatchReviewCategory::DiffSize,
            "medium_diff",
            None,
            "Patch is larger than the low-risk threshold.",
            vec![format!("changed_lines={changed_lines}")],
            "Prefer manual review before applying broadly.",
        ));
    }
    if scope.hunk_count > LARGE_DIFF_HUNK_THRESHOLD {
        findings.push(finding(
            PatchReviewSeverity::Medium,
            PatchReviewCategory::DiffSize,
            "many_hunks",
            None,
            "Patch has many hunks and may include unrelated edits.",
            vec![format!("hunk_count={}", scope.hunk_count)],
            "Check for unrelated formatting or refactor noise.",
        ));
    }
    for file in &scope.deleted_files {
        findings.push(finding(
            PatchReviewSeverity::High,
            PatchReviewCategory::ScopeControl,
            "file_deleted",
            Some(file.clone()),
            "Patch deletes a file.",
            vec![file.clone()],
            "Do not auto-apply deleted files without human confirmation.",
        ));
    }
    findings
}

fn test_adequacy_findings(test: &PatchReviewTestAdequacy) -> Vec<PatchReviewFinding> {
    let mut findings = Vec::new();
    if test.expected && test.touched_test_files.is_empty() {
        findings.push(finding(
            PatchReviewSeverity::Medium,
            PatchReviewCategory::TestAdequacy,
            "no_test_file_touched",
            None,
            "Patch appears to change behavior but does not touch a test file.",
            test.required_commands.clone(),
            "Add or update a focused regression test, or mark why no test is needed.",
        ));
    }
    for file in &test.missing_test_files {
        findings.push(finding(
            PatchReviewSeverity::High,
            PatchReviewCategory::TestAdequacy,
            "required_test_file_missing",
            Some(file.clone()),
            "Patch plan expected this test file to change, but the patch did not touch it.",
            vec![file.clone()],
            "Repair the patch so required tests cover the requested behavior.",
        ));
    }
    findings
}

fn safety_findings(generated_diff: &str) -> Vec<PatchReviewFinding> {
    let mut findings = Vec::new();
    for line in generated_diff.lines().filter_map(diff_added_line) {
        let lower = line.to_ascii_lowercase();
        if lower.contains("unsafe ") || lower.contains("unsafe{") || lower.contains("unsafe {") {
            findings.push(finding(
                PatchReviewSeverity::High,
                PatchReviewCategory::Safety,
                "unsafe_added",
                None,
                "Patch adds unsafe Rust code.",
                vec![line.to_string()],
                "Require human review before accepting unsafe code.",
            ));
        }
        if lower.contains(".unwrap()") || lower.contains(".expect(") || lower.contains("panic!(") {
            findings.push(finding(
                PatchReviewSeverity::Medium,
                PatchReviewCategory::Safety,
                "panic_path_added",
                None,
                "Patch adds unwrap/expect/panic paths.",
                vec![line.to_string()],
                "Prefer explicit error handling unless this is test-only code.",
            ));
        }
        if (lower.contains("println!") || lower.contains("dbg!") || lower.contains("tracing::"))
            && (lower.contains("password")
                || lower.contains("token")
                || lower.contains("secret")
                || lower.contains("api_key"))
        {
            findings.push(finding(
                PatchReviewSeverity::High,
                PatchReviewCategory::Safety,
                "sensitive_logging_added",
                None,
                "Patch appears to add logging for sensitive data.",
                vec![line.to_string()],
                "Remove sensitive logging before accepting the patch.",
            ));
        }
    }
    for line in generated_diff.lines().filter_map(diff_removed_line) {
        let lower = line.to_ascii_lowercase();
        if lower.contains("check_auth")
            || lower.contains("authorize")
            || lower.contains("permission")
            || lower.contains("csrf")
        {
            findings.push(finding(
                PatchReviewSeverity::High,
                PatchReviewCategory::RegressionRisk,
                "auth_check_removed",
                None,
                "Patch removes code that looks like an authorization or permission check.",
                vec![line.to_string()],
                "Require human review and verify auth behavior before accepting.",
            ));
        }
    }
    findings
}

fn verification_findings(verification: &PatchReviewVerificationSummary) -> Vec<PatchReviewFinding> {
    if !verification.provided {
        return vec![finding(
            PatchReviewSeverity::Medium,
            PatchReviewCategory::Verification,
            "verification_not_provided",
            None,
            "Patch review did not receive a verification report.",
            Vec::new(),
            "Run patch-verify-run before auto-applying this patch.",
        )];
    }
    match verification.status {
        Some(PatchVerificationExecutionStatus::Passed) => Vec::new(),
        Some(PatchVerificationExecutionStatus::ManualVerificationRequired) => vec![finding(
            PatchReviewSeverity::High,
            PatchReviewCategory::Verification,
            "manual_verification_required",
            None,
            "Automatic verification passed, but required manual checks were skipped.",
            verification.skipped_required_commands.clone(),
            "Run the skipped manual checks before applying.",
        )],
        Some(status) => vec![finding(
            PatchReviewSeverity::Critical,
            PatchReviewCategory::Verification,
            "verification_failed",
            None,
            format!("Patch verification did not pass: {status:?}."),
            verification
                .failed_commands
                .iter()
                .chain(verification.blocked_reasons.iter())
                .cloned()
                .collect(),
            "Do not apply; send this patch back through the repair loop.",
        )],
        None => Vec::new(),
    }
}
