use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use chrono::Utc;

use super::types::{
    FitCase, FitPrior, FitPriorEnvironment, FitPriorMatch, FitPriorQuery, FitPriorScope,
    FitTranslationFeatures,
};

const MIN_CROSS_SUCCESSES: usize = 3;
const MIN_CROSS_SCREENS: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PriorGroupKey {
    scope: u8,
    component_kind: String,
    definition_id: Option<String>,
    properties: String,
    density_bucket: Option<i32>,
    font_scale_bucket: Option<i32>,
    theme: Option<String>,
    parent_layout_kind: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct FitPriorIndex {
    priors: Vec<FitPrior>,
}

impl FitPriorIndex {
    pub(crate) fn build_candidates(cases: &[FitCase]) -> Self {
        let mut groups = BTreeMap::<PriorGroupKey, Vec<&FitCase>>::new();
        for case in cases {
            groups
                .entry(group_key(case, FitPriorScope::ExactComponent))
                .or_default()
                .push(case);
            groups
                .entry(group_key(case, FitPriorScope::CrossComponent))
                .or_default()
                .push(case);
        }
        let priors = groups
            .into_iter()
            .filter_map(|(key, evidence)| build_prior(key, evidence))
            .collect();
        Self { priors }
    }

    pub(crate) fn from_priors(priors: Vec<FitPrior>) -> Self {
        Self { priors }
    }

    pub(crate) fn priors(&self) -> &[FitPrior] {
        &self.priors
    }

    pub(crate) fn into_priors(self) -> Vec<FitPrior> {
        self.priors
    }

    pub(crate) fn top_k(&self, query: &FitPriorQuery) -> Vec<FitPriorMatch> {
        let mut matches = self
            .priors
            .iter()
            .filter_map(|prior| {
                match_score(prior, query).map(|score| FitPriorMatch {
                    prior: prior.clone(),
                    score,
                })
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.prior.prior_id.cmp(&right.prior.prior_id))
        });
        matches.truncate(query.limit.clamp(1, 50));
        matches
    }
}

pub(crate) fn prior_matches_case(prior: &FitPrior, case: &FitCase) -> bool {
    if prior.component_kind != case.component_kind || prior.property_set != case.property_set {
        return false;
    }
    if prior.scope == FitPriorScope::ExactComponent
        && prior.definition_id.as_deref() != Some(case.definition_id.as_str())
    {
        return false;
    }
    environment_matches(prior, case)
}

fn group_key(case: &FitCase, scope: FitPriorScope) -> PriorGroupKey {
    PriorGroupKey {
        scope: match scope {
            FitPriorScope::ExactComponent => 0,
            FitPriorScope::CrossComponent => 1,
        },
        component_kind: case.component_kind.clone(),
        definition_id: (scope == FitPriorScope::ExactComponent).then(|| case.definition_id.clone()),
        properties: case.property_set.join("\u{1f}"),
        density_bucket: bucket(case.environment.density, 0.5),
        font_scale_bucket: bucket(case.environment.font_scale, 0.1),
        theme: case
            .environment
            .theme
            .clone()
            .map(|value| value.to_ascii_lowercase()),
        parent_layout_kind: case.translation_features.parent_layout_kind.clone(),
    }
}

