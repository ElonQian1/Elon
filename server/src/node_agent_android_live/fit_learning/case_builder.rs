use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use chrono::Utc;
use serde_json::Value;

use super::super::fit_run::{FitRunDocument, FitRunPhase, FitScore, FitTrial};
use super::types::{
    FitCase, FitCaseEnvironment, FitCaseOutcome, FitCaseProvenance, FitCaseReview,
    FitPropertyAdjustment, FitScoreEvidence, FitTranslationFeatures, FitTrialEvidence,
    FitUserDecision, FIT_CASE_SCHEMA_VERSION,
};

impl FitCase {
    pub(crate) fn from_fit_run(
        run: &FitRunDocument,
        trials: &[FitTrial],
        review: FitCaseReview,
    ) -> Self {
        let final_candidate = run.current.as_ref().or(run.best.as_ref());
        let target_score_passed =
            final_candidate.is_some_and(|candidate| candidate.score.passes(&run.thresholds));
        let source_parity_passed = final_candidate.is_some_and(|candidate| {
            candidate.source_parity_verified
                && candidate
                    .source_parity_loss
                    .is_some_and(|loss| loss <= run.thresholds.max_source_parity_loss)
        });
        let promotable = run.phase == FitRunPhase::Accepted
            && target_score_passed
            && source_parity_passed
            && review.decision == FitUserDecision::Accepted;
        let outcome = case_outcome(run.phase, review.decision, promotable);
        let trial_evidence = trials.iter().map(to_trial_evidence).collect::<Vec<_>>();
        let reviewed_at = review.decided_at.unwrap_or_else(|| Utc::now().to_rfc3339());
        Self {
            schema_version: FIT_CASE_SCHEMA_VERSION,
            case_id: format!("case:{}", run.run_id),
            // 学习规范会进入项目源码；绝不能把开发机绝对路径写进 Git。
            project_root: ".".to_string(),
            package_name: run.package_name.clone(),
            definition_id: run.pair.definition_id.clone(),
            component_kind: normalize_kind(&review.component_kind),
            property_set: normalized_properties(&run.properties, trials),
            environment: FitCaseEnvironment {
                screen_id: run.environment.screen_id.clone(),
                scenario: run.environment.scenario.clone(),
                theme: run.environment.theme.clone(),
                locale: run.environment.locale.clone(),
                density: run.environment.density,
                font_scale: run.environment.font_scale,
                viewport_width: run.environment.viewport_width,
                viewport_height: run.environment.viewport_height,
            },
            translation_features: translation_features(run),
            run_phase: phase_name(run.phase),
            outcome,
            user_decision: review.decision,
            target_score_passed,
            source_parity_passed,
            promotable,
            baseline_score: run
                .baseline
                .as_ref()
                .map(|candidate| score_evidence(&candidate.score)),
            final_score: final_candidate.map(|candidate| score_evidence(&candidate.score)),
            source_parity_loss: final_candidate.and_then(|candidate| candidate.source_parity_loss),
            adjustments: property_adjustments(trials),
            trials: trial_evidence,
            provenance: FitCaseProvenance {
                run_id: run.run_id.clone(),
                target_sha256: run.pair.target_sha256.clone(),
                source_revision: final_candidate
                    .and_then(|candidate| candidate.source_revision.clone())
                    .or_else(|| run.source_revision.clone()),
                runtime_build_id: final_candidate
                    .and_then(|candidate| candidate.runtime_build_id.clone())
                    .or_else(|| run.runtime_build_id.clone()),
                commit_id: final_candidate.and_then(|candidate| candidate.commit_id.clone()),
                trial_ids: trials.iter().map(|trial| trial.trial_id.clone()).collect(),
                final_screenshot_path: project_relative_artifact(
                    &run.project_root,
                    final_candidate.and_then(|candidate| candidate.screenshot_path.as_deref()),
                ),
                final_diff_artifact_path: project_relative_artifact(
                    &run.project_root,
                    final_candidate.and_then(|candidate| candidate.diff_artifact_path.as_deref()),
                ),
            },
            reviewed_at,
            review_note: review.note,
        }
    }
}

pub(crate) fn translation_features(run: &FitRunDocument) -> FitTranslationFeatures {
    let viewport_width = run.environment.viewport_width.map(f64::from);
    let viewport_height = run.environment.viewport_height.map(f64::from);
    let target_width = extent(
        run.pair.projected_target_rect.left,
        run.pair.projected_target_rect.right,
    );
    let target_height = extent(
        run.pair.projected_target_rect.top,
        run.pair.projected_target_rect.bottom,
    );
    let current_width = extent(run.pair.current_rect.left, run.pair.current_rect.right);
    let current_height = extent(run.pair.current_rect.top, run.pair.current_rect.bottom);
    FitTranslationFeatures {
        parent_layout_kind: run
            .pair
            .parent_layout_kind
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase()),
        target_width_ratio: ratio(target_width, viewport_width),
        target_height_ratio: ratio(target_height, viewport_height),
        current_width_ratio: ratio(current_width, viewport_width),
        current_height_ratio: ratio(current_height, viewport_height),
        width_scale: ratio(target_width, current_width),
        height_scale: ratio(target_height, current_height),
        target_aspect_ratio: ratio(target_width, target_height),
        current_aspect_ratio: ratio(current_width, current_height),
    }
}

