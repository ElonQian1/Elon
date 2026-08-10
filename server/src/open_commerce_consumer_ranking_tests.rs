use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::ranking_test_support::{capability, fixture, merchant, names, request, MerchantSpec};
use crate::{
    open_commerce_consumer_model::ConsumerDiscoveryRequest,
    open_commerce_model::{ACCESS_AUTHORIZED, ACCESS_PUBLIC},
};

const POLICIES: [&str; 5] = [
    "transparent_preference_match.v1",
    "lowest_unit_price.v1",
    "public_access_first.v1",
    "recently_updated.v1",
    "merchant_name.v1",
];

#[test]
fn five_ranking_policies_are_explicit_stable_non_paid_and_fail_closed() {
    let fixture = fixture();
    let alpha = fixture.publish(merchant(
        "Alpha Cafe",
        "alpha-cafe",
        json!({"category":"Cafe","city":"JiAn","tags":["Quiet"]}),
        vec![capability("alpha.menu", ACCESS_AUTHORIZED, 300, 86_400)],
    ));
    let beta = fixture.publish(merchant(
        "Beta Retail",
        "beta-retail",
        json!({"category":"Retail","city":"NanChang","tags":[]}),
        vec![capability("beta.menu", ACCESS_PUBLIC, 200, 86_400)],
    ));
    let gamma = fixture.publish(merchant(
        "Gamma Cafe",
        "gamma-cafe",
        json!({"category":"Cafe","city":"JiAn","tags":["Quiet"]}),
        vec![capability("gamma.menu", ACCESS_PUBLIC, 100, 86_400)],
    ));
    fixture.set_capability_time(&alpha, "alpha.menu", "2026-01-01T00:00:00Z");
    fixture.set_capability_time(&beta, "beta.menu", "2026-03-01T00:00:00Z");
    fixture.set_capability_time(&gamma, "gamma.menu", "2026-02-01T00:00:00Z");
    let before = fixture.snapshot();

    let mut base = request();
    base.preferences.categories = vec!["cafe".to_string()];
    base.preferences.tags = vec!["quiet".to_string()];
    base.preferences.city = Some("jian".to_string());
    base.preferences.prefer_public = true;
    let expected = [
        ["Gamma Cafe", "Alpha Cafe", "Beta Retail"],
        ["Gamma Cafe", "Beta Retail", "Alpha Cafe"],
        ["Gamma Cafe", "Beta Retail", "Alpha Cafe"],
        ["Beta Retail", "Gamma Cafe", "Alpha Cafe"],
        ["Alpha Cafe", "Beta Retail", "Gamma Cafe"],
    ];

    let default = fixture.discover(clone_request(&base)).unwrap();
    assert_eq!(default.ranking_policy, POLICIES[0]);
    assert!(!default.ranking_is_user_selected);
    assert_eq!(names(&default), strings(&expected[0]));

    let mut blank = clone_request(&base);
    blank.ranking_policy = Some("   ".to_string());
    let blank = fixture.discover(blank).unwrap();
    assert_eq!(blank.ranking_policy, POLICIES[0]);
    assert!(!blank.ranking_is_user_selected);
    assert_eq!(names(&blank), strings(&expected[0]));

    for (policy, expected_names) in POLICIES.iter().zip(expected.iter()) {
        let mut ranked = clone_request(&base);
        ranked.ranking_policy = Some((*policy).to_string());
        let first = fixture.discover(ranked).unwrap();
        assert_eq!(first.ranking_policy, *policy);
        assert!(first.ranking_is_user_selected);
        assert!(!first.ranking_is_paid);
        assert_eq!(names(&first), strings(expected_names));
        assert_eq!(first.available_ranking_policies.len(), 5);
        assert_eq!(
            first
                .available_ranking_policies
                .iter()
                .map(|item| item.key.as_str())
                .collect::<Vec<_>>(),
            POLICIES
        );
        assert!(first
            .available_ranking_policies
            .iter()
            .all(|item| !item.paid_placement
                && !item.label.is_empty()
                && !item.explanation.is_empty()));

        let mut repeat = clone_request(&base);
        repeat.ranking_policy = Some((*policy).to_string());
        assert_eq!(names(&fixture.discover(repeat).unwrap()), names(&first));
    }

    let mut unknown = clone_request(&base);
    unknown.ranking_policy = Some("paid-secret.v1".to_string());
    assert!(fixture
        .discover(unknown)
        .unwrap_err()
        .to_string()
        .contains("排序策略不受支持"));
    assert_eq!(before, fixture.snapshot());
}

#[test]
fn capability_selection_uses_price_before_stable_key_for_equal_primary_values() {
    let fixture = fixture();
    let merchant_id = fixture.publish(merchant(
        "Capability Choice",
        "capability-choice",
        json!({"category":"Cafe"}),
        vec![
            capability("a.expensive", ACCESS_PUBLIC, 90_000, 86_400),
            capability("z.cheap", ACCESS_PUBLIC, 10_000, 86_400),
        ],
    ));
    let tied_at = "2026-01-01T00:00:00Z";
    fixture.set_capability_time(&merchant_id, "a.expensive", tied_at);
    fixture.set_capability_time(&merchant_id, "z.cheap", tied_at);

    for policy in POLICIES {
        let mut ranked = request();
        ranked.ranking_policy = Some(policy.to_string());
        let response = fixture.discover(ranked).unwrap();
        assert_eq!(response.matches.len(), 1);
        assert_eq!(response.matches[0].capability.capability_key, "z.cheap");
        assert_eq!(response.matches[0].capability.unit_price_micros, 10_000);
    }
}

