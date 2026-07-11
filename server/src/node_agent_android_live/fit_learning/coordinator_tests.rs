use std::fs;

use super::coordinator::{record_and_promote, top_k_for_run, FitLearningCoordinator};
use super::store::FitLearningStore;
use super::tests::{fit_case, run_document_at, trial_documents};
use super::types::{FitCaseOutcome, FitPriorScope, FitUserDecision};

#[test]
fn accepted_terminal_run_records_promotes_and_can_be_retrieved() {
    let root = temp_project("accepted");
    let run = run_document_at(root.to_str().unwrap(), "ACCEPTED", 0.01, true, 0.01);
    let trials = trial_documents();
    let result = record_and_promote(
        &run,
        &trials,
        FitUserDecision::Accepted,
        Some("用户确认按钮比例正确".into()),
    )
    .unwrap();
    assert!(result.case.promotable);
    assert_eq!(result.recorded_case_count, 1);
    assert_eq!(result.promotion.document.priors.len(), 1);
    assert_eq!(
        result.promotion.document.priors[0].scope,
        FitPriorScope::ExactComponent
    );
    assert_eq!(
        result.promotion.document.priors[0]
            .median_deltas
            .get("height"),
        Some(&6.0)
    );

    let matches = top_k_for_run(&run, 3).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(
        matches[0].prior.definition_id.as_deref(),
        Some("checkout.pay")
    );
    cleanup(root);
}

#[test]
fn failed_and_rejected_terminal_run_is_kept_as_negative_evidence() {
    let root = temp_project("negative");
    let run = run_document_at(root.to_str().unwrap(), "FAILED", 0.01, true, 0.01);
    let result = record_and_promote(
        &run,
        &trial_documents(),
        FitUserDecision::Rejected,
        Some("用户认为圆角仍不正确".into()),
    )
    .unwrap();
    assert!(!result.case.promotable);
    assert_eq!(result.case.outcome, FitCaseOutcome::Rejected);
    assert!(result.promotion.document.priors.is_empty());
    let cases = FitLearningStore::new(&root).unwrap().load_cases().unwrap();
    assert_eq!(cases.cases.len(), 1);
    assert!(!cases.cases[0].passes_promotion_gates());
    cleanup(root);
}

#[test]
fn non_terminal_run_is_not_recorded() {
    let root = temp_project("non-terminal");
    let run = run_document_at(root.to_str().unwrap(), "CREATED", 0.01, true, 0.01);
    let coordinator = FitLearningCoordinator::for_run(&run).unwrap();
    let error = coordinator
        .record_and_promote(&run, &trial_documents(), FitUserDecision::Pending, None)
        .unwrap_err();
    assert!(error.to_string().contains("只有终态"));
    assert!(!FitLearningStore::new(&root).unwrap().cases_path().exists());
    cleanup(root);
}

#[test]
fn historically_inconsistent_adjustment_blocks_prior_save() {
    let root = temp_project("regression");
    let store = FitLearningStore::new(&root).unwrap();
    store
        .record_case(fit_case(
            "historical-outlier",
            "checkout.pay",
            "checkout",
            30.0,
            true,
        ))
        .unwrap();
    let run = run_document_at(root.to_str().unwrap(), "ACCEPTED", 0.01, true, 0.01);
    let result =
        record_and_promote(&run, &trial_documents(), FitUserDecision::Accepted, None).unwrap();
    assert!(result.promotion.document.priors.is_empty());
    assert!(result
        .promotion
        .decisions
        .iter()
        .any(|decision| !decision.promoted && decision.reason.contains("holdout 回归")));
    assert!(store.load_priors().unwrap().priors.is_empty());
    cleanup(root);
}

fn temp_project(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "elon-fit-learning-{label}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn cleanup(root: std::path::PathBuf) {
    fs::remove_dir_all(root).unwrap();
}
