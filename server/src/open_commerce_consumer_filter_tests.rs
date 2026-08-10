use serde_json::Value;

use super::filter_test_support::{
    fixture, names, request, CONSTRAINT_CAPABILITY, CONSTRAINT_EXACT, CONSTRAINT_MALFORMED,
    CONSTRAINT_PARTIAL_TAGS, CONSTRAINT_WRONG_CATEGORY, CONSTRAINT_WRONG_CITY,
    MATRIX_ACTION_AUTHORIZED, MATRIX_ACTION_PUBLIC, MATRIX_CAPABILITY, MATRIX_QUERY_AUTHORIZED,
    MATRIX_QUERY_PUBLIC, PRICE_CAPABILITY, PRICE_CNY_ABOVE, PRICE_CNY_BELOW, PRICE_CNY_EQUAL,
    PRICE_EUR, PRICE_USD,
};

#[test]
fn price_filter_is_currency_safe_bounded_and_compatible_with_cny_default() {
    let fixture = fixture();
    let default = fixture.discover(request("Price")).unwrap();
    assert_eq!(default.price_filter.currency, None);
    assert_eq!(default.price_filter.max_unit_price_micros, None);
    assert_eq!(default.matches.len(), 5);

    let mut usd_request = request("Price");
    usd_request.price_currency = Some("  usd  ".to_string());
    let usd = fixture.discover(usd_request).unwrap();
    assert_eq!(names(&usd), vec![PRICE_USD.to_string()]);
    assert_eq!(usd.price_filter.currency.as_deref(), Some("USD"));
    assert_eq!(usd.price_filter.max_unit_price_micros, None);
    assert!(usd.matches[0]
        .reasons
        .iter()
        .any(|reason| reason == "调用价币种匹配 USD"));

    let mut compatible_cny = request("Price");
    compatible_cny.preferences.max_unit_price_micros = Some(500);
    let compatible_cny = fixture.discover(compatible_cny).unwrap();
    assert_eq!(compatible_cny.price_filter.currency.as_deref(), Some("CNY"));
    assert_eq!(
        names(&compatible_cny),
        sorted_names(&[PRICE_CNY_BELOW, PRICE_CNY_EQUAL])
    );
    assert!(compatible_cny.matches.iter().all(|item| item
        .reasons
        .iter()
        .any(|reason| reason == "调用价不超过 500 微单位 CNY")));

    let mut explicit_eur = request("Price");
    explicit_eur.price_currency = Some("eur".to_string());
    explicit_eur.preferences.max_unit_price_micros = Some(1);
    assert_eq!(
        names(&fixture.discover(explicit_eur).unwrap()),
        vec![PRICE_EUR.to_string()]
    );

    for invalid_currency in ["US", "USDT", "12A", "€UR"] {
        let mut invalid = request("Price");
        invalid.price_currency = Some(invalid_currency.to_string());
        assert!(fixture
            .discover(invalid)
            .unwrap_err()
            .to_string()
            .contains("三位字母代码"));
    }
    for invalid_price in [-1, 1_000_000_000_000_001] {
        let mut invalid = request("Price");
        invalid.preferences.max_unit_price_micros = Some(invalid_price);
        assert!(fixture
            .discover(invalid)
            .unwrap_err()
            .to_string()
            .contains("最大调用价格超出允许范围"));
    }

    for ranking_policy in [
        "transparent_preference_match.v1",
        "lowest_unit_price.v1",
        "public_access_first.v1",
        "recently_updated.v1",
        "merchant_name.v1",
    ] {
        let mut ranked = request("Price");
        ranked.capability_key = Some(PRICE_CAPABILITY.to_string());
        ranked.price_currency = Some("CNY".to_string());
        ranked.preferences.max_unit_price_micros = Some(500);
        ranked.ranking_policy = Some(ranking_policy.to_string());
        let ranked = fixture.discover(ranked).unwrap();
        assert_eq!(ranked.ranking_policy, ranking_policy);
        assert_eq!(ranked.candidate_scope.eligible_match_count, 2);
        assert_eq!(ranked.matches.len(), 2);
    }
}

