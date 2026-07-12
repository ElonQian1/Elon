mod index;
mod promotion;
mod store;

use anyhow::Result;
use serde_json::{json, Value};

use super::super::fit_run::{FitRunDocument, FitTrial};
use super::eval::FitHoldoutEvaluator;
use super::types::{
    FitCase, FitCaseEnvironment, FitCaseOutcome, FitCaseProvenance, FitHoldoutResult, FitPrior,
    FitPropertyAdjustment, FitScoreEvidence, FitTranslationFeatures, FitUserDecision,
    FIT_CASE_SCHEMA_VERSION,
};

pub(super) struct MockEvaluator {
    pub(super) regress: bool,
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
        translation_features: FitTranslationFeatures {
            parent_layout_kind: Some("column".into()),
            target_width_ratio: Some(0.5),
            target_height_ratio: Some(0.05),
            current_width_ratio: Some(0.45),
            current_height_ratio: Some(0.04),
            width_scale: Some(1.1),
            height_scale: Some(1.25),
            target_aspect_ratio: Some(4.5),
            current_aspect_ratio: Some(5.0),
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

pub(super) fn run_document(
    phase: &str,
    loss: f64,
    parity: bool,
    parity_loss: f64,
) -> FitRunDocument {
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
