use std::collections::BTreeSet;

use super::{
    symbol_index_compression_types::CompressionLevel,
    symbol_index_impact_types::{ImpactTestHint, SymbolImpactResponse},
    symbol_index_patch_plan_types::{
        PatchEditPriority, PatchEditTarget, PatchEditType, PatchPlanningDecision,
    },
    symbol_index_query_types::SymbolHit,
    symbol_index_ranker::{RankedContextItem, RerankDecision},
    symbol_index_retrieval_plan::{QueryIntent, RetrievalPlan},
};

pub(crate) const MAX_MUST_EDIT: usize = 6;
pub(crate) const MAX_SHOULD_INSPECT: usize = 6;
pub(crate) const MAX_MAYBE_EDIT: usize = 6;
pub(crate) const MAX_TRACE: usize = 24;

pub(crate) fn patch_required(plan: &RetrievalPlan) -> bool {
    matches!(
        plan.intent,
        QueryIntent::DebugError
            | QueryIntent::ModifyBehavior
            | QueryIntent::Refactor
            | QueryIntent::AddFeature
            | QueryIntent::Test
    ) || plan.features.mentions_modify
        || plan.features.mentions_add_feature
        || plan.features.mentions_refactor
}

pub(crate) fn classify_planning_decision(
    item: &RankedContextItem,
    plan: &RetrievalPlan,
    seed: &SymbolHit,
    compression_level: CompressionLevel,
    patch_required: bool,
    reasons: &mut Vec<String>,
) -> PatchPlanningDecision {
    if item.decision == RerankDecision::Drop || compression_level == CompressionLevel::Drop {
        reasons.push("dropped_by_reranker_or_compressor".to_string());
        return PatchPlanningDecision::Skip;
    }
    let primary = is_primary_target(item, seed);
    let error_context = is_error_context(item, plan);
    let test_context = item.is_test_context || looks_like_test(&item.file_path);
    if !patch_required {
        reasons.push("intent_context_only_no_patch_required".to_string());
        return if item.rank <= 5 {
            PatchPlanningDecision::ShouldInspect
        } else {
            PatchPlanningDecision::Skip
        };
    }

    if primary {
        reasons.push("chosen_seed_or_top_target".to_string());
    }
    if error_context {
        reasons.push("error_or_status_context".to_string());
    }
    if test_context {
        reasons.push("test_context_for_regression".to_string());
    }

    match plan.intent {
        QueryIntent::Refactor if primary || has_graph_reference(item) => {
            PatchPlanningDecision::MustEdit
        }
        QueryIntent::AddFeature
            if primary || is_handler_or_service_context(item) || test_context =>
        {
            PatchPlanningDecision::MustEdit
        }
        QueryIntent::ModifyBehavior | QueryIntent::Test
            if primary || error_context || test_context =>
        {
            PatchPlanningDecision::MustEdit
        }
        QueryIntent::DebugError
            if plan.features.mentions_modify && (primary || error_context || test_context) =>
        {
            PatchPlanningDecision::MustEdit
        }
        QueryIntent::DebugError if primary || error_context || test_context => {
            PatchPlanningDecision::ShouldInspect
        }
        _ if item.rank <= 3 && item.decision == RerankDecision::MustInclude => {
            PatchPlanningDecision::ShouldInspect
        }
        _ if item.rank <= 10 && item.decision != RerankDecision::Drop => {
            PatchPlanningDecision::MaybeEdit
        }
        _ => PatchPlanningDecision::Skip,
    }
}

pub(crate) fn target_from_ranked(
    item: &RankedContextItem,
    plan: &RetrievalPlan,
    compression_level: CompressionLevel,
    decision: PatchPlanningDecision,
    reasons: Vec<String>,
) -> PatchEditTarget {
    PatchEditTarget {
        file_path: item.file_path.clone(),
        symbol_id: item.symbol_id.clone(),
        qualified_name: Some(item.label.clone()),
        start_line: item.start_line,
        end_line: item.end_line,
        edit_type: infer_edit_type(item, plan, decision),
        priority: priority_for_decision(decision),
        reason: compact_reason(reasons, item),
        source_rank: item.rank,
        source_decision: item.decision,
        compression_level,
        sources: item.sources.clone(),
    }
}

