use super::{
    symbol_index_patch_check::check_symbol_patch_diff,
    symbol_index_patch_generation_types::SymbolPatchGeneration,
    symbol_index_patch_plan_types::SymbolPatchPlan,
    symbol_index_patch_review_analysis::{
        affected_symbols, build_plan_compliance, build_scope, build_test_adequacy,
        build_verification_summary,
    },
    symbol_index_patch_review_findings::{build_findings, decide, next_steps, summary},
    symbol_index_patch_review_render::render_review_report,
    symbol_index_patch_review_types::SymbolPatchReviewResponse,
    symbol_index_patch_verification_run_types::SymbolPatchVerificationRunResponse,
};

pub(crate) fn build_symbol_patch_review(
    plan: &SymbolPatchPlan,
    generation: &SymbolPatchGeneration,
    generated_diff: &str,
    verification: Option<&SymbolPatchVerificationRunResponse>,
) -> SymbolPatchReviewResponse {
    let contract_check = check_symbol_patch_diff(generation, generated_diff);
    let scope = build_scope(&contract_check, generated_diff);
    let plan_compliance = build_plan_compliance(plan, generation, &scope, &contract_check);
    let test_adequacy = build_test_adequacy(plan, &scope);
    let verification = build_verification_summary(verification);
    let affected_symbols = affected_symbols(plan, &scope.touched_files);
    let findings = build_findings(
        &contract_check,
        &plan_compliance,
        &scope,
        &test_adequacy,
        &verification,
        generated_diff,
    );
    let decision = decide(&findings);
    let summary = summary(decision, &findings, &scope, &verification);
    let next_steps = next_steps(decision);
    let review_report_markdown = render_review_report(
        decision,
        &summary,
        &plan_compliance,
        &scope,
        &test_adequacy,
        &verification,
        &affected_symbols,
        &findings,
        &next_steps,
    );

    SymbolPatchReviewResponse {
        task: plan.task.clone(),
        decision,
        summary,
        plan_compliance,
        scope,
        test_adequacy,
        verification,
        affected_symbols,
        findings,
        next_steps,
        review_report_markdown,
    }
}
