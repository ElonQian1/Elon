use super::*;

pub(super) fn generation_mode(plan: &SymbolPatchPlan) -> PatchGenerationMode {
    if !plan.patch_required {
        PatchGenerationMode::NoPatch
    } else if plan.must_edit.is_empty()
        || plan
            .must_edit
            .iter()
            .all(|target| target.edit_type == PatchEditType::InspectOnly)
    {
        PatchGenerationMode::InspectOnly
    } else {
        PatchGenerationMode::GenerateDiff
    }
}

pub(super) fn blocked_reasons(plan: &SymbolPatchPlan, mode: PatchGenerationMode) -> Vec<String> {
    let mut reasons = Vec::new();
    match mode {
        PatchGenerationMode::NoPatch => {
            reasons.push("patch_plan_says_context_only".to_string());
        }
        PatchGenerationMode::InspectOnly => {
            reasons.push("no_required_edit_target_ready_for_diff_generation".to_string());
        }
        PatchGenerationMode::GenerateDiff => {}
    }
    if plan.must_edit.len() > MAX_GENERATION_STEPS {
        reasons.push(format!(
            "generation_step_cap={} required_targets={}",
            MAX_GENERATION_STEPS,
            plan.must_edit.len()
        ));
    }
    reasons
}

pub(super) fn generation_step(
    order: usize,
    target: &PatchEditTarget,
    plan: &SymbolPatchPlan,
    compressed: &SymbolCompressedContext,
) -> PatchGenerationStep {
    PatchGenerationStep {
        order,
        file_path: target.file_path.clone(),
        symbol_id: target.symbol_id.clone(),
        qualified_name: target.qualified_name.clone(),
        start_line: target.start_line,
        end_line: target.end_line,
        edit_type: target.edit_type,
        action: action_for_target(target, plan),
        constraints: constraints_for_target(target, plan),
        evidence: evidence_for_target(target, compressed),
    }
}

pub(super) fn action_for_target(target: &PatchEditTarget, plan: &SymbolPatchPlan) -> String {
    let symbol = target
        .qualified_name
        .as_deref()
        .unwrap_or("selected target");
    match target.edit_type {
        PatchEditType::ModifyBehavior => {
            format!("Change `{symbol}` only where it owns the requested behavior.")
        }
        PatchEditType::ModifyErrorMapping => {
            format!("Adjust the error/status mapping in `{symbol}` to satisfy the requested status behavior.")
        }
        PatchEditType::AddErrorVariant => {
            format!("Add the minimal error variant or mapping branch required by `{symbol}`.")
        }
        PatchEditType::UpdateTest => {
            format!("Update `{symbol}` so the regression assertion matches the requested behavior.")
        }
        PatchEditType::AddTest => {
            format!("Add a focused regression test near `{symbol}` for the requested behavior.")
        }
        PatchEditType::RenameSymbol => {
            format!("Rename `{symbol}` and preserve behavior across selected references.")
        }
        PatchEditType::UpdateReferences => {
            format!("Update references to `{symbol}` consistently with the refactor.")
        }
        PatchEditType::AddRoute => {
            format!(
                "Wire the requested feature through the route or handler pattern near `{symbol}`."
            )
        }
        PatchEditType::AddServiceMethod => {
            format!("Add the minimal service behavior near `{symbol}` following existing style.")
        }
        PatchEditType::AddRepositoryMethod => {
            format!("Add the minimal repository/storage behavior near `{symbol}` following existing style.")
        }
        PatchEditType::AddConfig => {
            format!(
                "Add only the configuration needed near `{symbol}` for: {}",
                plan.task
            )
        }
        PatchEditType::InspectOnly => {
            format!("Inspect `{symbol}` first; do not edit unless it proves ownership.")
        }
    }
}

pub(super) fn constraints_for_target(
    target: &PatchEditTarget,
    plan: &SymbolPatchPlan,
) -> Vec<String> {
    let mut constraints = Vec::new();
    constraints.push("Do not edit files outside the diff contract allowed_files list.".to_string());
    constraints.push("Preserve unrelated public behavior and formatting style.".to_string());
    constraints.push(format!(
        "Keep the edit scoped to {}:{}.",
        target.file_path,
        target.start_line.unwrap_or_default()
    ));
    constraints.extend(
        plan.proposed_changes
            .iter()
            .filter(|change| same_change_target(change, target))
            .flat_map(|change| change.constraints.iter().take(2).cloned()),
    );
    if matches!(
        target.edit_type,
        PatchEditType::UpdateTest | PatchEditType::AddTest
    ) {
        constraints
            .push("Keep the test narrow enough to fail before the behavior edit.".to_string());
    }
    dedupe(constraints)
}

pub(super) fn same_change_target(change: &ProposedPatchChange, target: &PatchEditTarget) -> bool {
    change.target_file_path == target.file_path
        && (change.target_symbol.as_deref() == target.qualified_name.as_deref()
            || change.target_symbol.is_none()
            || target.qualified_name.is_none())
}

