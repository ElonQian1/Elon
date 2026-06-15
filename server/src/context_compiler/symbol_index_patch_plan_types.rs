use serde::Serialize;

use super::{
    symbol_index_compression_types::CompressionLevel, symbol_index_ranker::RerankDecision,
    symbol_index_retrieval_plan::QueryIntent,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SymbolPatchPlan {
    pub(crate) task: String,
    pub(crate) intent: QueryIntent,
    pub(crate) plan_kind: String,
    pub(crate) patch_required: bool,
    pub(crate) summary: PatchPlanSummary,
    pub(crate) must_edit: Vec<PatchEditTarget>,
    pub(crate) should_inspect: Vec<PatchEditTarget>,
    pub(crate) maybe_edit: Vec<PatchEditTarget>,
    pub(crate) proposed_changes: Vec<ProposedPatchChange>,
    pub(crate) test_plan: PatchTestPlan,
    pub(crate) risk_notes: Vec<String>,
    pub(crate) open_questions: Vec<String>,
    pub(crate) trace: Vec<PatchPlanningTrace>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchPlanSummary {
    pub(crate) must_edit_count: usize,
    pub(crate) should_inspect_count: usize,
    pub(crate) maybe_edit_count: usize,
    pub(crate) test_target_count: usize,
    pub(crate) risk_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchEditTarget {
    pub(crate) file_path: String,
    pub(crate) symbol_id: Option<String>,
    pub(crate) qualified_name: Option<String>,
    pub(crate) start_line: Option<usize>,
    pub(crate) end_line: Option<usize>,
    pub(crate) edit_type: PatchEditType,
    pub(crate) priority: PatchEditPriority,
    pub(crate) reason: String,
    pub(crate) source_rank: usize,
    pub(crate) source_decision: RerankDecision,
    pub(crate) compression_level: CompressionLevel,
    pub(crate) sources: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchEditType {
    ModifyBehavior,
    ModifyErrorMapping,
    AddErrorVariant,
    UpdateTest,
    AddTest,
    RenameSymbol,
    UpdateReferences,
    AddRoute,
    AddServiceMethod,
    AddRepositoryMethod,
    AddConfig,
    InspectOnly,
}

impl PatchEditType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            PatchEditType::ModifyBehavior => "modify_behavior",
            PatchEditType::ModifyErrorMapping => "modify_error_mapping",
            PatchEditType::AddErrorVariant => "add_error_variant",
            PatchEditType::UpdateTest => "update_test",
            PatchEditType::AddTest => "add_test",
            PatchEditType::RenameSymbol => "rename_symbol",
            PatchEditType::UpdateReferences => "update_references",
            PatchEditType::AddRoute => "add_route",
            PatchEditType::AddServiceMethod => "add_service_method",
            PatchEditType::AddRepositoryMethod => "add_repository_method",
            PatchEditType::AddConfig => "add_config",
            PatchEditType::InspectOnly => "inspect_only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchEditPriority {
    Required,
    High,
    Medium,
    Low,
}

impl PatchEditPriority {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            PatchEditPriority::Required => "required",
            PatchEditPriority::High => "high",
            PatchEditPriority::Medium => "medium",
            PatchEditPriority::Low => "low",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProposedPatchChange {
    pub(crate) target_file_path: String,
    pub(crate) target_symbol: Option<String>,
    pub(crate) edit_type: PatchEditType,
    pub(crate) current_behavior: Option<String>,
    pub(crate) desired_behavior: String,
    pub(crate) instructions: Vec<String>,
    pub(crate) constraints: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchTestPlan {
    pub(crate) commands: Vec<String>,
    pub(crate) target_tests: Vec<String>,
    pub(crate) expected_behavior: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchPlanningTrace {
    pub(crate) rank: usize,
    pub(crate) file_path: String,
    pub(crate) label: String,
    pub(crate) decision: PatchPlanningDecision,
    pub(crate) reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PatchPlanningDecision {
    MustEdit,
    ShouldInspect,
    MaybeEdit,
    Skip,
}

impl PatchPlanningDecision {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            PatchPlanningDecision::MustEdit => "must_edit",
            PatchPlanningDecision::ShouldInspect => "should_inspect",
            PatchPlanningDecision::MaybeEdit => "maybe_edit",
            PatchPlanningDecision::Skip => "skip",
        }
    }
}