#[test]
fn capability_kind_and_access_level_cover_all_public_discovery_combinations() {
    let fixture = fixture();
    for (kind, access_level, expected_name, authorization_status) in [
        ("query", "public", MATRIX_QUERY_PUBLIC, "not_required"),
        (
            "query",
            "authorized",
            MATRIX_QUERY_AUTHORIZED,
            "app_registration_required",
        ),
        ("action", "public", MATRIX_ACTION_PUBLIC, "not_required"),
        (
            "action",
            "authorized",
            MATRIX_ACTION_AUTHORIZED,
            "app_registration_required",
        ),
    ] {
        let mut filtered = request("Capability");
        filtered.capability_key = Some(MATRIX_CAPABILITY.to_string());
        filtered.capability_kind = Some(format!("  {kind}  "));
        filtered.access_level = Some(format!("  {access_level}  "));
        let filtered = fixture.discover(filtered).unwrap();
        assert_eq!(names(&filtered), vec![expected_name.to_string()]);
        assert_eq!(filtered.capability_filter.kind.as_deref(), Some(kind));
        assert_eq!(
            filtered.capability_filter.access_level.as_deref(),
            Some(access_level)
        );
        assert_eq!(filtered.matches[0].capability.kind, kind);
        assert_eq!(filtered.matches[0].capability.access_level, access_level);
        assert_eq!(
            filtered.matches[0].authorization.status,
            authorization_status
        );
        assert!(filtered.matches[0]
            .reasons
            .iter()
            .any(|reason| reason == &format!("能力类型匹配 {kind}")));
        assert!(filtered.matches[0]
            .reasons
            .iter()
            .any(|reason| reason == &format!("访问级别匹配 {access_level}")));
    }

    for (kind, access_level, expected) in [
        (Some("QUERY"), None, "能力类型"),
        (Some("unknown"), None, "能力类型"),
        (None, Some("PUBLIC"), "访问级别"),
        (None, Some("unknown"), "访问级别"),
        (None, Some("owner_only"), "不支持 owner_only"),
    ] {
        let mut invalid = request("Capability");
        invalid.capability_kind = kind.map(str::to_string);
        invalid.access_level = access_level.map(str::to_string);
        assert!(fixture
            .discover(invalid)
            .unwrap_err()
            .to_string()
            .contains(expected));
    }

    let mut impossible = request("Price");
    impossible.capability_key = Some(PRICE_CAPABILITY.to_string());
    impossible.capability_kind = Some("action".to_string());
    assert!(fixture.discover(impossible).unwrap().matches.is_empty());

    let mut owner_only_key = request("Capability");
    owner_only_key.capability_key = Some("matrix.owner".to_string());
    assert!(fixture.discover(owner_only_key).unwrap().matches.is_empty());
}

#[test]
fn preference_constraints_keep_soft_defaults_and_fail_closed_on_missing_profiles() {
    let fixture = fixture();
    let preferences = || crate::open_commerce_consumer_model::ConsumerPreferences {
        categories: vec!["  cAFE  ".to_string(), "cafe".to_string()],
        tags: vec![
            " quiet ".to_string(),
            "WIFI".to_string(),
            "vegan".to_string(),
        ],
        city: Some("  jIaN  ".to_string()),
        max_unit_price_micros: None,
        prefer_public: false,
    };

    let mut soft = request("Constraint");
    soft.preferences = preferences();
    let soft = fixture.discover(soft).unwrap();
    assert_eq!(soft.matches.len(), 5);
    assert_eq!(soft.matches[0].merchant.display_name, CONSTRAINT_EXACT);
    assert!(!soft.preference_constraints.require_city_match);
    assert!(!soft.preference_constraints.require_category_match);
    assert!(!soft.preference_constraints.require_all_tags_match);

    let mut city = request("Constraint");
    city.preferences = preferences();
    city.require_city_match = true;
    assert_eq!(
        names(&fixture.discover(city).unwrap()),
        sorted_names(&[
            CONSTRAINT_EXACT,
            CONSTRAINT_WRONG_CATEGORY,
            CONSTRAINT_PARTIAL_TAGS,
        ])
    );

    let mut category = request("Constraint");
    category.preferences = preferences();
    category.require_category_match = true;
    assert_eq!(
        names(&fixture.discover(category).unwrap()),
        sorted_names(&[
            CONSTRAINT_EXACT,
            CONSTRAINT_WRONG_CITY,
            CONSTRAINT_PARTIAL_TAGS,
        ])
    );

    let mut tags = request("Constraint");
    tags.preferences = preferences();
    tags.require_all_tags_match = true;
    assert_eq!(
        names(&fixture.discover(tags).unwrap()),
        sorted_names(&[
            CONSTRAINT_EXACT,
            CONSTRAINT_WRONG_CITY,
            CONSTRAINT_WRONG_CATEGORY,
        ])
    );

    let mut all = request("Constraint");
    all.preferences = preferences();
    all.require_city_match = true;
    all.require_category_match = true;
    all.require_all_tags_match = true;
    let all = fixture.discover(all).unwrap();
    assert_eq!(names(&all), vec![CONSTRAINT_EXACT.to_string()]);
    assert!(all.matches[0]
        .reasons
        .iter()
        .any(|reason| reason == "硬性城市条件匹配 JiAn"));
    assert!(all.matches[0]
        .reasons
        .iter()
        .any(|reason| reason == "硬性经营类别条件匹配 Cafe"));
    assert!(all.matches[0]
        .reasons
        .iter()
        .any(|reason| reason == "硬性标签条件全部匹配 3 项"));

    let mut any_category = request("Constraint");
    any_category.preferences.categories = vec!["retail".to_string(), "cafe".to_string()];
    any_category.require_category_match = true;
    assert_eq!(
        names(&fixture.discover(any_category).unwrap()),
        sorted_names(&[
            CONSTRAINT_EXACT,
            CONSTRAINT_WRONG_CITY,
            CONSTRAINT_WRONG_CATEGORY,
            CONSTRAINT_PARTIAL_TAGS,
        ])
    );

    for (city, category, tags, expected) in [
        (true, false, false, "必须填写城市"),
        (false, true, false, "必须填写至少一个类别"),
        (false, false, true, "必须填写至少一个标签"),
    ] {
        let mut missing = request("Constraint");
        missing.require_city_match = city;
        missing.require_category_match = category;
        missing.require_all_tags_match = tags;
        assert!(fixture
            .discover(missing)
            .unwrap_err()
            .to_string()
            .contains(expected));
    }

    assert!(!names(&soft).iter().any(|name| name == "missing"));
    assert!(names(&soft).contains(&CONSTRAINT_MALFORMED.to_string()));
}

