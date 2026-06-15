use serde::Serialize;

use super::symbol_index_patch_verification_run_types::PatchVerificationExecutionStatus;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolPatchReviewResponse {
    pub(crate) task: String,
    pub(crate) decision: PatchReviewDecision,
    pub(crate) summary: String,
    pub(crate) plan_compliance: PatchReviewPlanCompliance,
    pub(crate) scope: PatchReviewScope,
    pub(crate) test_adequacy: PatchReviewTestAdequacy,
    pub(crate) verification: PatchReviewVerificationSummary,
    pub(crate) affected_symbols: Vec<String>,
    pub(crate) findings: Vec<PatchReviewFinding>,
    pub(crate) next_steps: Vec<String>,
    pub(crate) review_report_markdown: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchReviewDecision {
    Approve,
    ApproveWithNotes,
    NeedsHumanReview,
    Reject,
}

impl PatchReviewDecision {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            PatchReviewDecision::Approve => "approve",
            PatchReviewDecision::ApproveWithNotes => "approve_with_notes",
            PatchReviewDecision::NeedsHumanReview => "needs_human_review",
            PatchReviewDecision::Reject => "reject",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub(crate) enum PatchReviewSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl PatchReviewSeverity {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            PatchReviewSeverity::Info => "info",
            PatchReviewSeverity::Low => "low",
            PatchReviewSeverity::Medium => "medium",
            PatchReviewSeverity::High => "high",
            PatchReviewSeverity::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchReviewCategory {
    PlanCompliance,
    ScopeControl,
    DiffSize,
    TestAdequacy,
    Verification,
    Safety,
    RegressionRisk,
}

impl PatchReviewCategory {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            PatchReviewCategory::PlanCompliance => "plan_compliance",
            PatchReviewCategory::ScopeControl => "scope_control",
            PatchReviewCategory::DiffSize => "diff_size",
            PatchReviewCategory::TestAdequacy => "test_adequacy",
            PatchReviewCategory::Verification => "verification",
            PatchReviewCategory::Safety => "safety",
            PatchReviewCategory::RegressionRisk => "regression_risk",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchReviewFinding {
    pub(crate) severity: PatchReviewSeverity,
    pub(crate) category: PatchReviewCategory,
    pub(crate) code: String,
    pub(crate) file_path: Option<String>,
    pub(crate) message: String,
    pub(crate) evidence: Vec<String>,
    pub(crate) recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchReviewPlanCompliance {
    pub(crate) required_files: Vec<String>,
    pub(crate) required_files_touched: Vec<String>,
    pub(crate) required_files_missing: Vec<String>,
    pub(crate) unexpected_files_touched: Vec<String>,
    pub(crate) forbidden_files_touched: Vec<String>,
    pub(crate) must_edit_coverage: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchReviewScope {
    pub(crate) touched_files: Vec<String>,
    pub(crate) touched_file_count: usize,
    pub(crate) added_lines: usize,
    pub(crate) removed_lines: usize,
    pub(crate) hunk_count: usize,
    pub(crate) added_files: Vec<String>,
    pub(crate) deleted_files: Vec<String>,
    pub(crate) test_files_touched: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchReviewTestAdequacy {
    pub(crate) expected: bool,
    pub(crate) status: String,
    pub(crate) required_test_files: Vec<String>,
    pub(crate) touched_test_files: Vec<String>,
    pub(crate) missing_test_files: Vec<String>,
    pub(crate) required_commands: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchReviewVerificationSummary {
    pub(crate) provided: bool,
    pub(crate) status: Option<PatchVerificationExecutionStatus>,
    pub(crate) success: bool,
    pub(crate) executed_command_count: usize,
    pub(crate) failed_commands: Vec<String>,
    pub(crate) skipped_required_commands: Vec<String>,
    pub(crate) blocked_reasons: Vec<String>,
}

pub(crate) fn finding(
    severity: PatchReviewSeverity,
    category: PatchReviewCategory,
    code: &str,
    file_path: Option<String>,
    message: impl Into<String>,
    evidence: Vec<String>,
    recommendation: &str,
) -> PatchReviewFinding {
    PatchReviewFinding {
        severity,
        category,
        code: code.to_string(),
        file_path,
        message: message.into(),
        evidence,
        recommendation: recommendation.to_string(),
    }
}
