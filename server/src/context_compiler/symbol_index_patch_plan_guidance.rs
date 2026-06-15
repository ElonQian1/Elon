use std::collections::BTreeSet;

use super::{
    symbol_index_patch_plan_rules::patch_required,
    symbol_index_patch_plan_types::{
        PatchEditTarget, PatchEditType, PatchTestPlan, ProposedPatchChange,
    },
    symbol_index_retrieval_plan::{QueryIntent, RetrievalPlan},
};

pub(crate) fn build_proposed_changes(
    must_edit: &[PatchEditTarget],
    should_inspect: &[PatchEditTarget],
    plan: &RetrievalPlan,
    task: &str,
) -> Vec<ProposedPatchChange> {
    let mut changes = must_edit
        .iter()
        .take(8)
        .map(|target| proposed_change(target, plan, task))
        .collect::<Vec<_>>();
    if changes.is_empty() {
        changes.extend(
            should_inspect
                .iter()
                .take(3)
                .map(|target| proposed_change(target, plan, task)),
        );
    }
    changes
}

pub(crate) fn build_test_plan(
    must_edit: &[PatchEditTarget],
    should_inspect: &[PatchEditTarget],
    maybe_edit: &[PatchEditTarget],
    task: &str,
) -> PatchTestPlan {
    let targets = must_edit
        .iter()
        .chain(should_inspect.iter())
        .chain(maybe_edit.iter())
        .filter(|target| {
            matches!(
                target.edit_type,
                PatchEditType::UpdateTest | PatchEditType::AddTest
            )
        })
        .filter_map(|target| target.qualified_name.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut commands = targets
        .iter()
        .take(3)
        .map(|target| format!("cargo test {}", test_selector(target)))
        .collect::<Vec<_>>();
    if commands.is_empty() && has_rust_target(must_edit, should_inspect, maybe_edit) {
        commands.push("cargo test".to_string());
    }
    PatchTestPlan {
        commands,
        target_tests: targets,
        expected_behavior: vec![
            format!("The implementation satisfies: {task}"),
            "No unrelated public behavior changes are introduced.".to_string(),
        ],
    }
}

pub(crate) fn risk_notes(plan: &RetrievalPlan) -> Vec<String> {
    match plan.intent {
        QueryIntent::DebugError => vec![
            "Separate the actual error source from the error-to-response mapping before editing."
                .to_string(),
            "Do not hide infrastructure/database failures behind user-facing auth/client errors."
                .to_string(),
        ],
        QueryIntent::ModifyBehavior => vec![
            "Update behavior and regression tests together.".to_string(),
            "Check caller and error mapping contexts before narrowing the patch.".to_string(),
        ],
        QueryIntent::Refactor => vec![
            "Keep the refactor behavior-preserving unless the task explicitly asks otherwise."
                .to_string(),
            "Do not finish until references, impls, tests, and exports are accounted for."
                .to_string(),
        ],
        QueryIntent::AddFeature => vec![
            "Follow the existing handler/service/repository/test layering.".to_string(),
            "Add tests before broadening the feature across unrelated modules.".to_string(),
        ],
        QueryIntent::Test => {
            vec!["Prefer a narrow failing/passing test loop before broad checks.".to_string()]
        }
        QueryIntent::Locate | QueryIntent::Explain => {
            vec![
                "This query is context-only; do not generate a code patch from it by default."
                    .to_string(),
            ]
        }
        QueryIntent::Unknown => {
            vec!["Intent is uncertain; inspect selected targets before applying edits.".to_string()]
        }
    }
}

pub(crate) fn open_questions(
    plan: &RetrievalPlan,
    must_edit: &[PatchEditTarget],
    should_inspect: &[PatchEditTarget],
    test_plan: &PatchTestPlan,
) -> Vec<String> {
    let mut questions = Vec::new();
    if !patch_required(plan) {
        questions.push(
            "Patch is not required for this intent unless the user explicitly asks for edits."
                .to_string(),
        );
    }
    if patch_required(plan) && test_plan.target_tests.is_empty() {
        questions.push("No related test target was found; add or locate a regression test before finalizing behavior changes.".to_string());
    }
    if matches!(plan.intent, QueryIntent::DebugError)
        && !must_edit
            .iter()
            .chain(should_inspect.iter())
            .any(|target| target.edit_type == PatchEditType::ModifyErrorMapping)
    {
        questions.push("No explicit error/status mapping target was found; inspect handler/error conversion manually.".to_string());
    }
    questions
}

fn proposed_change(
    target: &PatchEditTarget,
    plan: &RetrievalPlan,
    task: &str,
) -> ProposedPatchChange {
    let (desired_behavior, instructions, constraints, current_behavior) =
        change_guidance(target, plan, task);
    ProposedPatchChange {
        target_file_path: target.file_path.clone(),
        target_symbol: target.qualified_name.clone(),
        edit_type: target.edit_type,
        current_behavior,
        desired_behavior,
        instructions,
        constraints,
    }
}

fn change_guidance(
    target: &PatchEditTarget,
    plan: &RetrievalPlan,
    task: &str,
) -> (String, Vec<String>, Vec<String>, Option<String>) {
    match target.edit_type {
        PatchEditType::ModifyErrorMapping => (
            "Ensure the error/status mapping matches the requested behavior.".to_string(),
            vec![
                "Inspect the conversion from domain errors to transport/status responses."
                    .to_string(),
                "Add or adjust only the mapping branch needed by this task.".to_string(),
            ],
            vec![
                "Do not convert unrelated internal/database failures into user/auth failures."
                    .to_string(),
            ],
            Some(
                "A selected error/status context may currently map to the wrong response."
                    .to_string(),
            ),
        ),
        PatchEditType::UpdateTest | PatchEditType::AddTest => (
            "Keep or add a regression test that proves the requested behavior.".to_string(),
            vec![
                "Use the selected test hint as the first validation target.".to_string(),
                "Assert the observable behavior described by the user task.".to_string(),
            ],
            vec!["Prefer a narrow test before broad suite runs.".to_string()],
            None,
        ),
        PatchEditType::RenameSymbol | PatchEditType::UpdateReferences => (
            "Apply the refactor across definitions, references, implementations, and tests."
                .to_string(),
            vec![
                "Update the definition and every selected reference consistently.".to_string(),
                "Check public exports, trait impls, and test references before finishing."
                    .to_string(),
            ],
            vec![
                "Avoid behavior changes while doing a rename/refactor unless explicitly required."
                    .to_string(),
            ],
            None,
        ),
        PatchEditType::AddRoute
        | PatchEditType::AddServiceMethod
        | PatchEditType::AddRepositoryMethod
        | PatchEditType::AddErrorVariant
        | PatchEditType::AddConfig => (
            format!("Implement the requested feature using the existing project pattern: {task}"),
            vec![
                "Mirror the selected neighboring handler/service/repository style.".to_string(),
                "Wire the new path through the same error and test conventions as related code."
                    .to_string(),
            ],
            vec!["Do not introduce a parallel framework or duplicate architecture.".to_string()],
            None,
        ),
        PatchEditType::InspectOnly => (
            "Inspect this context before deciding whether code should change here.".to_string(),
            vec![
                "Read the selected range and verify whether it owns the observed behavior."
                    .to_string(),
            ],
            vec![
                "Do not edit inspect-only targets unless the code confirms ownership.".to_string(),
            ],
            None,
        ),
        PatchEditType::ModifyBehavior => (
            format!("Modify behavior to satisfy the user task: {task}"),
            vec![
                "Change the selected target only where it owns the requested behavior.".to_string(),
                "Preserve callers/callees that are unrelated to the requested change.".to_string(),
            ],
            intent_constraints(plan),
            Some(
                "Selected target currently appears to own part of the requested behavior."
                    .to_string(),
            ),
        ),
    }
}

fn intent_constraints(plan: &RetrievalPlan) -> Vec<String> {
    if matches!(plan.intent, QueryIntent::DebugError) {
        vec![
            "Do not change unrelated error paths while fixing the observed failure.".to_string(),
            "Preserve successful-path behavior.".to_string(),
        ]
    } else {
        vec!["Keep the patch scoped to selected targets and related tests.".to_string()]
    }
}

fn test_selector(target: &str) -> String {
    target
        .rsplit("::")
        .next()
        .unwrap_or(target)
        .trim_matches('`')
        .to_string()
}

fn has_rust_target(
    must_edit: &[PatchEditTarget],
    should_inspect: &[PatchEditTarget],
    maybe_edit: &[PatchEditTarget],
) -> bool {
    must_edit
        .iter()
        .chain(should_inspect.iter())
        .chain(maybe_edit.iter())
        .any(|target| target.file_path.ends_with(".rs"))
}
