use std::collections::BTreeSet;

use super::{
    symbol_index_compression_types::SymbolCompressedContext,
    symbol_index_patch_generation_types::{
        PatchApplyReadiness, PatchApplyReadinessLevel, PatchDiffContract, PatchGenerationMode,
        PatchGenerationStep, PatchGenerationTrace, SymbolPatchGeneration,
    },
    symbol_index_patch_plan_types::{
        PatchEditTarget, PatchEditType, ProposedPatchChange, SymbolPatchPlan,
    },
};

const MAX_GENERATION_STEPS: usize = 8;

pub(crate) fn build_symbol_patch_generation(
    task: &str,
    patch_plan: &SymbolPatchPlan,
    compressed: &SymbolCompressedContext,
) -> SymbolPatchGeneration {
    let mode = generation_mode(patch_plan);
    let blocked_reasons = blocked_reasons(patch_plan, mode);
    let ready_to_generate = mode == PatchGenerationMode::GenerateDiff && blocked_reasons.is_empty();
    let edit_sequence = patch_plan
        .must_edit
        .iter()
        .take(MAX_GENERATION_STEPS)
        .enumerate()
        .map(|(index, target)| generation_step(index + 1, target, patch_plan, compressed))
        .collect::<Vec<_>>();
    let diff_contract = diff_contract(patch_plan, ready_to_generate);
    let apply_readiness = apply_readiness(patch_plan, &diff_contract, mode, ready_to_generate);
    let prompt = generation_prompt(task, patch_plan, &diff_contract, mode, ready_to_generate);
    let trace = generation_trace(patch_plan, mode);

    SymbolPatchGeneration {
        task: task.to_string(),
        mode,
        ready_to_generate,
        edit_sequence,
        diff_contract,
        apply_readiness,
        prompt,
        blocked_reasons,
        trace,
    }
}


#[path = "symbol_index_patch_generation_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