#[test]
fn candidate_scope_counts_truncation_receipt_and_tied_directory_window_are_stable() {
    let fixture = fixture();
    let tied_at = "2026-01-01T00:00:00Z";
    let mut merchants = Vec::new();
    for index in 0..105 {
        let display_name = format!("Candidate {index:03}");
        let slug = format!("candidate-{index:03}");
        let key = format!("candidate.{index:03}");
        let merchant_id = fixture.publish(merchant(
            &display_name,
            &slug,
            json!({"category":"candidate"}),
            vec![capability(&key, ACCESS_PUBLIC, 0, 86_400)],
        ));
        fixture.set_directory_time(&merchant_id, tied_at);
        merchants.push((merchant_id, display_name));
    }
    let before = fixture.snapshot();
    merchants.sort_by(|left, right| left.0.cmp(&right.0));
    let mut expected_window = merchants
        .iter()
        .take(100)
        .map(|(_, name)| name.clone())
        .collect::<Vec<_>>();
    expected_window.sort();

    let mut fifty = request();
    fifty.ranking_policy = Some("merchant_name.v1".to_string());
    fifty.include_ranking_receipt = true;
    let fifty = fixture.discover(fifty).unwrap();
    assert_eq!(names(&fifty), expected_window[..50]);
    assert_eq!(fifty.candidate_scope.candidate_cap, 100);
    assert_eq!(fifty.candidate_scope.directory_candidate_count, 100);
    assert_eq!(fifty.candidate_scope.eligible_match_count, 100);
    assert_eq!(fifty.candidate_scope.returned_match_count, 50);
    assert!(fifty.candidate_scope.results_truncated);

    let payload = receipt_payload(&fifty);
    assert_eq!(payload["candidate_scope"]["candidate_cap"], 100);
    assert_eq!(payload["candidate_scope"]["directory_candidate_count"], 100);
    assert_eq!(payload["eligible_match_count"], 100);
    assert_eq!(payload["returned_match_count"], 50);

    let mut one = request();
    one.ranking_policy = Some("merchant_name.v1".to_string());
    one.limit = 1;
    let one = fixture.discover(one).unwrap();
    assert_eq!(names(&one), vec![expected_window[0].clone()]);
    assert_eq!(one.candidate_scope.directory_candidate_count, 100);
    assert!(one.candidate_scope.results_truncated);

    let mut exact = request();
    exact.query = Some(expected_window[0].clone());
    let exact = fixture.discover(exact).unwrap();
    assert_eq!(exact.candidate_scope.directory_candidate_count, 1);
    assert_eq!(exact.candidate_scope.eligible_match_count, 1);
    assert_eq!(exact.candidate_scope.returned_match_count, 1);
    assert!(!exact.candidate_scope.results_truncated);

    let mut empty = request();
    empty.query = Some("no-such-directory-entry".to_string());
    let empty = fixture.discover(empty).unwrap();
    assert_eq!(empty.candidate_scope.directory_candidate_count, 0);
    assert_eq!(empty.candidate_scope.eligible_match_count, 0);
    assert_eq!(empty.candidate_scope.returned_match_count, 0);
    assert!(!empty.candidate_scope.results_truncated);
    assert_eq!(before, fixture.snapshot());
}