pub(super) fn evidence_for_target(
    target: &PatchEditTarget,
    compressed: &SymbolCompressedContext,
) -> Vec<String> {
    let mut evidence = compressed
        .blocks
        .iter()
        .filter(|block| {
            block.file_path == target.file_path
                || (target.symbol_id.is_some()
                    && block.symbol_id.as_deref() == target.symbol_id.as_deref())
        })
        .take(3)
        .map(|block| {
            format!(
                "rank={} level={} sources={} reasons={}",
                block.rank,
                block.level.as_str(),
                block.sources.join("+"),
                block.reasons.join("; ")
            )
        })
        .collect::<Vec<_>>();
    if evidence.is_empty() {
        evidence.push(format!(
            "patch_plan_target sourceRank={} compression={}",
            target.source_rank,
            target.compression_level.as_str()
        ));
    }
    evidence
}

pub(super) fn diff_contract(plan: &SymbolPatchPlan, ready_to_generate: bool) -> PatchDiffContract {
    let allowed_files = if ready_to_generate {
        file_set(
            plan.must_edit
                .iter()
                .filter(|target| target.edit_type != PatchEditType::InspectOnly),
        )
    } else {
        Vec::new()
    };
    let inspect_only_files = file_set(
        plan.should_inspect
            .iter()
            .chain(plan.maybe_edit.iter())
            .chain(
                plan.must_edit
                    .iter()
                    .filter(|target| target.edit_type == PatchEditType::InspectOnly),
            ),
    );
    let mut required_tests = plan.test_plan.commands.clone();
    if required_tests.is_empty() && ready_to_generate {
        required_tests.push("cargo test".to_string());
    }
    let mut verification_commands = required_tests.clone();
    if ready_to_generate {
        verification_commands.push("git diff --check".to_string());
    }
    PatchDiffContract {
        output_format: "unified_diff_only".to_string(),
        apply_strategy: "generate_diff_then_apply_patch_after_human_or_agent_review".to_string(),
        allowed_files,
        inspect_only_files,
        forbidden_patterns: vec![
            "Do not include prose outside the unified diff.".to_string(),
            "Do not modify lockfiles, generated files, or unrelated formatting-only blocks unless they are explicitly allowed.".to_string(),
            "Do not edit inspect_only_files without re-running retrieval or adding explicit evidence.".to_string(),
        ],
        required_tests,
        verification_commands,
        safety_checks: vec![
            "Read every allowed file before generating the diff.".to_string(),
            "Keep the patch minimal and reversible.".to_string(),
            "Include regression tests when behavior or status mapping changes.".to_string(),
            "Stop and return blocked reasons if required source context is missing.".to_string(),
        ],
    }
}

pub(super) fn apply_readiness(
    plan: &SymbolPatchPlan,
    contract: &PatchDiffContract,
    mode: PatchGenerationMode,
    ready_to_generate: bool,
) -> PatchApplyReadiness {
    let level = if ready_to_generate {
        PatchApplyReadinessLevel::ReadyAfterDiff
    } else if mode == PatchGenerationMode::InspectOnly {
        PatchApplyReadinessLevel::NeedsInspection
    } else {
        PatchApplyReadinessLevel::NotApplicable
    };
    let can_run_apply_check = ready_to_generate && !contract.allowed_files.is_empty();
    let mut source_requirements = plan
        .must_edit
        .iter()
        .take(MAX_GENERATION_STEPS)
        .map(source_requirement)
        .collect::<Vec<_>>();
    source_requirements.extend(plan.should_inspect.iter().take(4).map(|target| {
        format!(
            "Inspect before broadening patch: {}",
            source_requirement(target)
        )
    }));
    let pre_apply_checks = if can_run_apply_check {
        vec![
            "git status --short".to_string(),
            "git apply --check <generated.patch>".to_string(),
            "confirm generated diff touches only allowed_files".to_string(),
        ]
    } else {
        vec!["do not run git apply until a unified diff exists".to_string()]
    };
    let mut post_apply_checks = contract.verification_commands.clone();
    if can_run_apply_check
        && !post_apply_checks
            .iter()
            .any(|command| command == "git status --short")
    {
        post_apply_checks.push("git status --short".to_string());
    }
    PatchApplyReadiness {
        level,
        apply_check_status: apply_check_status(level).to_string(),
        can_run_apply_check,
        requires_generated_diff: can_run_apply_check,
        source_requirements: dedupe(source_requirements),
        pre_apply_checks,
        post_apply_checks: dedupe(post_apply_checks),
        rollback_strategy: rollback_strategy(level).to_string(),
        risk_level: risk_level(plan, contract).to_string(),
        notes: apply_notes(plan, level),
    }
}

pub(super) fn source_requirement(target: &PatchEditTarget) -> String {
    let symbol = target
        .qualified_name
        .as_deref()
        .unwrap_or("selected target");
    let line = target
        .start_line
        .map(|line| format!(":{line}"))
        .unwrap_or_default();
    format!("Read {}{} `{}`", target.file_path, line, symbol)
}