fn extent(start: i32, end: i32) -> Option<f64> {
    (end > start).then(|| f64::from(end - start))
}

fn ratio(numerator: Option<f64>, denominator: Option<f64>) -> Option<f64> {
    numerator
        .zip(denominator)
        .filter(|(left, right)| left.is_finite() && right.is_finite() && right.abs() > 0.000_001)
        .map(|(left, right)| left / right)
        .filter(|value| value.is_finite())
}

pub(super) fn sanitize_case_for_storage(mut case: FitCase) -> FitCase {
    let project_root = case.project_root.clone();
    case.provenance.final_screenshot_path = project_relative_artifact(
        &project_root,
        case.provenance.final_screenshot_path.as_deref(),
    );
    case.provenance.final_diff_artifact_path = project_relative_artifact(
        &project_root,
        case.provenance.final_diff_artifact_path.as_deref(),
    );
    case.project_root = ".".to_string();
    case
}

fn project_relative_artifact(project_root: &str, value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    let root = PathBuf::from(project_root);
    let artifact = PathBuf::from(value);
    let relative = if artifact.is_absolute() {
        artifact
            .strip_prefix(&root)
            .ok()
            .map(Path::to_path_buf)
            .or_else(|| {
                let canonical_root = root.canonicalize().ok()?;
                let canonical_artifact = artifact.canonicalize().ok()?;
                canonical_artifact
                    .strip_prefix(canonical_root)
                    .ok()
                    .map(Path::to_path_buf)
            })?
    } else {
        artifact
    };
    let mut safe = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::Normal(value) => safe.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!safe.as_os_str().is_empty()).then(|| safe.to_string_lossy().replace('\\', "/"))
}

fn case_outcome(phase: FitRunPhase, decision: FitUserDecision, promotable: bool) -> FitCaseOutcome {
    if decision == FitUserDecision::Rejected {
        return FitCaseOutcome::Rejected;
    }
    if promotable {
        return FitCaseOutcome::Accepted;
    }
    match phase {
        FitRunPhase::Failed => FitCaseOutcome::Failed,
        FitRunPhase::Plateau => FitCaseOutcome::Plateau,
        FitRunPhase::Cancelled => FitCaseOutcome::Cancelled,
        _ => FitCaseOutcome::Incomplete,
    }
}

fn phase_name(phase: FitRunPhase) -> String {
    serde_json::to_value(phase)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

fn normalize_kind(value: &str) -> String {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() {
        "unknown".to_string()
    } else {
        value
    }
}

fn normalized_properties(properties: &[String], trials: &[FitTrial]) -> Vec<String> {
    let mut result = properties
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    for operation in trials
        .iter()
        .filter_map(|trial| trial.candidate.as_ref())
        .flat_map(|candidate| &candidate.operations)
    {
        if let Some(property) = operation.get("property").and_then(Value::as_str) {
            result.insert(property.to_string());
        }
    }
    result.into_iter().collect()
}

fn property_adjustments(trials: &[FitTrial]) -> Vec<FitPropertyAdjustment> {
    let mut values = BTreeMap::<String, (Option<f64>, Option<f64>, u32)>::new();
    for operation in trials
        .iter()
        .filter_map(|trial| trial.candidate.as_ref())
        .flat_map(|candidate| &candidate.operations)
    {
        let Some(property) = operation.get("property").and_then(Value::as_str) else {
            continue;
        };
        let after = nested_number(operation.get("value"));
        let before = nested_number(operation.get("beforeValue"))
            .or_else(|| nested_number(operation.get("before")))
            .or_else(|| nested_number(operation.get("previousValue")));
        let entry = values.entry(property.to_string()).or_default();
        if entry.0.is_none() {
            entry.0 = before.or(entry.1);
        }
        if after.is_some() {
            entry.1 = after;
            entry.2 = entry.2.saturating_add(1);
        }
    }
    values
        .into_iter()
        .map(
            |(property, (first_value, final_value, observations))| FitPropertyAdjustment {
                property,
                first_value,
                final_value,
                delta: first_value
                    .zip(final_value)
                    .map(|(first, last)| last - first),
                observations,
            },
        )
        .collect()
}

fn nested_number(value: Option<&Value>) -> Option<f64> {
    value.and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.get("value").and_then(Value::as_f64))
    })
}

fn to_trial_evidence(trial: &FitTrial) -> FitTrialEvidence {
    FitTrialEvidence {
        trial_id: trial.trial_id.clone(),
        kind: serde_json::to_value(trial.kind)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_else(|| "UNKNOWN".to_string()),
        accepted_as_best: trial.accepted_as_best,
        score: trial
            .candidate
            .as_ref()
            .map(|candidate| score_evidence(&candidate.score)),
        error: trial.error.clone(),
    }
}

fn score_evidence(score: &FitScore) -> FitScoreEvidence {
    FitScoreEvidence {
        scorer_version: score.scorer_version.clone(),
        overall_loss: score.overall_loss,
        geometry_error: score.geometry_error,
        color_error: score.color_error,
        edge_error: score.edge_error,
        hard_failures: score.hard_failures.clone(),
    }
}