#[test]
fn current_declaration_filter_is_opt_in_composable_and_committed_to_receipt() {
    let fixture = fixture();
    let mixed = fixture.publish(merchant(
        "Mixed Freshness",
        "mixed-freshness",
        json!({"category":"freshness"}),
        vec![
            capability("a.current", ACCESS_PUBLIC, 0, 86_400),
            capability("b.stale", ACCESS_PUBLIC, 0, 1),
            capability("c.unknown", ACCESS_PUBLIC, 0, 0),
        ],
    ));
    let stale = fixture.publish(merchant(
        "Stale Only",
        "stale-only",
        json!({"category":"freshness"}),
        vec![capability("b.stale", ACCESS_PUBLIC, 0, 1)],
    ));
    let unknown = fixture.publish(merchant(
        "Unknown Only",
        "unknown-only",
        json!({"category":"freshness"}),
        vec![capability("c.unknown", ACCESS_PUBLIC, 0, 0)],
    ));
    let now = Utc::now().to_rfc3339();
    fixture.set_capability_time(&mixed, "a.current", &now);
    fixture.set_capability_time(&mixed, "b.stale", "2020-01-01T00:00:00Z");
    fixture.set_capability_time(&mixed, "c.unknown", &now);
    fixture.set_capability_time(&stale, "b.stale", "2020-01-01T00:00:00Z");
    fixture.set_capability_time(&unknown, "c.unknown", &now);
    let before = fixture.snapshot();

    let omitted: ConsumerDiscoveryRequest = serde_json::from_value(json!({})).unwrap();
    assert!(!omitted.require_current_declaration);
    let default = fixture.discover(request()).unwrap();
    assert_eq!(default.freshness_requirement, "any_declaration");
    assert_eq!(
        sorted(names(&default)),
        strings(&["Mixed Freshness", "Stale Only", "Unknown Only"])
    );

    let mut current_request = request();
    current_request.require_current_declaration = true;
    current_request.include_ranking_receipt = true;
    let current = fixture.discover(current_request).unwrap();
    assert_eq!(current.freshness_requirement, "current_declaration");
    assert_eq!(names(&current), vec!["Mixed Freshness".to_string()]);
    assert_eq!(current.matches[0].capability.capability_key, "a.current");
    assert!(current.matches[0]
        .reasons
        .iter()
        .any(|reason| reason == "符合消费者要求的商户声明有效期"));
    let current_payload = receipt_payload(&current);
    assert_eq!(
        current_payload["freshness_requirement"],
        "current_declaration"
    );

    let mut unfiltered_receipt = request();
    unfiltered_receipt.include_ranking_receipt = true;
    let unfiltered = fixture.discover(unfiltered_receipt).unwrap();
    let unfiltered_payload = receipt_payload(&unfiltered);
    assert_eq!(
        unfiltered_payload["freshness_requirement"],
        "any_declaration"
    );
    assert_ne!(
        current_payload["request_fingerprint_sha256"],
        unfiltered_payload["request_fingerprint_sha256"]
    );

    for policy in POLICIES {
        let mut ranked = request();
        ranked.ranking_policy = Some(policy.to_string());
        ranked.require_current_declaration = true;
        assert_eq!(
            names(&fixture.discover(ranked).unwrap()),
            vec!["Mixed Freshness"]
        );
    }

    let mut stale_key = request();
    stale_key.capability_key = Some("b.stale".to_string());
    stale_key.require_current_declaration = true;
    assert!(fixture.discover(stale_key).unwrap().matches.is_empty());

    let mut current_key = request();
    current_key.capability_key = Some("a.current".to_string());
    current_key.require_current_declaration = true;
    assert_eq!(
        names(&fixture.discover(current_key).unwrap()),
        vec!["Mixed Freshness"]
    );
    assert_eq!(before, fixture.snapshot());
}

#[test]
fn ranking_receipt_hashes_exact_bytes_hides_request_values_and_stays_read_only() {
    let fixture = fixture();
    let secret_query = "secret-search-fragment";
    let merchant_id = fixture.publish(MerchantSpec {
        display_name: "Receipt Merchant",
        slug: "receipt-merchant",
        description: secret_query,
        public_profile: json!({
            "category":"secret-category",
            "city":"secret-city",
            "tags":["secret-tag"]
        }),
        capabilities: vec![capability("receipt.lookup", ACCESS_PUBLIC, 7, 86_400)],
    });

    let mut disabled = request();
    disabled.query = Some(secret_query.to_string());
    assert!(fixture
        .discover(disabled)
        .unwrap()
        .ranking_receipt
        .is_none());

    let before = fixture.snapshot();
    let mut enabled = request();
    enabled.query = Some(secret_query.to_string());
    enabled.include_ranking_receipt = true;
    enabled.preferences.categories = vec!["secret-category".to_string()];
    enabled.preferences.tags = vec!["secret-tag".to_string()];
    enabled.preferences.city = Some("secret-city".to_string());
    let response = fixture.discover(enabled).unwrap();
    let receipt = response.ranking_receipt.as_ref().unwrap();
    assert_eq!(receipt.schema, "open_commerce.consumer_ranking_receipt.v1");
    assert_eq!(receipt.hash_algorithm, "sha256");
    assert!(!receipt.signed_by_operator);
    assert_eq!(
        receipt.payload_sha256,
        sha256_hex(&receipt.canonical_payload_json)
    );
    for private_value in [
        secret_query,
        "secret-category",
        "secret-tag",
        "secret-city",
        "pc-web",
    ] {
        assert!(!receipt.canonical_payload_json.contains(private_value));
    }
    let payload = receipt_payload(&response);
    assert_eq!(payload["ranking"]["paid_placement"], false);
    assert_eq!(payload["candidate_scope"]["operator_exhaustive"], false);
    assert_eq!(payload["candidate_scope"]["candidate_cap"], 100);
    assert_eq!(payload["ordered_results"].as_array().unwrap().len(), 1);
    assert_eq!(payload["ordered_results"][0]["merchant_id"], merchant_id);
    assert!(payload["ordered_results"][0].get("source").is_some());
    assert!(payload["ordered_results"][0].get("freshness").is_some());
    DateTime::parse_from_rfc3339(payload["generated_at"].as_str().unwrap()).unwrap();
    assert_eq!(before, fixture.snapshot());
}

fn clone_request(request: &ConsumerDiscoveryRequest) -> ConsumerDiscoveryRequest {
    request.clone()
}

fn receipt_payload(
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

fn sha256_hex(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn strings<const N: usize>(values: &[&str; N]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}