fn build_prior(key: PriorGroupKey, evidence: Vec<&FitCase>) -> Option<FitPrior> {
    let successes = evidence
        .iter()
        .copied()
        .filter(|case| case.passes_promotion_gates())
        .collect::<Vec<_>>();
    let screens = successes
        .iter()
        .map(|case| case.screen_key())
        .collect::<BTreeSet<_>>();
    let scope = if key.scope == 0 {
        FitPriorScope::ExactComponent
    } else {
        FitPriorScope::CrossComponent
    };
    let qualified = match scope {
        FitPriorScope::ExactComponent => !successes.is_empty(),
        FitPriorScope::CrossComponent => {
            successes.len() >= MIN_CROSS_SUCCESSES && screens.len() >= MIN_CROSS_SCREENS
        }
    };
    if !qualified {
        return None;
    }
    let success_count = successes.len() as u32;
    let failure_count = evidence.len().saturating_sub(successes.len()) as u32;
    let success_rate = success_count as f64 / evidence.len().max(1) as f64;
    let confidence = confidence(scope, success_count, screens.len() as u32, success_rate);
    let properties = key
        .properties
        .split('\u{1f}')
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let prior_id = stable_prior_id(&key);
    Some(FitPrior {
        prior_id,
        scope,
        component_kind: key.component_kind,
        definition_id: key.definition_id,
        property_set: properties,
        environment: FitPriorEnvironment {
            density: median(
                successes
                    .iter()
                    .filter_map(|case| case.environment.density.map(f64::from)),
            ),
            font_scale: median(
                successes
                    .iter()
                    .filter_map(|case| case.environment.font_scale.map(f64::from)),
            ),
            theme: key.theme,
        },
        success_count,
        failure_count,
        screen_count: screens.len() as u32,
        success_rate,
        confidence,
        translation_features: aggregate_translation_features(&successes, key.parent_layout_kind),
        median_deltas: median_deltas(&successes),
        median_factors: median_factors(&successes),
        case_ids: unique(successes.iter().map(|case| case.case_id.clone())),
        run_ids: unique(successes.iter().map(|case| case.provenance.run_id.clone())),
        source_revisions: unique(
            successes
                .iter()
                .filter_map(|case| case.provenance.source_revision.clone()),
        ),
        target_sha256s: unique(
            successes
                .iter()
                .map(|case| case.provenance.target_sha256.clone()),
        ),
        holdout: None,
        updated_at: Utc::now().to_rfc3339(),
    })
}

fn aggregate_translation_features(
    cases: &[&FitCase],
    parent_layout_kind: Option<String>,
) -> FitTranslationFeatures {
    FitTranslationFeatures {
        parent_layout_kind,
        target_width_ratio: median(
            cases
                .iter()
                .filter_map(|case| case.translation_features.target_width_ratio),
        ),
        target_height_ratio: median(
            cases
                .iter()
                .filter_map(|case| case.translation_features.target_height_ratio),
        ),
        current_width_ratio: median(
            cases
                .iter()
                .filter_map(|case| case.translation_features.current_width_ratio),
        ),
        current_height_ratio: median(
            cases
                .iter()
                .filter_map(|case| case.translation_features.current_height_ratio),
        ),
        width_scale: median(
            cases
                .iter()
                .filter_map(|case| case.translation_features.width_scale),
        ),
        height_scale: median(
            cases
                .iter()
                .filter_map(|case| case.translation_features.height_scale),
        ),
        target_aspect_ratio: median(
            cases
                .iter()
                .filter_map(|case| case.translation_features.target_aspect_ratio),
        ),
        current_aspect_ratio: median(
            cases
                .iter()
                .filter_map(|case| case.translation_features.current_aspect_ratio),
        ),
    }
}

fn median_deltas(cases: &[&FitCase]) -> BTreeMap<String, f64> {
    let mut values = BTreeMap::<String, Vec<f64>>::new();
    for case in cases {
        for adjustment in &case.adjustments {
            if let Some(delta) = adjustment.delta.filter(|value| value.is_finite()) {
                values
                    .entry(adjustment.property.clone())
                    .or_default()
                    .push(delta);
            }
        }
    }
    values
        .into_iter()
        .filter_map(|(property, values)| median(values.into_iter()).map(|value| (property, value)))
        .collect()
}

fn median_factors(cases: &[&FitCase]) -> BTreeMap<String, f64> {
    let mut values = BTreeMap::<String, Vec<f64>>::new();
    for case in cases {
        for adjustment in &case.adjustments {
            let factor = adjustment
                .first_value
                .zip(adjustment.final_value)
                .filter(|(first, final_value)| {
                    first.is_finite() && final_value.is_finite() && first.abs() > 0.000_001
                })
                .map(|(first, final_value)| final_value / first)
                .filter(|value| value.is_finite() && *value > 0.0 && *value <= 10.0);
            if let Some(factor) = factor {
                values
                    .entry(adjustment.property.clone())
                    .or_default()
                    .push(factor);
            }
        }
    }
    values
        .into_iter()
        .filter_map(|(property, values)| median(values.into_iter()).map(|value| (property, value)))
        .collect()
}

fn median(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut values = values.filter(|value| value.is_finite()).collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[middle - 1] + values[middle]) / 2.0)
    } else {
        Some(values[middle])
    }
}

