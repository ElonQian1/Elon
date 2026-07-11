use super::super::promotion::{promote_priors, FitPromotionPolicy};
use super::super::types::{FitCase, FitCaseOutcome, FitCaseReview, FitPriorScope, FitUserDecision};
use super::{fit_case, run_document, trial_documents, MockEvaluator};

#[test]
fn fit_case_requires_all_four_promotion_gates() {
    let trials = trial_documents();
    let accepted = FitCase::from_fit_run(
        &run_document("ACCEPTED", 0.01, true, 0.01),
        &trials,
        review(FitUserDecision::Accepted),
    );
    assert!(accepted.promotable);
    assert_eq!(accepted.outcome, FitCaseOutcome::Accepted);
    assert_eq!(accepted.adjustments[0].delta, Some(6.0));

    let rejected = FitCase::from_fit_run(
        &run_document("ACCEPTED", 0.01, true, 0.01),
        &trials,
        review(FitUserDecision::Rejected),
    );
    assert!(!rejected.promotable);
    assert_eq!(rejected.outcome, FitCaseOutcome::Rejected);

    let bad_target = FitCase::from_fit_run(
        &run_document("ACCEPTED", 0.20, true, 0.01),
        &trials,
        review(FitUserDecision::Accepted),
    );
    assert!(!bad_target.promotable);
    assert!(!bad_target.target_score_passed);

    let bad_source = FitCase::from_fit_run(
        &run_document("ACCEPTED", 0.01, false, 0.20),
        &trials,
        review(FitUserDecision::Accepted),
    );
    assert!(!bad_source.promotable);
    assert!(!bad_source.source_parity_passed);

    let failed_phase = FitCase::from_fit_run(
        &run_document("FAILED", 0.01, true, 0.01),
        &trials,
        review(FitUserDecision::Accepted),
    );
    assert!(!failed_phase.promotable);
    assert_eq!(failed_phase.outcome, FitCaseOutcome::Failed);
}

#[test]
fn holdout_regression_rejects_cross_component_promotion() {
    let training = vec![
        fit_case("1", "checkout.pay", "checkout", 4.0, true),
        fit_case("2", "profile.save", "profile", 5.0, true),
        fit_case("3", "settings.save", "settings", 6.0, true),
    ];
    let holdout = vec![fit_case("regress", "new.save", "new-screen", 5.0, true)];
    let result = promote_priors(
        &training,
        &holdout,
        &MockEvaluator { regress: true },
        &FitPromotionPolicy::default(),
    )
    .unwrap();
    assert!(result
        .document
        .priors
        .iter()
        .all(|prior| prior.scope != FitPriorScope::CrossComponent));
    assert!(result
        .decisions
        .iter()
        .any(|decision| !decision.promoted && decision.reason.contains("holdout 回归")));
}

fn review(decision: FitUserDecision) -> FitCaseReview {
    FitCaseReview {
        decision,
        component_kind: "button".into(),
        decided_at: Some("2026-07-12T00:00:00Z".into()),
        note: None,
    }
}