pub(super) fn apply_check_status(level: PatchApplyReadinessLevel) -> &'static str {
    match level {
        PatchApplyReadinessLevel::ReadyAfterDiff => "ready_to_check_after_diff_generation",
        PatchApplyReadinessLevel::NeedsInspection => "inspect_sources_before_generating_diff",
        PatchApplyReadinessLevel::NotApplicable => "no_patch_apply_check_required",
    }
}

pub(super) fn rollback_strategy(level: PatchApplyReadinessLevel) -> &'static str {
    match level {
        PatchApplyReadinessLevel::ReadyAfterDiff => {
            "If apply or validation fails, reverse the generated patch with git apply -R or discard only the allowed files."
        }
        PatchApplyReadinessLevel::NeedsInspection => {
            "No patch should be applied; rerun retrieval after inspection identifies an edit owner."
        }
        PatchApplyReadinessLevel::NotApplicable => {
            "No rollback needed because no code patch should be generated."
        }
    }
}

pub(super) fn risk_level(plan: &SymbolPatchPlan, contract: &PatchDiffContract) -> &'static str {
    if contract.allowed_files.len() > 4 || !plan.risk_notes.is_empty() {
        "high"
    } else if !contract.inspect_only_files.is_empty() || contract.required_tests.is_empty() {
        "medium"
    } else {
        "low"
    }
}

pub(super) fn apply_notes(plan: &SymbolPatchPlan, level: PatchApplyReadinessLevel) -> Vec<String> {
    let mut notes = Vec::new();
    match level {
        PatchApplyReadinessLevel::ReadyAfterDiff => {
            notes.push("Patch applicability is unknown until the concrete unified diff exists; run git apply --check before applying.".to_string());
        }
        PatchApplyReadinessLevel::NeedsInspection => {
            notes.push("The patch plan has no editable must_edit target yet; inspect selected files first.".to_string());
        }
        PatchApplyReadinessLevel::NotApplicable => {
            notes.push(
                "The task is context-only, so patch application is intentionally disabled."
                    .to_string(),
            );
        }
    }
    notes.extend(plan.risk_notes.iter().take(3).cloned());
    dedupe(notes)
}

pub(super) fn file_set<'a>(targets: impl Iterator<Item = &'a PatchEditTarget>) -> Vec<String> {
    targets
        .map(|target| target.file_path.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn generation_prompt(
    task: &str,
    plan: &SymbolPatchPlan,
    contract: &PatchDiffContract,
    mode: PatchGenerationMode,
    ready_to_generate: bool,
) -> String {
    if !ready_to_generate {
        return format!(
            "Do not generate a code diff for this task yet. Mode: {}. Explain or inspect using the selected context, then rerun task-pack if an edit becomes necessary. Task: {}",
            mode.as_str(),
            task
        );
    }

    let allowed = if contract.allowed_files.is_empty() {
        "- none".to_string()
    } else {
        contract.allowed_files.join("\n- ")
    };
    let tests = if contract.required_tests.is_empty() {
        "- no targeted test discovered; add one if behavior changes".to_string()
    } else {
        contract.required_tests.join("\n- ")
    };
    format!(
        "Generate a unified diff only. Task: {task}\nAllowed files:\n- {allowed}\nRequired tests:\n- {tests}\nPatch kind: {}\nRules:\n- Read each allowed file before editing.\n- Modify only allowed files.\n- Preserve unrelated behavior.\n- Include or update regression tests for observable behavior changes.\n- Before applying, verify the generated patch with git apply --check <generated.patch>.\n- Return blocked instead of guessing if exact source context is missing.",
        plan.plan_kind
    )
}

pub(super) fn generation_trace(
    plan: &SymbolPatchPlan,
    mode: PatchGenerationMode,
) -> Vec<PatchGenerationTrace> {
    let mut trace = plan
        .must_edit
        .iter()
        .map(|target| trace_entry("must_edit", target, mode, "allowed_for_generation"))
        .collect::<Vec<_>>();
    trace.extend(
        plan.should_inspect
            .iter()
            .take(6)
            .map(|target| trace_entry("should_inspect", target, mode, "inspect_before_edit")),
    );
    trace.extend(
        plan.maybe_edit
            .iter()
            .take(4)
            .map(|target| trace_entry("maybe_edit", target, mode, "not_allowed_by_default")),
    );
    trace
}

pub(super) fn trace_entry(
    source_kind: &str,
    target: &PatchEditTarget,
    mode: PatchGenerationMode,
    fallback_reason: &str,
) -> PatchGenerationTrace {
    let generation_decision = if mode == PatchGenerationMode::GenerateDiff
        && source_kind == "must_edit"
        && target.edit_type != PatchEditType::InspectOnly
    {
        "allow_edit"
    } else {
        "inspect_only"
    };
    PatchGenerationTrace {
        source_kind: source_kind.to_string(),
        file_path: target.file_path.clone(),
        symbol_id: target.symbol_id.clone(),
        qualified_name: target.qualified_name.clone(),
        edit_type: target.edit_type,
        generation_decision: generation_decision.to_string(),
        reason: if target.reason.is_empty() {
            fallback_reason.to_string()
        } else {
            target.reason.clone()
        },
    }
}

pub(super) fn dedupe(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