fn confidence(scope: FitPriorScope, successes: u32, screens: u32, success_rate: f64) -> f64 {
    let sample_target = if scope == FitPriorScope::ExactComponent {
        2.0
    } else {
        6.0
    };
    let sample = (f64::from(successes) / sample_target).min(1.0);
    let coverage = (f64::from(screens) / 3.0).min(1.0);
    (success_rate * (0.7 * sample + 0.3 * coverage)).clamp(0.0, 1.0)
}

fn match_score(prior: &FitPrior, query: &FitPriorQuery) -> Option<f64> {
    if prior.component_kind != query.component_kind.trim().to_ascii_lowercase() {
        return None;
    }
    let property_similarity = jaccard(&prior.property_set, &query.properties);
    if property_similarity <= 0.0 {
        return None;
    }
    let definition = match (&prior.definition_id, &query.definition_id) {
        (Some(left), Some(right)) if left == right => 55.0,
        (Some(_), _) => 0.0,
        _ => 16.0,
    };
    let density = proximity(
        prior.environment.density,
        query.density.map(f64::from),
        0.75,
        10.0,
    );
    let font = proximity(
        prior.environment.font_scale,
        query.font_scale.map(f64::from),
        0.2,
        8.0,
    );
    let theme = match (&prior.environment.theme, &query.theme) {
        (Some(left), Some(right)) if left.eq_ignore_ascii_case(right) => 7.0,
        (None, _) => 2.0,
        (Some(_), Some(_)) => -8.0,
        _ => 0.0,
    };
    let layout = match (
        &prior.translation_features.parent_layout_kind,
        &query.translation_features.parent_layout_kind,
    ) {
        (Some(left), Some(right)) if left.eq_ignore_ascii_case(right) => 12.0,
        (Some(_), Some(_)) => -14.0,
        _ => 2.0,
    };
    let geometry = translation_similarity(&prior.translation_features, &query.translation_features);
    Some(
        definition
            + property_similarity * 30.0
            + density
            + font
            + theme
            + layout
            + geometry
            + prior.confidence * 12.0
            + prior.success_rate * 8.0,
    )
}

fn translation_similarity(left: &FitTranslationFeatures, right: &FitTranslationFeatures) -> f64 {
    proximity(left.target_width_ratio, right.target_width_ratio, 0.25, 6.0)
        + proximity(
            left.target_height_ratio,
            right.target_height_ratio,
            0.25,
            6.0,
        )
        + proximity(left.width_scale, right.width_scale, 0.75, 4.0)
        + proximity(left.height_scale, right.height_scale, 0.75, 4.0)
}

fn environment_matches(prior: &FitPrior, case: &FitCase) -> bool {
    proximity(
        prior.environment.density,
        case.environment.density.map(f64::from),
        0.75,
        1.0,
    ) > 0.0
        && proximity(
            prior.environment.font_scale,
            case.environment.font_scale.map(f64::from),
            0.2,
            1.0,
        ) > 0.0
        && match (&prior.environment.theme, &case.environment.theme) {
            (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
            _ => true,
        }
}

fn proximity(left: Option<f64>, right: Option<f64>, tolerance: f64, weight: f64) -> f64 {
    match (left, right) {
        (Some(left), Some(right)) => (1.0 - ((left - right).abs() / tolerance)).max(0.0) * weight,
        _ => weight * 0.35,
    }
}

fn jaccard(left: &[String], right: &[String]) -> f64 {
    let left = left.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let right = right.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let union = left.union(&right).count();
    if union == 0 {
        1.0
    } else {
        left.intersection(&right).count() as f64 / union as f64
    }
}

fn bucket(value: Option<f32>, step: f32) -> Option<i32> {
    value
        .filter(|value| value.is_finite())
        .map(|value| (value / step).round() as i32)
}

fn stable_prior_id(key: &PriorGroupKey) -> String {
    let raw = format!("{key:?}");
    let hash = raw.bytes().fold(2_166_136_261_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    });
    format!("prior:{hash:08x}")
}

fn unique(values: impl Iterator<Item = String>) -> Vec<String> {
    values.collect::<BTreeSet<_>>().into_iter().collect()
}
