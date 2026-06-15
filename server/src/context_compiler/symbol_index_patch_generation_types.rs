use serde::Serialize;

use super::symbol_index_patch_plan_types::PatchEditType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchGenerationMode {
    GenerateDiff,
    InspectOnly,
    NoPatch,
}

impl PatchGenerationMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            PatchGenerationMode::GenerateDiff => "generate_diff",
            PatchGenerationMode::InspectOnly => "inspect_only",
            PatchGenerationMode::NoPatch => "no_patch",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolPatchGeneration {
    pub(crate) task: String,
    pub(crate) mode: PatchGenerationMode,
    pub(crate) ready_to_generate: bool,
    pub(crate) edit_sequence: Vec<PatchGenerationStep>,
    pub(crate) diff_contract: PatchDiffContract,
    pub(crate) apply_readiness: PatchApplyReadiness,
    pub(crate) prompt: String,
    pub(crate) blocked_reasons: Vec<String>,
    pub(crate) trace: Vec<PatchGenerationTrace>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchGenerationStep {
    pub(crate) order: usize,
    pub(crate) file_path: String,
    pub(crate) symbol_id: Option<String>,
    pub(crate) qualified_name: Option<String>,
    pub(crate) start_line: Option<usize>,
    pub(crate) end_line: Option<usize>,
    pub(crate) edit_type: PatchEditType,
    pub(crate) action: String,
    pub(crate) constraints: Vec<String>,
    pub(crate) evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchDiffContract {
    pub(crate) output_format: String,
    pub(crate) apply_strategy: String,
    pub(crate) allowed_files: Vec<String>,
    pub(crate) inspect_only_files: Vec<String>,
    pub(crate) forbidden_patterns: Vec<String>,
    pub(crate) required_tests: Vec<String>,
    pub(crate) verification_commands: Vec<String>,
    pub(crate) safety_checks: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchApplyReadinessLevel {
    ReadyAfterDiff,
    NeedsInspection,
    NotApplicable,
}

impl PatchApplyReadinessLevel {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            PatchApplyReadinessLevel::ReadyAfterDiff => "ready_after_diff",
            PatchApplyReadinessLevel::NeedsInspection => "needs_inspection",
            PatchApplyReadinessLevel::NotApplicable => "not_applicable",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchApplyReadiness {
    pub(crate) level: PatchApplyReadinessLevel,
    pub(crate) apply_check_status: String,
    pub(crate) can_run_apply_check: bool,
    pub(crate) requires_generated_diff: bool,
    pub(crate) source_requirements: Vec<String>,
    pub(crate) pre_apply_checks: Vec<String>,
    pub(crate) post_apply_checks: Vec<String>,
    pub(crate) rollback_strategy: String,
    pub(crate) risk_level: String,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchGenerationTrace {
    pub(crate) source_kind: String,
    pub(crate) file_path: String,
    pub(crate) symbol_id: Option<String>,
    pub(crate) qualified_name: Option<String>,
    pub(crate) edit_type: PatchEditType,
    pub(crate) generation_decision: String,
    pub(crate) reason: String,
}
