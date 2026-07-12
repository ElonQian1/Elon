use std::cmp::Ordering;

use anyhow::Result;
use serde::Serialize;

use super::store::FitLearningStore;
use super::types::{FitPriorScope, FitTranslationFeatures};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitLearningSummary {
    pub(crate) case_count: usize,
    pub(crate) promoted_case_count: usize,
    pub(crate) prior_count: usize,
    pub(crate) exact_prior_count: usize,
    pub(crate) cross_component_prior_count: usize,
    pub(crate) reusable_priors: Vec<FitLearningPriorSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FitLearningPriorSummary {
    pub(crate) prior_id: String,
    pub(crate) scope: FitPriorScope,
    pub(crate) component_kind: String,
    pub(crate) definition_id: Option<String>,
    pub(crate) property_set: Vec<String>,
    pub(crate) success_count: u32,
    pub(crate) screen_count: u32,
    pub(crate) confidence: f64,
    pub(crate) median_factors: std::collections::BTreeMap<String, f64>,
    pub(crate) translation_features: FitTranslationFeatures,
}

pub(crate) fn learning_summary(project_root: &str) -> Result<FitLearningSummary> {
    let store = FitLearningStore::new(project_root)?;
    let cases = store.load_cases()?;
    let priors = store.load_priors()?;
    let mut reusable_priors = priors
        .priors
        .iter()
        .filter(|prior| prior.confidence > 0.0)
        .map(|prior| FitLearningPriorSummary {
            prior_id: prior.prior_id.clone(),
            scope: prior.scope,
            component_kind: prior.component_kind.clone(),
            definition_id: prior.definition_id.clone(),
            property_set: prior.property_set.clone(),
            success_count: prior.success_count,
            screen_count: prior.screen_count,
            confidence: prior.confidence,
            median_factors: prior.median_factors.clone(),
            translation_features: prior.translation_features.clone(),
        })
        .collect::<Vec<_>>();
    reusable_priors.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.success_count.cmp(&left.success_count))
    });
    reusable_priors.truncate(12);
    Ok(FitLearningSummary {
        case_count: cases.cases.len(),
        promoted_case_count: cases
            .cases
            .iter()
            .filter(|case| case.passes_promotion_gates())
            .count(),
        prior_count: priors.priors.len(),
        exact_prior_count: priors
            .priors
            .iter()
            .filter(|prior| prior.scope == FitPriorScope::ExactComponent)
            .count(),
        cross_component_prior_count: priors
            .priors
            .iter()
            .filter(|prior| prior.scope == FitPriorScope::CrossComponent)
            .count(),
        reusable_priors,
    })
}
