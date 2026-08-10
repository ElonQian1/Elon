use crate::{
    open_commerce_consumer_model::ConsumerPreferences,
    open_commerce_consumer_preference_model::UpsertConsumerPreferenceProfileRequest,
    open_commerce_consumer_preference_service,
    open_commerce_portability_adoption_model::{
        ApplyConsumerPortabilityPreferencesRequest, RollbackConsumerPortabilityAdoptionRequest,
    },
};

#[path = "open_commerce_portability_adoption_test_support.rs"]
mod support;
use support::*;

const FIELDS: [&str; 5] = [
    "categories",
    "tags",
    "city",
    "max_unit_price_micros",
    "prefer_public",
];

#[test]
fn selective_fields_apply_and_rollback_without_overwriting_unselected_values() {
    for field in FIELDS {
        let fixture = fixture(true);
        let actor = fixture.owner_actor();
        let plan = super::adoption_plan(
            &fixture.state.store,
            &fixture.target_project_id,
            &fixture.import_id,
            &actor,
        )
        .unwrap();
        assert_eq!(plan.current_profile_revision, Some(1));
        assert_eq!(plan.preference_changes.len(), FIELDS.len());
        assert!(plan.preference_changes.iter().all(|change| change.changed));

        let adoption = super::apply_preferences(
            &fixture.state.store,
            &fixture.target_project_id,
            &fixture.import_id,
            &actor,
            apply_request(Some(1), vec![field, field]),
        )
        .unwrap();
        assert_eq!(adoption.selected_fields, vec![field.to_string()]);
        assert_eq!(adoption.before_revision, Some(1));
        assert_eq!(adoption.resulting_revision, 2);
        let expected = expected_after_field(
            fixture.current_preferences.clone(),
            &fixture.imported_preferences,
            field,
        );
        assert_preferences(&adoption.applied_preferences, &expected);
        assert_preferences(&fixture.owner_profile().unwrap().preferences, &expected);

        let rolled_back = super::rollback_adoption(
            &fixture.state.store,
            &fixture.target_project_id,
            &adoption.id,
            &actor,
            rollback_request(2),
        )
        .unwrap();
        assert_eq!(rolled_back.status, "rolled_back");
        assert_eq!(rolled_back.rollback_revision, Some(3));
        let restored = fixture.owner_profile().unwrap();
        assert_eq!(restored.revision, 3);
        assert_preferences(&restored.preferences, &fixture.current_preferences);
    }
}

#[test]
fn selective_adoption_without_current_profile_uses_defaults_and_deletes_on_rollback() {
    let fixture = fixture(false);
    let actor = fixture.owner_actor();
    let plan = super::adoption_plan(
        &fixture.state.store,
        &fixture.target_project_id,
        &fixture.import_id,
        &actor,
    )
    .unwrap();
    assert_eq!(plan.current_profile_revision, None);

    let adoption = super::apply_preferences(
        &fixture.state.store,
        &fixture.target_project_id,
        &fixture.import_id,
        &actor,
        apply_request(None, vec!["tags"]),
    )
    .unwrap();
    assert!(adoption.before_preferences.is_none());
    assert_eq!(adoption.before_revision, None);
    assert_eq!(adoption.resulting_revision, 1);
    let expected = ConsumerPreferences {
        tags: fixture.imported_preferences.tags.clone(),
        ..ConsumerPreferences::default()
    };
    assert_preferences(&adoption.applied_preferences, &expected);

    let rolled_back = super::rollback_adoption(
        &fixture.state.store,
        &fixture.target_project_id,
        &adoption.id,
        &actor,
        rollback_request(1),
    )
    .unwrap();
    assert_eq!(rolled_back.rollback_revision, None);
    assert!(fixture.owner_profile().is_none());
}

