use std::collections::{BTreeMap, BTreeSet};

use super::{
    symbol_index_compression_types::{CompressionLevel, SymbolCompressedContext},
    symbol_index_impact_types::SymbolImpactResponse,
    symbol_index_patch_plan_guidance::{
        build_proposed_changes, build_test_plan, open_questions, risk_notes,
    },
    symbol_index_patch_plan_rules::{
        add_test_hints, classify_planning_decision, ensure_seed_target, patch_required, plan_kind,
        target_from_ranked, target_key, MAX_MAYBE_EDIT, MAX_MUST_EDIT, MAX_SHOULD_INSPECT,
        MAX_TRACE,
    },
    symbol_index_patch_plan_types::{
        PatchEditTarget, PatchPlanSummary, PatchPlanningDecision, PatchPlanningTrace,
        SymbolPatchPlan,
    },
    symbol_index_query_types::SymbolHit,
    symbol_index_ranker::RankedContextItem,
    symbol_index_retrieval_plan::RetrievalPlan,
};

pub(crate) fn build_symbol_patch_plan(
    task: &str,
    plan: &RetrievalPlan,
    ranked: &[RankedContextItem],
    compressed: &SymbolCompressedContext,
    impact: &SymbolImpactResponse,
    chosen_seed: &SymbolHit,
) -> SymbolPatchPlan {
    let patch_required = patch_required(plan);
    let compression_by_id = compressed
        .blocks
        .iter()
        .map(|block| (block.id.as_str(), block.level))
        .collect::<BTreeMap<_, _>>();
    let mut must_edit = Vec::<PatchEditTarget>::new();
    let mut should_inspect = Vec::<PatchEditTarget>::new();
    let mut maybe_edit = Vec::<PatchEditTarget>::new();
    let mut trace = Vec::<PatchPlanningTrace>::new();
    let mut seen_targets = BTreeSet::<String>::new();

    for item in ranked.iter().take(MAX_TRACE) {
        let compression_level = compression_by_id
            .get(item.id.as_str())
            .copied()
            .unwrap_or(CompressionLevel::RelationOnly);
        let mut reasons = Vec::new();
        let decision = classify_planning_decision(
            item,
            plan,
            chosen_seed,
            compression_level,
            patch_required,
            &mut reasons,
        );
        trace.push(PatchPlanningTrace {
            rank: item.rank,
            file_path: item.file_path.clone(),
            label: item.label.clone(),
            decision,
            reasons: reasons.clone(),
        });
        if decision == PatchPlanningDecision::Skip {
            continue;
        }

        let target = target_from_ranked(item, plan, compression_level, decision, reasons);
        if !seen_targets.insert(target_key(&target)) {
            continue;
        }
        push_target(
            target,
            decision,
            &mut must_edit,
            &mut should_inspect,
            &mut maybe_edit,
        );
    }

    add_test_hints(
        &mut must_edit,
        &mut should_inspect,
        &mut seen_targets,
        impact,
        plan,
        patch_required,
    );
    ensure_seed_target(
        &mut must_edit,
        &mut should_inspect,
        &mut seen_targets,
        chosen_seed,
        plan,
        patch_required,
    );

    let proposed_changes = build_proposed_changes(&must_edit, &should_inspect, plan, task);
    let test_plan = build_test_plan(&must_edit, &should_inspect, &maybe_edit, task);
    let risk_notes = risk_notes(plan);
    let open_questions = open_questions(plan, &must_edit, &should_inspect, &test_plan);
    let summary = PatchPlanSummary {
        must_edit_count: must_edit.len(),
        should_inspect_count: should_inspect.len(),
        maybe_edit_count: maybe_edit.len(),
        test_target_count: test_plan.target_tests.len(),
        risk_count: risk_notes.len(),
    };

    SymbolPatchPlan {
        task: task.to_string(),
        intent: plan.intent,
        plan_kind: plan_kind(plan, patch_required).to_string(),
        patch_required,
        summary,
        must_edit,
        should_inspect,
        maybe_edit,
        proposed_changes,
        test_plan,
        risk_notes,
        open_questions,
        trace,
    }
}

fn push_target(
    target: PatchEditTarget,
    decision: PatchPlanningDecision,
    must_edit: &mut Vec<PatchEditTarget>,
    should_inspect: &mut Vec<PatchEditTarget>,
    maybe_edit: &mut Vec<PatchEditTarget>,
) {
    match decision {
        PatchPlanningDecision::MustEdit if must_edit.len() < MAX_MUST_EDIT => {
            must_edit.push(target)
        }
        PatchPlanningDecision::ShouldInspect if should_inspect.len() < MAX_SHOULD_INSPECT => {
            should_inspect.push(target)
        }
        PatchPlanningDecision::MaybeEdit if maybe_edit.len() < MAX_MAYBE_EDIT => {
            maybe_edit.push(target)
        }
        PatchPlanningDecision::MustEdit if should_inspect.len() < MAX_SHOULD_INSPECT => {
            should_inspect.push(target)
        }
        PatchPlanningDecision::ShouldInspect | PatchPlanningDecision::MaybeEdit
            if maybe_edit.len() < MAX_MAYBE_EDIT =>
        {
            maybe_edit.push(target)
        }
        _ => {}
    }
}
