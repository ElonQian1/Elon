use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use super::{
    receipt_is_within_age,
    source_test_support::{
        fixture, merchant_names, request, CATALOG_SEARCH, DISABLED_LINK_NAME, FUTURE_NAME,
        OLD_NAME, RECENT_NAME, STALE_LINK_NAME, STATIC_NAME,
    },
};
use crate::open_commerce_consumer_model::{ConsumerDiscoveryRequest, ConsumerSourceFilterOption};

#[test]
fn source_requirement_and_declaration_freshness_compose_without_hiding_defaults() {
    let fixture = fixture();
    let omitted: ConsumerDiscoveryRequest = serde_json::from_value(json!({})).unwrap();
    assert!(!omitted.require_internal_sync_receipt);
    assert!(omitted.source_provider_key.is_none());
    assert!(omitted.source_data_domain.is_none());
    assert!(omitted.max_source_age_seconds.is_none());

    let default = fixture.discover(request()).unwrap();
    assert_eq!(default.source_requirement, "any_merchant_source");
    assert_eq!(
        merchant_names(&default),
        sorted_names(&[
            STATIC_NAME,
            RECENT_NAME,
            OLD_NAME,
            FUTURE_NAME,
            STALE_LINK_NAME,
            DISABLED_LINK_NAME,
        ])
    );
    assert_source_kind(
        &default,
        &fixture.static_merchant_id,
        "merchant_static_data",
    );
    assert_source_kind(
        &default,
        &fixture.recent_merchant_id,
        "merchant_static_data",
    );
    assert_source_kind(
        &default,
        &fixture.stale_link_merchant_id,
        "merchant_static_data",
    );
    assert_source_kind(
        &default,
        &fixture.disabled_link_merchant_id,
        "merchant_static_data",
    );

    let mut current_request = request();
    current_request.require_current_declaration = true;
    let current = fixture.discover(current_request).unwrap();
    assert_eq!(
        merchant_names(&current),
        sorted_names(&[
            STATIC_NAME,
            RECENT_NAME,
            FUTURE_NAME,
            STALE_LINK_NAME,
            DISABLED_LINK_NAME,
        ])
    );

    let mut internal_request = request();
    internal_request.require_internal_sync_receipt = true;
    let internal = fixture.discover(internal_request).unwrap();
    assert_eq!(internal.source_requirement, "internal_sync_receipt");
    assert_eq!(
        merchant_names(&internal),
        sorted_names(&[RECENT_NAME, OLD_NAME, FUTURE_NAME])
    );
    assert!(internal.matches.iter().all(|item| item
        .reasons
        .iter()
        .any(|reason| reason == "已关联商户项目内部业务同步回执")));

    let mut both_request = request();
    both_request.require_current_declaration = true;
    both_request.require_internal_sync_receipt = true;
    let both = fixture.discover(both_request).unwrap();
    assert_eq!(
        merchant_names(&both),
        sorted_names(&[RECENT_NAME, FUTURE_NAME])
    );

    for ranking_policy in [
        "transparent_preference_match.v1",
        "lowest_unit_price.v1",
        "public_access_first.v1",
        "recently_updated.v1",
        "merchant_name.v1",
    ] {
        let mut ranked_request = request();
        ranked_request.require_internal_sync_receipt = true;
        ranked_request.capability_key = Some(CATALOG_SEARCH.to_string());
        ranked_request.ranking_policy = Some(ranking_policy.to_string());
        let ranked = fixture.discover(ranked_request).unwrap();
        assert_eq!(ranked.ranking_policy, ranking_policy);
        assert_eq!(ranked.candidate_scope.eligible_match_count, 3);
        assert_eq!(ranked.matches.len(), 3);
    }
}

