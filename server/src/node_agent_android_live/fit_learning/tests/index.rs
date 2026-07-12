use super::super::prior_index::FitPriorIndex;
use super::super::types::{FitPriorQuery, FitPriorScope, FitTranslationFeatures};
use super::fit_case;

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
    assert!((cross.median_factors["height"] - 1.125).abs() < 0.000_001);
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

fn count_scope(index: &FitPriorIndex, scope: FitPriorScope) -> usize {
    index
        .priors()
        .iter()
        .filter(|prior| prior.scope == scope)
        .count()
}