#[test]
fn invalid_and_stale_selections_fail_without_creating_adoptions() {
    let fixture = fixture(true);
    let actor = fixture.owner_actor();
    for request in [
        ApplyConsumerPortabilityPreferencesRequest {
            expected_current_revision: Some(1),
            selected_fields: vec!["city".to_string()],
            confirmed_by_user: false,
        },
        apply_request(Some(1), Vec::new()),
        apply_request(Some(1), vec!["private_notes"]),
    ] {
        assert!(super::apply_preferences(
            &fixture.state.store,
            &fixture.target_project_id,
            &fixture.import_id,
            &actor,
            request,
        )
        .is_err());
    }

    let mut changed = fixture.current_preferences.clone();
    changed.city = fixture.imported_preferences.city.clone();
    open_commerce_consumer_preference_service::upsert_profile(
        &fixture.state.store,
        &fixture.target_project_id,
        &actor,
        UpsertConsumerPreferenceProfileRequest {
            preferences: changed,
        },
    )
    .unwrap();
    let unchanged_error = super::apply_preferences(
        &fixture.state.store,
        &fixture.target_project_id,
        &fixture.import_id,
        &actor,
        apply_request(Some(2), vec!["city"]),
    )
    .unwrap_err();
    assert!(unchanged_error
        .to_string()
        .contains("真实发生变化的偏好字段"));

    let stale_fixture = support::fixture(true);
    let stale_actor = stale_fixture.owner_actor();
    open_commerce_consumer_preference_service::upsert_profile(
        &stale_fixture.state.store,
        &stale_fixture.target_project_id,
        &stale_actor,
        UpsertConsumerPreferenceProfileRequest {
            preferences: stale_fixture.current_preferences.clone(),
        },
    )
    .unwrap();
    let stale_error = super::apply_preferences(
        &stale_fixture.state.store,
        &stale_fixture.target_project_id,
        &stale_fixture.import_id,
        &stale_actor,
        apply_request(Some(1), vec!["tags"]),
    )
    .unwrap_err();
    assert!(stale_error.to_string().contains("已变化"));
    assert!(super::list_adoptions(
        &stale_fixture.state.store,
        &stale_fixture.target_project_id,
        &stale_actor,
        100,
    )
    .unwrap()
    .is_empty());
}

#[test]
fn legacy_records_derive_all_changed_fields_and_audit_omits_preference_values() {
    let fixture = fixture(true);
    let actor = fixture.owner_actor();
    let adoption = fixture
        .state
        .store
        .apply_consumer_portability_preferences(
            &fixture.import_id,
            &fixture.target_project_id,
            &fixture.owner_id,
            Some(1),
            &fixture.imported_preferences,
        )
        .unwrap();
    assert_eq!(adoption.selected_fields, FIELDS.map(str::to_string));
    let listed = super::list_adoptions(
        &fixture.state.store,
        &fixture.target_project_id,
        &actor,
        100,
    )
    .unwrap();
    assert_eq!(listed[0].selected_fields, FIELDS.map(str::to_string));

    let rolled_back = super::rollback_adoption(
        &fixture.state.store,
        &fixture.target_project_id,
        &adoption.id,
        &actor,
        rollback_request(2),
    )
    .unwrap();
    assert_eq!(rolled_back.status, "rolled_back");

    let fresh_fixture = support::fixture(true);
    let fresh_actor = fresh_fixture.owner_actor();
    super::apply_preferences(
        &fresh_fixture.state.store,
        &fresh_fixture.target_project_id,
        &fresh_fixture.import_id,
        &fresh_actor,
        apply_request(Some(1), FIELDS.to_vec()),
    )
    .unwrap();
    let audit = fresh_fixture
        .state
        .store
        .list_project_open_commerce_audit(&fresh_fixture.target_project_id, 20)
        .unwrap()
        .into_iter()
        .find(|event| event.action == "consumer_portability.preferences_adopted")
        .unwrap();
    let metadata = audit.metadata.to_string();
    assert!(metadata.contains("selected_fields"));
    assert!(!metadata.contains("Shanghai"));
    assert!(!metadata.contains("coffee"));
    assert!(!metadata.contains("quiet"));
}

fn apply_request(
    expected_current_revision: Option<i64>,
    fields: Vec<&str>,
) -> ApplyConsumerPortabilityPreferencesRequest {
    ApplyConsumerPortabilityPreferencesRequest {
        expected_current_revision,
        selected_fields: fields.into_iter().map(str::to_string).collect(),
        confirmed_by_user: true,
    }
}

fn rollback_request(revision: i64) -> RollbackConsumerPortabilityAdoptionRequest {
    RollbackConsumerPortabilityAdoptionRequest {
        expected_current_revision: revision,
        confirmed_by_user: true,
    }
}

fn expected_after_field(
    mut current: ConsumerPreferences,
    imported: &ConsumerPreferences,
    field: &str,
) -> ConsumerPreferences {
    match field {
        "categories" => current.categories = imported.categories.clone(),
        "tags" => current.tags = imported.tags.clone(),
        "city" => current.city = imported.city.clone(),
        "max_unit_price_micros" => current.max_unit_price_micros = imported.max_unit_price_micros,
        "prefer_public" => current.prefer_public = imported.prefer_public,
        _ => panic!("unexpected field {field}"),
    }
    current
}

fn assert_preferences(actual: &ConsumerPreferences, expected: &ConsumerPreferences) {
    assert_eq!(actual.categories, expected.categories);
    assert_eq!(actual.tags, expected.tags);
    assert_eq!(actual.city, expected.city);
    assert_eq!(actual.max_unit_price_micros, expected.max_unit_price_micros);
    assert_eq!(actual.prefer_public, expected.prefer_public);
}