pub(crate) fn add_test_hints(
    must_edit: &mut Vec<PatchEditTarget>,
    should_inspect: &mut Vec<PatchEditTarget>,
    seen_targets: &mut BTreeSet<String>,
    impact: &SymbolImpactResponse,
    plan: &RetrievalPlan,
    patch_required: bool,
) {
    for hint in impact.test_hints.iter().take(4) {
        let target = target_from_test_hint(hint, plan, patch_required);
        if !seen_targets.insert(target_key(&target)) {
            continue;
        }
        if patch_required && should_test_be_must_edit(plan) && must_edit.len() < MAX_MUST_EDIT {
            must_edit.push(target);
        } else if should_inspect.len() < MAX_SHOULD_INSPECT {
            should_inspect.push(target);
        }
    }
}

pub(crate) fn ensure_seed_target(
    must_edit: &mut Vec<PatchEditTarget>,
    should_inspect: &mut Vec<PatchEditTarget>,
    seen_targets: &mut BTreeSet<String>,
    seed: &SymbolHit,
    plan: &RetrievalPlan,
    patch_required: bool,
) {
    if !patch_required || !must_edit.is_empty() {
        return;
    }
    let target = PatchEditTarget {
        file_path: seed.file_path.clone(),
        symbol_id: Some(seed.id.clone()),
        qualified_name: Some(seed.qualified_name.clone()),
        start_line: Some(seed.start_line),
        end_line: Some(seed.end_line),
        edit_type: match plan.intent {
            QueryIntent::Refactor => PatchEditType::RenameSymbol,
            QueryIntent::AddFeature => PatchEditType::AddServiceMethod,
            QueryIntent::DebugError if !plan.features.mentions_modify => PatchEditType::InspectOnly,
            _ => PatchEditType::ModifyBehavior,
        },
        priority: PatchEditPriority::Required,
        reason: "fallback_to_chosen_seed_when_ranked_targets_are_not_editable".to_string(),
        source_rank: 1,
        source_decision: RerankDecision::MustInclude,
        compression_level: CompressionLevel::FullSymbolBody,
        sources: vec!["chosen_seed".to_string()],
    };
    if !seen_targets.insert(target_key(&target)) {
        return;
    }
    if target.edit_type == PatchEditType::InspectOnly {
        should_inspect.push(target);
    } else {
        must_edit.push(target);
    }
}

pub(crate) fn target_key(target: &PatchEditTarget) -> String {
    if let Some(symbol_id) = target.symbol_id.as_deref() {
        format!("symbol:{symbol_id}")
    } else {
        format!(
            "file:{}:{}",
            target.file_path,
            target.start_line.unwrap_or_default()
        )
    }
}

pub(crate) fn plan_kind(plan: &RetrievalPlan, patch_required: bool) -> &'static str {
    if !patch_required {
        return "context_only";
    }
    match plan.intent {
        QueryIntent::DebugError if plan.features.mentions_modify => "debug_fix",
        QueryIntent::DebugError => "diagnostic_then_patch",
        QueryIntent::ModifyBehavior => "behavior_change",
        QueryIntent::Refactor => "refactor",
        QueryIntent::AddFeature => "feature_addition",
        QueryIntent::Test => "test_update",
        QueryIntent::Unknown => "inspect_then_patch",
        _ => "patch_plan",
    }
}

fn target_from_test_hint(
    hint: &ImpactTestHint,
    plan: &RetrievalPlan,
    patch_required: bool,
) -> PatchEditTarget {
    PatchEditTarget {
        file_path: hint.path.clone(),
        symbol_id: Some(hint.symbol_id.clone()),
        qualified_name: Some(hint.symbol_name.clone()),
        start_line: Some(hint.line),
        end_line: None,
        edit_type: if matches!(plan.intent, QueryIntent::AddFeature) {
            PatchEditType::AddTest
        } else if patch_required {
            PatchEditType::UpdateTest
        } else {
            PatchEditType::InspectOnly
        },
        priority: if patch_required && should_test_be_must_edit(plan) {
            PatchEditPriority::Required
        } else {
            PatchEditPriority::High
        },
        reason: format!(
            "impact_test_hint edge_kind={} reason={}",
            hint.edge_kind.as_deref().unwrap_or("unknown"),
            hint.reason
        ),
        source_rank: 0,
        source_decision: RerankDecision::Include,
        compression_level: CompressionLevel::FocusedSnippet,
        sources: vec!["graph_test_hint".to_string()],
    }
}