#[test]
fn combined_filters_are_committed_to_receipt_without_business_writes() {
    let fixture = fixture();
    let before = fixture.snapshot();
    let mut combined = request("Constraint");
    combined.capability_key = Some(CONSTRAINT_CAPABILITY.to_string());
    combined.capability_kind = Some("query".to_string());
    combined.access_level = Some("public".to_string());
    combined.price_currency = Some("cny".to_string());
    combined.preferences.categories = vec!["cafe".to_string()];
    combined.preferences.tags = vec!["quiet".to_string(), "wifi".to_string()];
    combined.preferences.city = Some("jian".to_string());
    combined.preferences.max_unit_price_micros = Some(100);
    combined.require_city_match = true;
    combined.require_category_match = true;
    combined.require_all_tags_match = true;
    combined.include_ranking_receipt = true;
    let combined = fixture.discover(combined).unwrap();
    assert_eq!(names(&combined), vec![CONSTRAINT_EXACT.to_string()]);
    let payload = receipt_payload(&combined);
    assert_eq!(payload["price_filter"]["currency"], "CNY");
    assert_eq!(payload["price_filter"]["max_unit_price_micros"], 100);
    assert_eq!(payload["capability_filter"]["kind"], "query");
    assert_eq!(payload["capability_filter"]["access_level"], "public");
    assert_eq!(
        payload["preference_constraints"]["require_city_match"],
        true
    );
    assert_eq!(
        payload["preference_constraints"]["require_category_match"],
        true
    );
    assert_eq!(
        payload["preference_constraints"]["require_all_tags_match"],
        true
    );
    let fingerprint = payload["request_fingerprint_sha256"]
        .as_str()
        .unwrap()
        .to_string();

    let mut changed = request("Constraint");
    changed.capability_key = Some(CONSTRAINT_CAPABILITY.to_string());
    changed.capability_kind = Some("query".to_string());
    changed.access_level = Some("public".to_string());
    changed.price_currency = Some("CNY".to_string());
    changed.preferences.categories = vec!["cafe".to_string()];
    changed.preferences.tags = vec!["quiet".to_string(), "wifi".to_string()];
    changed.preferences.city = Some("jian".to_string());
    changed.preferences.max_unit_price_micros = Some(100);
    changed.require_city_match = true;
    changed.require_category_match = true;
    changed.require_all_tags_match = false;
    changed.include_ranking_receipt = true;
    let changed = fixture.discover(changed).unwrap();
    let changed_payload = receipt_payload(&changed);
    assert_ne!(
        fingerprint,
        changed_payload["request_fingerprint_sha256"]
            .as_str()
            .unwrap()
    );
    assert_eq!(before, fixture.snapshot());
}

fn sorted_names(values: &[&str]) -> Vec<String> {
    let mut values = values
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    values.sort();
    values
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