#[test]
fn provider_and_domain_filters_are_exact_normalized_and_fail_closed() {
    let fixture = fixture();
    let mut filtered_request = request();
    filtered_request.source_provider_key = Some("  ALPHA_ERP  ".to_string());
    filtered_request.source_data_domain = Some("  CATALOG  ".to_string());
    filtered_request.capability_key = Some("  CATALOG.SEARCH  ".to_string());
    filtered_request.price_currency = Some(" cny ".to_string());
    filtered_request.capability_kind = Some(" query ".to_string());
    filtered_request.access_level = Some(" public ".to_string());
    filtered_request.preferences.city = Some("吉安".to_string());
    filtered_request.preferences.categories = vec!["retail".to_string()];
    filtered_request.preferences.tags = vec!["open".to_string()];
    filtered_request.preferences.max_unit_price_micros = Some(0);
    filtered_request.require_current_declaration = true;
    filtered_request.require_city_match = true;
    filtered_request.require_category_match = true;
    filtered_request.require_all_tags_match = true;
    let filtered = fixture.discover(filtered_request).unwrap();

    assert_eq!(merchant_names(&filtered), vec![RECENT_NAME.to_string()]);
    assert_eq!(filtered.source_requirement, "internal_sync_receipt");
    assert_eq!(
        filtered.source_filter.provider_key.as_deref(),
        Some("alpha_erp")
    );
    assert_eq!(
        filtered.source_filter.data_domain.as_deref(),
        Some("catalog")
    );
    assert_eq!(filtered.capability_filter.kind.as_deref(), Some("query"));
    assert_eq!(
        filtered.capability_filter.access_level.as_deref(),
        Some("public")
    );
    assert!(filtered.preference_constraints.require_city_match);
    assert!(filtered.preference_constraints.require_category_match);
    assert!(filtered.preference_constraints.require_all_tags_match);
    let matched = &filtered.matches[0];
    assert_eq!(
        matched.capability.source.provider_key.as_deref(),
        Some("alpha_erp")
    );
    assert_eq!(
        matched.capability.source.data_domain.as_deref(),
        Some("catalog")
    );
    assert!(matched
        .reasons
        .iter()
        .any(|reason| reason == "来源厂商标识匹配 alpha_erp"));
    assert!(matched
        .reasons
        .iter()
        .any(|reason| reason == "来源数据域匹配 catalog"));

    let mut impossible = request();
    impossible.source_provider_key = Some("beta_erp".to_string());
    impossible.source_data_domain = Some("catalog".to_string());
    assert!(fixture.discover(impossible).unwrap().matches.is_empty());

    let mut blanks = request();
    blanks.source_provider_key = Some("  ".to_string());
    blanks.source_data_domain = Some("\t".to_string());
    let blanks = fixture.discover(blanks).unwrap();
    assert_eq!(blanks.source_requirement, "any_merchant_source");
    assert_eq!(blanks.matches.len(), 6);

    for (provider, domain, expected) in [
        (Some("bad provider"), None, "平台标识"),
        (None, Some("bad domain"), "来源数据域"),
        (Some("x"), None, "平台标识"),
    ] {
        let mut invalid = request();
        invalid.source_provider_key = provider.map(str::to_string);
        invalid.source_data_domain = domain.map(str::to_string);
        assert!(fixture
            .discover(invalid)
            .unwrap_err()
            .to_string()
            .contains(expected));
    }
}

