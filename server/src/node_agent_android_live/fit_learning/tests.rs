use std::fs;

use anyhow::Result;
use serde_json::{json, Value};

use super::super::fit_run::{FitRunDocument, FitTrial};
use super::eval::FitHoldoutEvaluator;
use super::prior_index::FitPriorIndex;
use super::promotion::{promote_priors, FitPromotionPolicy};
use super::store::FitLearningStore;
use super::types::{
    FitCase, FitCaseEnvironment, FitCaseOutcome, FitCaseProvenance, FitCaseReview,
    FitHoldoutResult, FitPrior, FitPriorQuery, FitPriorScope, FitPropertyAdjustment,
    FitScoreEvidence, FitUserDecision, FIT_CASE_SCHEMA_VERSION,
};

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
fn exact_prior_needs_one_case_but_cross_prior_needs_three_cases_and_two_screens() {
    let first = fit_case("1", "checkout.pay", "checkout", 4.0, true);
    let second = fit_case("2", "profile.save", "profile", 6.0, true);
    let only_two = FitPriorIndex::build_candidates(&[first.clone(), second.clone()]);
    assert_eq!(count_scope(&only_two, FitPriorScope::ExactComponent), 2);
    assert_eq!(count_scope(&only_two, FitPriorScope::CrossComponent), 0);

    let third = fit_case("3", "checkout.cancel", "checkout", 8.0, true);
    let rejected = fit_case("4", "checkout.other", "checkout", 100.0, false);
    let index = FitPriorIndex::build_candidates(&[first, second, third, rejected]);
    let cross = index
        .priors()
        .iter()
        .find(|prior| prior.scope == FitPriorScope::CrossComponent)
        .expect("cross prior");
    assert_eq!(cross.success_count, 3);
    assert_eq!(cross.failure_count, 1);
    assert_eq!(cross.screen_count, 2);
    assert_eq!(cross.success_rate, 0.75);
    assert_eq!(cross.median_deltas.get("height"), Some(&6.0));
}