fn infer_edit_type(
    item: &RankedContextItem,
    plan: &RetrievalPlan,
    decision: PatchPlanningDecision,
) -> PatchEditType {
    if decision == PatchPlanningDecision::ShouldInspect
        && matches!(plan.intent, QueryIntent::DebugError)
        && !plan.features.mentions_modify
    {
        return PatchEditType::InspectOnly;
    }
    if item.is_test_context || looks_like_test(&item.file_path) {
        return if matches!(plan.intent, QueryIntent::AddFeature) {
            PatchEditType::AddTest
        } else {
            PatchEditType::UpdateTest
        };
    }
    if is_error_context(item, plan) {
        return if matches!(plan.intent, QueryIntent::AddFeature) {
            PatchEditType::AddErrorVariant
        } else {
            PatchEditType::ModifyErrorMapping
        };
    }
    match plan.intent {
        QueryIntent::Refactor if item.rank <= 3 => PatchEditType::RenameSymbol,
        QueryIntent::Refactor => PatchEditType::UpdateReferences,
        QueryIntent::AddFeature if is_handler_context(item) => PatchEditType::AddRoute,
        QueryIntent::AddFeature if is_repository_context(item) => {
            PatchEditType::AddRepositoryMethod
        }
        QueryIntent::AddFeature if is_config_context(item) => PatchEditType::AddConfig,
        QueryIntent::AddFeature => PatchEditType::AddServiceMethod,
        QueryIntent::DebugError if !plan.features.mentions_modify => PatchEditType::InspectOnly,
        QueryIntent::ModifyBehavior | QueryIntent::DebugError | QueryIntent::Test => {
            PatchEditType::ModifyBehavior
        }
        _ => PatchEditType::InspectOnly,
    }
}

fn should_test_be_must_edit(plan: &RetrievalPlan) -> bool {
    matches!(
        plan.intent,
        QueryIntent::ModifyBehavior
            | QueryIntent::Refactor
            | QueryIntent::AddFeature
            | QueryIntent::Test
    ) || plan.features.mentions_modify
}

fn priority_for_decision(decision: PatchPlanningDecision) -> PatchEditPriority {
    match decision {
        PatchPlanningDecision::MustEdit => PatchEditPriority::Required,
        PatchPlanningDecision::ShouldInspect => PatchEditPriority::High,
        PatchPlanningDecision::MaybeEdit => PatchEditPriority::Medium,
        PatchPlanningDecision::Skip => PatchEditPriority::Low,
    }
}

fn is_primary_target(item: &RankedContextItem, seed: &SymbolHit) -> bool {
    item.symbol_id.as_deref() == Some(seed.id.as_str())
        || item.label.contains(&seed.name)
        || (item.rank <= 3 && item.file_path == seed.file_path)
}

fn is_error_context(item: &RankedContextItem, plan: &RetrievalPlan) -> bool {
    if plan.features.status_codes.is_empty() && plan.features.error_like_terms.is_empty() {
        return false;
    }
    contains_any(
        &lower_item_text(item),
        &[
            "error",
            "err",
            "status",
            "response",
            "unauthorized",
            "forbidden",
            "internal",
            "panic",
            "exception",
            "app_error",
            "intoresponse",
            "500",
            "401",
            "403",
        ],
    )
}

fn has_graph_reference(item: &RankedContextItem) -> bool {
    item.sources
        .iter()
        .any(|source| source.starts_with("graph_") || source == "graph")
        || item
            .reasons
            .iter()
            .any(|reason| reason.contains("reference") || reason.contains("impacted"))
}

fn is_handler_or_service_context(item: &RankedContextItem) -> bool {
    contains_any(
        &lower_item_text(item),
        &[
            "handler",
            "route",
            "api",
            "controller",
            "endpoint",
            "service",
        ],
    )
}

fn is_handler_context(item: &RankedContextItem) -> bool {
    contains_any(
        &lower_item_text(item),
        &["handler", "route", "api", "controller", "endpoint"],
    )
}

fn is_repository_context(item: &RankedContextItem) -> bool {
    contains_any(
        &lower_item_text(item),
        &["repo", "repository", "store", "db", "sql", "dao"],
    )
}

fn is_config_context(item: &RankedContextItem) -> bool {
    contains_any(&lower_item_text(item), &["config", "env", "setting"])
}

fn looks_like_test(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with("_tests.rs")
        || path.contains("tests.rs")
}

fn lower_item_text(item: &RankedContextItem) -> String {
    format!(
        "{} {} {} {} {}",
        item.label,
        item.file_path,
        item.matched_terms.join(" "),
        item.sources.join(" "),
        item.reasons.join(" ")
    )
    .to_ascii_lowercase()
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn compact_reason(reasons: Vec<String>, item: &RankedContextItem) -> String {
    let mut values = reasons
        .into_iter()
        .chain(item.reasons.iter().take(3).cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if values.is_empty() {
        values.push(format!("ranked_context_rank={}", item.rank));
    }
    values.join("; ")
}