#[test]
fn receipt_age_bounds_and_invalid_timestamps_fail_closed() {
    let fixture = fixture();
    let mut recent_request = request();
    recent_request.max_source_age_seconds = Some(120);
    let recent = fixture.discover(recent_request).unwrap();
    assert_eq!(merchant_names(&recent), vec![RECENT_NAME.to_string()]);
    assert_eq!(recent.source_requirement, "internal_sync_receipt");
    assert_eq!(recent.source_filter.max_age_seconds, Some(120));
    assert!(recent.matches[0]
        .reasons
        .iter()
        .any(|reason| reason == "内部同步回执完成时间不超过 120 秒"));

    let mut one_second_request = request();
    one_second_request.max_source_age_seconds = Some(1);
    assert!(fixture
        .discover(one_second_request)
        .unwrap()
        .matches
        .is_empty());

    let mut year_request = request();
    year_request.max_source_age_seconds = Some(31_536_000);
    assert_eq!(
        merchant_names(&fixture.discover(year_request).unwrap()),
        sorted_names(&[RECENT_NAME, OLD_NAME])
    );

    for invalid_age in [0, -1, 31_536_001] {
        let mut invalid = request();
        invalid.max_source_age_seconds = Some(invalid_age);
        assert!(fixture
            .discover(invalid)
            .unwrap_err()
            .to_string()
            .contains("1 秒到 365 天"));
    }

    let discovery_time = DateTime::parse_from_rfc3339("2026-08-10T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let mut capability = fixture.directory_capability(&fixture.recent_merchant_id, CATALOG_SEARCH);
    capability.source.receipt_completed_at = Some("2026-08-09T23:59:00Z".to_string());
    assert!(receipt_is_within_age(&capability, 60, &discovery_time));
    capability.source.receipt_completed_at = Some("2026-08-09T23:58:59Z".to_string());
    assert!(!receipt_is_within_age(&capability, 60, &discovery_time));
    capability.source.receipt_completed_at = Some("2026-08-10T00:00:01Z".to_string());
    assert!(!receipt_is_within_age(&capability, 60, &discovery_time));
    capability.source.receipt_completed_at = Some("not-a-timestamp".to_string());
    assert!(!receipt_is_within_age(&capability, 60, &discovery_time));
    capability.source.receipt_completed_at = None;
    assert!(!receipt_is_within_age(&capability, 60, &discovery_time));
    capability.source.kind = "merchant_static_data".to_string();
    capability.source.receipt_completed_at = Some("2026-08-10T00:00:00Z".to_string());
    assert!(!receipt_is_within_age(&capability, 60, &discovery_time));
}

#[test]
fn source_filter_options_count_capabilities_without_obeying_active_source_filters() {
    let fixture = fixture();
    let unfiltered = fixture.discover(request()).unwrap();
    assert_eq!(
        option_pairs(&unfiltered.source_filter_options.providers),
        vec![
            ("alpha_erp".to_string(), 2),
            ("beta_erp".to_string(), 1),
            ("future_erp".to_string(), 1),
        ]
    );
    assert_eq!(
        option_pairs(&unfiltered.source_filter_options.data_domains),
        vec![("catalog".to_string(), 3), ("inventory".to_string(), 1)]
    );
    assert_eq!(
        unfiltered.source_filter_options.scope,
        "current_operator_candidate_window.v1"
    );
    assert!(!unfiltered.source_filter_options.operator_exhaustive);

    let mut active_filters = request();
    active_filters.source_provider_key = Some("alpha_erp".to_string());
    active_filters.source_data_domain = Some("catalog".to_string());
    active_filters.max_source_age_seconds = Some(120);
    let active_filters = fixture.discover(active_filters).unwrap();
    assert_eq!(
        merchant_names(&active_filters),
        vec![RECENT_NAME.to_string()]
    );
    assert_eq!(
        option_pairs(&active_filters.source_filter_options.providers),
        option_pairs(&unfiltered.source_filter_options.providers)
    );
    assert_eq!(
        option_pairs(&active_filters.source_filter_options.data_domains),
        option_pairs(&unfiltered.source_filter_options.data_domains)
    );

    let mut capability_request = request();
    capability_request.capability_key = Some(CATALOG_SEARCH.to_string());
    let capability_response = fixture.discover(capability_request).unwrap();
    assert_eq!(
        option_pairs(&capability_response.source_filter_options.providers),
        vec![
            ("alpha_erp".to_string(), 1),
            ("beta_erp".to_string(), 1),
            ("future_erp".to_string(), 1),
        ]
    );
    assert_eq!(
        option_pairs(&capability_response.source_filter_options.data_domains),
        vec![("catalog".to_string(), 2), ("inventory".to_string(), 1)]
    );

    let serialized = serde_json::to_string(&unfiltered.source_filter_options).unwrap();
    for private_field in [
        "integration_receipt_id",
        "sync_receipt_id",
        "cursor",
        "scopes",
        "merchant_id",
    ] {
        assert!(!serialized.contains(private_field));
    }
}

#[test]
fn ranking_receipt_commits_normalized_filters_and_discovery_is_read_only() {
    let fixture = fixture();
    let before = fixture.snapshot();
    let mut filtered_request = request();
    filtered_request.include_ranking_receipt = true;
    filtered_request.source_provider_key = Some(" ALPHA_ERP ".to_string());
    filtered_request.source_data_domain = Some(" CATALOG ".to_string());
    filtered_request.max_source_age_seconds = Some(120);
    let filtered = fixture.discover(filtered_request).unwrap();
    let payload = ranking_payload(&filtered);

    assert_eq!(payload["source_requirement"], "internal_sync_receipt");
    assert_eq!(payload["source_filter"]["provider_key"], "alpha_erp");
    assert_eq!(payload["source_filter"]["data_domain"], "catalog");
    assert_eq!(payload["source_filter"]["max_age_seconds"], 120);
    assert_eq!(payload["ordered_results"].as_array().unwrap().len(), 1);
    assert_eq!(
        payload["ordered_results"][0]["source"]["externally_verified"],
        false
    );
    let first_fingerprint = payload["request_fingerprint_sha256"]
        .as_str()
        .unwrap()
        .to_string();

    let mut changed_request = request();
    changed_request.include_ranking_receipt = true;
    changed_request.source_provider_key = Some("alpha_erp".to_string());
    changed_request.source_data_domain = Some("catalog".to_string());
    changed_request.max_source_age_seconds = Some(121);
    let changed = fixture.discover(changed_request).unwrap();
    let changed_payload = ranking_payload(&changed);
    assert_ne!(
        first_fingerprint,
        changed_payload["request_fingerprint_sha256"]
            .as_str()
            .unwrap()
    );
    assert_eq!(before, fixture.snapshot());
}

#[test]
fn invalidated_sources_are_absent_from_filters_and_options() {
    let fixture = fixture();
    let response = fixture.discover(request()).unwrap();
    for merchant_id in [
        &fixture.stale_link_merchant_id,
        &fixture.disabled_link_merchant_id,
    ] {
        let matched = response
            .matches
            .iter()
            .find(|item| &item.merchant.id == merchant_id)
            .unwrap();
        assert_eq!(matched.capability.source.kind, "merchant_static_data");
        assert!(matched.capability.source.provider_key.is_none());
        assert!(matched.capability.source.data_domain.is_none());
        assert!(matched.capability.source.integration_receipt_id.is_none());
    }
    let providers = option_pairs(&response.source_filter_options.providers);
    assert!(!providers.iter().any(|(value, _)| value == "stale_erp"));
    assert!(!providers.iter().any(|(value, _)| value == "disabled_erp"));

    let mut required = request();
    required.require_internal_sync_receipt = true;
    let required = fixture.discover(required).unwrap();
    assert!(!required.matches.iter().any(|item| {
        item.merchant.id == fixture.stale_link_merchant_id
            || item.merchant.id == fixture.disabled_link_merchant_id
    }));
}

fn sorted_names(values: &[&str]) -> Vec<String> {
    let mut values = values
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    values.sort();
    values
}

fn assert_source_kind(
    response: &crate::open_commerce_consumer_model::ConsumerDiscoveryResponse,
    merchant_id: &str,
    expected: &str,
) {
    let matched = response
        .matches
        .iter()
        .find(|item| item.merchant.id == merchant_id)
        .unwrap();
    assert_eq!(matched.capability.source.kind, expected);
}

fn option_pairs(values: &[ConsumerSourceFilterOption]) -> Vec<(String, usize)> {
    values
        .iter()
        .map(|value| (value.value.clone(), value.capability_count))
        .collect()
}

fn ranking_payload(
    response: &crate::open_commerce_consumer_model::ConsumerDiscoveryResponse,
) -> Value {
    serde_json::from_str(
        &response
            .ranking_receipt
            .as_ref()
            .unwrap()
            .canonical_payload_json,
    )
    .unwrap()
}