#[test]
fn top_k_prefers_exact_definition_then_environment() {
    let cases = vec![
        fit_case("1", "checkout.pay", "checkout", 4.0, true),
        fit_case("2", "profile.save", "profile", 5.0, true),
        fit_case("3", "settings.save", "settings", 6.0, true),
    ];
    let index = FitPriorIndex::build_candidates(&cases);
    let matches = index.top_k(&FitPriorQuery {
        component_kind: "button".into(),
        definition_id: Some("checkout.pay".into()),
        properties: vec!["height".into()],
        density: Some(3.0),
        font_scale: Some(1.0),
        theme: Some("dark".into()),
        limit: 3,
    });
    assert!(!matches.is_empty());
    assert_eq!(
        matches[0].prior.definition_id.as_deref(),
        Some("checkout.pay")
    );
    assert!(matches
        .windows(2)
        .all(|pair| pair[0].score >= pair[1].score));
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

#[test]
fn store_retains_negative_evidence_and_atomically_updates_priors() {
    let root = std::env::temp_dir().join(format!(
        "elon-fit-learning-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    let store = FitLearningStore::new(&root).unwrap();
    let accepted = fit_case("1", "checkout.pay", "checkout", 4.0, true);
    let rejected = fit_case("2", "checkout.pay", "checkout", 8.0, false);
    store.record_case(accepted.clone()).unwrap();
    let cases = store.record_case(rejected).unwrap();
    assert_eq!(cases.cases.len(), 2);
    assert!(cases.cases.iter().any(|case| !case.promotable));

    let result = promote_priors(
        &cases.cases,
        &[],
        &MockEvaluator { regress: false },
        &FitPromotionPolicy::default(),
    )
    .unwrap();
    store.save_priors(&result.document).unwrap();
    let loaded = store.load_priors().unwrap();
    assert!(!loaded.priors.is_empty());
    assert!(store
        .priors_path()
        .ends_with(".elon/ui-standards/fit-priors.v1.json"));
    assert_eq!(store.load_cases().unwrap().cases.len(), 2);
    let cases_text = fs::read_to_string(store.cases_path()).unwrap();
    assert!(!cases_text.contains("D:/project"));
    assert!(cases_text.contains("\"projectRoot\": \".\""));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fixed_learning_backup_recovers_corrupt_primary() {
    let root = std::env::temp_dir().join(format!(
        "elon-fit-learning-backup-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    let store = FitLearningStore::new(&root).unwrap();
    store
        .record_case(fit_case("1", "checkout.pay", "checkout", 4.0, true))
        .unwrap();
    store
        .record_case(fit_case("2", "profile.save", "profile", 6.0, true))
        .unwrap();
    let cases_path = store.cases_path();
    let backup_path = cases_path.with_file_name("fit-cases.v1.json.bak");
    assert!(backup_path.is_file());
    fs::write(&cases_path, b"corrupt-primary").unwrap();
    let recovered = store.load_cases().unwrap();
    assert_eq!(recovered.cases.len(), 1);
    assert!(serde_json::from_str::<Value>(&fs::read_to_string(&cases_path).unwrap()).is_ok());

    let promoted = promote_priors(
        &recovered.cases,
        &[],
        &MockEvaluator { regress: false },
        &FitPromotionPolicy::default(),
    )
    .unwrap()
    .document;
    store.save_priors(&promoted).unwrap();
    store.save_priors(&promoted).unwrap();
    let priors_path = store.priors_path();
    assert!(priors_path
        .with_file_name("fit-priors.v1.json.bak")
        .is_file());
    fs::write(&priors_path, b"corrupt-primary").unwrap();
    assert!(!store.load_priors().unwrap().priors.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn artifact_provenance_is_project_relative_or_removed() {
    let root = std::env::temp_dir().join(format!(
        "elon-fit-learning-paths-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let frame = root.join(".elon/ui-tuner/fit-runs/fit_1/frames/final.png");
    fs::create_dir_all(frame.parent().unwrap()).unwrap();
    fs::write(&frame, b"png").unwrap();
    let mut case = fit_case("1", "checkout.pay", "checkout", 4.0, true);
    case.project_root = root.display().to_string();
    case.provenance.final_screenshot_path = Some(frame.display().to_string());
    let outside = root.parent().unwrap().join("outside/diff.json");
    case.provenance.final_diff_artifact_path = Some(outside.display().to_string());
    let store = FitLearningStore::new(&root).unwrap();
    let document = store.record_case(case).unwrap();
    let persisted = &document.cases[0];
    assert_eq!(persisted.project_root, ".");
    assert_eq!(
        persisted.provenance.final_screenshot_path.as_deref(),
        Some(".elon/ui-tuner/fit-runs/fit_1/frames/final.png")
    );
    assert!(persisted.provenance.final_diff_artifact_path.is_none());
    let text = fs::read_to_string(store.cases_path()).unwrap();
    assert!(!text.contains(&root.display().to_string()));
    assert!(!text.contains(&outside.display().to_string()));
    fs::remove_dir_all(root).unwrap();
}

struct MockEvaluator {
    regress: bool,
}

impl FitHoldoutEvaluator for MockEvaluator {
    fn evaluate(&self, _prior: &FitPrior, case: &FitCase) -> Result<FitHoldoutResult> {
        Ok(FitHoldoutResult {
            case_id: case.case_id.clone(),
            baseline_loss: 0.02,
            promoted_loss: if self.regress { 0.08 } else { 0.01 },
            passed: !self.regress,
        })
    }
}

fn review(decision: FitUserDecision) -> FitCaseReview {
    FitCaseReview {
        decision,
        component_kind: "button".into(),
        decided_at: Some("2026-07-12T00:00:00Z".into()),
        note: None,
    }
}

fn count_scope(index: &FitPriorIndex, scope: FitPriorScope) -> usize {
    index
        .priors()
        .iter()
        .filter(|prior| prior.scope == scope)
        .count()
}

pub(super) fn fit_case(
    id: &str,
    definition_id: &str,
    screen_id: &str,
    delta: f64,
    promotable: bool,
) -> FitCase {
    FitCase {
        schema_version: FIT_CASE_SCHEMA_VERSION,
        case_id: format!("case:{id}"),
        project_root: "D:/project".into(),
        package_name: "com.example".into(),
        definition_id: definition_id.into(),
        component_kind: "button".into(),
        property_set: vec!["height".into()],
        environment: FitCaseEnvironment {
            screen_id: Some(screen_id.into()),
            scenario: Some("normal".into()),
            theme: Some("dark".into()),
            locale: Some("zh-CN".into()),
            density: Some(3.0),
            font_scale: Some(1.0),
            viewport_width: Some(1080),
            viewport_height: Some(2400),
        },
        run_phase: if promotable { "ACCEPTED" } else { "FAILED" }.into(),
        outcome: if promotable {
            FitCaseOutcome::Accepted
        } else {
            FitCaseOutcome::Rejected
        },
        user_decision: if promotable {
            FitUserDecision::Accepted
        } else {
            FitUserDecision::Rejected
        },
        target_score_passed: promotable,
        source_parity_passed: promotable,
        promotable,
        baseline_score: Some(score(0.2)),
        final_score: Some(score(if promotable { 0.01 } else { 0.2 })),
        source_parity_loss: promotable.then_some(0.01),
        adjustments: vec![FitPropertyAdjustment {
            property: "height".into(),
            first_value: Some(48.0),
            final_value: Some(48.0 + delta),
            delta: Some(delta),
            observations: 2,
        }],
        trials: Vec::new(),
        provenance: FitCaseProvenance {
            run_id: format!("fit_{id}"),
            target_sha256: format!("sha-{id}"),
            source_revision: Some(format!("source-{id}")),
            runtime_build_id: Some(format!("build-{id}")),
            commit_id: Some(format!("commit-{id}")),
            trial_ids: vec![format!("trial-{id}")],
            final_screenshot_path: None,
            final_diff_artifact_path: None,
        },
        reviewed_at: "2026-07-12T00:00:00Z".into(),
        review_note: None,
    }
}

fn score(loss: f64) -> FitScoreEvidence {
    FitScoreEvidence {
        scorer_version: "test-v1".into(),
        overall_loss: loss,
        geometry_error: loss,
        color_error: loss,
        edge_error: loss,
        hard_failures: Vec::new(),
    }
}

pub(super) fn trial_documents() -> Vec<FitTrial> {
    vec![serde_json::from_value(json!({
        "sequence": 1,
        "trialId": "trial-1",
        "kind": "LIVE_APPLY",
        "createdAt": "2026-07-12T00:00:00Z",
        "durationMs": 10,
        "evaluations": 2,
        "candidate": candidate(0.01, true, 0.01),
        "acceptedAsBest": true,
        "error": null,
        "checkpoint": checkpoint()
    }))
    .unwrap()]
}

fn run_document(phase: &str, loss: f64, parity: bool, parity_loss: f64) -> FitRunDocument {
    run_document_at("D:/project", phase, loss, parity, parity_loss)
}

pub(super) fn run_document_at(
    project_root: &str,
    phase: &str,
    loss: f64,
    parity: bool,
    parity_loss: f64,
) -> FitRunDocument {
    serde_json::from_value(json!({
        "schemaVersion": 1,
        "runId": "fit_1",
        "sessionId": "session_1",
        "projectRoot": project_root,
        "packageName": "com.example",
        "deviceId": "device-1",
        "phase": phase,
        "stopReason": "SOURCE_VERIFIED",
        "pair": {
            "targetDesignId": "design-1", "targetSha256": "sha-1",
            "targetRect": rect(), "runtimeNodeId": "node-1", "definitionId": "checkout.pay",
            "instanceKey": null, "currentRect": rect(), "projectedTargetRect": rect(),
            "calibrationId": null, "confidence": 1.0
        },
        "environment": {
            "screenId": "checkout", "scenario": "normal", "theme": "dark", "locale": "zh-CN",
            "viewportWidth": 1080, "viewportHeight": 2400, "density": 3.0,
            "fontScale": 1.0, "rotation": 0, "insets": null
        },
        "properties": ["height"],
        "budget": {
            "maxDurationMs": 1000, "maxLocalEvaluations": 10, "maxCodexRounds": 1,
            "maxBuildRounds": 1, "maxNoImprovementTrials": 3
        },
        "usage": {
            "elapsedMs": 10, "localEvaluations": 2, "codexRounds": 0,
            "buildRounds": 1, "noImprovementTrials": 0, "codexTokens": null
        },
        "thresholds": {
            "maxOverallLoss": 0.035, "maxGeometryError": 0.02, "maxColorError": 0.04,
            "maxEdgeError": 0.06, "maxSourceParityLoss": 0.035,
            "minMeaningfulImprovement": 0.001, "plateauWindow": 6
        },
        "baseline": candidate(0.2, false, 1.0),
        "current": candidate(loss, parity, parity_loss),
        "best": candidate(loss, parity, parity_loss),
        "handoff": null, "resumePhase": null, "runtimeBuildId": "build-1", "treeRevision": 1,
        "sourceRevision": "source-1", "createdAt": "2026-07-12T00:00:00Z",
        "updatedAt": "2026-07-12T00:00:00Z", "lastSequence": 1,
        "lastError": null, "processedCommands": []
    }))
    .unwrap()
}

fn candidate(loss: f64, parity: bool, parity_loss: f64) -> Value {
    json!({
        "trialId": "trial-1",
        "score": {
            "scorerVersion": "test-v1", "overallLoss": loss, "geometryError": loss,
            "colorError": loss, "edgeError": loss, "alphaError": 0.0,
            "shapeError": null, "typographyError": null, "hardFailures": []
        },
        "operations": [{
            "property": "height", "beforeValue": {"type": "dp", "value": 48.0},
            "value": {"type": "dp", "value": 54.0}
        }],
        "screenshotPath": "frame.png", "diffArtifactPath": "diff.json",
        "runtimeBuildId": "build-1", "sourceRevision": "source-1", "commitId": "commit-1",
        "sourceParityLoss": parity_loss, "sourceParityVerified": parity
    })
}

fn checkpoint() -> Value {
    json!({
        "phase": "ACCEPTED", "stopReason": "SOURCE_VERIFIED",
        "usage": {
            "elapsedMs": 10, "localEvaluations": 2, "codexRounds": 0,
            "buildRounds": 1, "noImprovementTrials": 0, "codexTokens": null
        },
        "current": null, "best": null
    })
}

fn rect() -> Value {
    json!({"left": 0, "top": 0, "right": 100, "bottom": 50})
}
