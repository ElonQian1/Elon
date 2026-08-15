use crate::{
    open_commerce_consumer_model::ConsumerPreferences,
    open_commerce_consumer_preference_model::UpsertConsumerPreferenceProfileRequest,
    open_commerce_consumer_preference_service,
    open_commerce_portability_adoption_model::ApplyConsumerPortabilityPreferencesRequest,
    open_commerce_portability_import_service,
    open_commerce_portability_merge_model::{
        ApplyConsumerPortabilityMergeRequest, ConsumerPortabilityFieldSelection,
        CreateConsumerPortabilityMergePlanRequest, RollbackConsumerPortabilityMergeRequest,
    },
};

#[path = "open_commerce_portability_adoption_test_support.rs"]
mod support;
use support::*;

#[test]
fn plan_exposes_conflicts_without_automatic_effects() {
    let fixture = fixture(true);
    let alternate_id = add_import(&fixture, Some(alternate_preferences()), "alternate");
    let plan = super::merge_plan(
        &fixture.state.store,
        &fixture.target_project_id,
        &fixture.owner_actor(),
        plan_request(vec![fixture.import_id.clone(), alternate_id]),
    )
    .unwrap();

    assert_eq!(plan.current_profile_revision, Some(1));
    assert_eq!(plan.sources.len(), 2);
    assert_eq!(plan.fields.len(), 5);
    assert!(!plan.automatic_conflict_resolution);
    assert!(!plan.automatic_relationship_restore);
    assert!(!plan.automatic_business_write);
    assert_eq!(field(&plan, "categories").distinct_candidate_count, 2);
    assert!(field(&plan, "categories").conflict);
    assert_eq!(field(&plan, "tags").distinct_candidate_count, 1);
    assert!(!field(&plan, "tags").conflict);
    assert!(plan.fields.iter().all(|item| item.candidates.len() == 2));
}

#[test]
fn apply_combines_explicit_sources_and_audits_only_provenance() {
    let fixture = fixture(true);
    let alternate_id = add_import(&fixture, Some(alternate_preferences()), "alternate");
    let adoption = super::apply_merge(
        &fixture.state.store,
        &fixture.target_project_id,
        &fixture.owner_actor(),
        apply_request(
            vec![fixture.import_id.clone(), alternate_id.clone()],
            Some(1),
            vec![
                selection("categories", &fixture.import_id),
                selection("city", &alternate_id),
            ],
        ),
    )
    .unwrap();

    assert_eq!(adoption.before_revision, Some(1));
    assert_eq!(adoption.resulting_revision, 2);
    assert_eq!(
        adoption.applied_preferences.categories,
        vec!["coffee", "dessert"]
    );
    assert_eq!(
        adoption.applied_preferences.city.as_deref(),
        Some("Guangzhou")
    );
    assert_eq!(
        adoption.applied_preferences.tags,
        fixture.current_preferences.tags
    );
    assert_eq!(adoption.field_sources.len(), 2);
    assert!(adoption
        .field_sources
        .iter()
        .any(|source| source.field == "city" && source.import_id == alternate_id));

    let listed = super::list_merges(
        &fixture.state.store,
        &fixture.target_project_id,
        &fixture.owner_actor(),
        100,
    )
    .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, adoption.id);

    let audit = fixture
        .state
        .store
        .list_project_open_commerce_audit(&fixture.target_project_id, 50)
        .unwrap()
        .into_iter()
        .find(|event| event.action == "consumer_portability.preferences_merged")
        .unwrap();
    let metadata = audit.metadata.to_string();
    assert!(metadata.contains("field_sources"));
    assert!(metadata.contains("source_import_ids"));
    for private_value in ["coffee", "dessert", "Guangzhou", "quiet", "wifi"] {
        assert!(!metadata.contains(private_value));
    }
}

#[test]
fn invalid_sources_and_field_selections_fail_closed() {
    let fixture = fixture(true);
    let alternate_id = add_import(&fixture, Some(alternate_preferences()), "alternate");
    let actor = fixture.owner_actor();

    for ids in [
        vec![fixture.import_id.clone()],
        vec![fixture.import_id.clone(), fixture.import_id.clone()],
        (0..11).map(|index| format!("import-{index}")).collect(),
    ] {
        assert!(super::merge_plan(
            &fixture.state.store,
            &fixture.target_project_id,
            &actor,
            plan_request(ids),
        )
        .is_err());
    }

    assert!(super::merge_plan(
        &fixture.state.store,
        &fixture.target_project_id,
        &fixture.member_actor(),
        plan_request(vec![fixture.import_id.clone(), alternate_id.clone()]),
    )
    .unwrap_err()
    .to_string()
    .contains("不存在"));

    let deleted_id = add_import(&fixture, Some(alternate_preferences()), "deleted");
    open_commerce_portability_import_service::delete_import(
        &fixture.state.store,
        &fixture.target_project_id,
        &deleted_id,
        &actor,
    )
    .unwrap();
    assert!(super::merge_plan(
        &fixture.state.store,
        &fixture.target_project_id,
        &actor,
        plan_request(vec![fixture.import_id.clone(), deleted_id]),
    )
    .is_err());

    let no_preferences_id = add_import(&fixture, None, "empty");
    assert!(super::merge_plan(
        &fixture.state.store,
        &fixture.target_project_id,
        &actor,
        plan_request(vec![fixture.import_id.clone(), no_preferences_id]),
    )
    .unwrap_err()
    .to_string()
    .contains("不包含消费者偏好档案"));

    let third_id = add_import(&fixture, Some(third_preferences()), "third");
    for request in [
        ApplyConsumerPortabilityMergeRequest {
            confirmed_by_user: false,
            ..apply_request(
                vec![fixture.import_id.clone(), alternate_id.clone()],
                Some(1),
                vec![selection("categories", &fixture.import_id)],
            )
        },
        apply_request(
            vec![fixture.import_id.clone(), alternate_id.clone()],
            Some(1),
            vec![selection("unknown", &fixture.import_id)],
        ),
        apply_request(
            vec![fixture.import_id.clone(), alternate_id.clone()],
            Some(1),
            vec![
                selection("city", &fixture.import_id),
                selection("city", &alternate_id),
            ],
        ),
        apply_request(
            vec![fixture.import_id.clone(), alternate_id.clone()],
            Some(1),
            vec![selection("city", &third_id)],
        ),
    ] {
        assert!(super::apply_merge(
            &fixture.state.store,
            &fixture.target_project_id,
            &actor,
            request,
        )
        .is_err());
    }

    let unchanged_id = add_import(
        &fixture,
        Some(fixture.current_preferences.clone()),
        "unchanged",
    );
    assert!(super::apply_merge(
        &fixture.state.store,
        &fixture.target_project_id,
        &actor,
        apply_request(
            vec![fixture.import_id.clone(), unchanged_id.clone()],
            Some(1),
            vec![selection("city", &unchanged_id)],
        ),
    )
    .unwrap_err()
    .to_string()
    .contains("真实发生变化"));

    open_commerce_consumer_preference_service::upsert_profile(
        &fixture.state.store,
        &fixture.target_project_id,
        &actor,
        UpsertConsumerPreferenceProfileRequest {
            preferences: fixture.current_preferences.clone(),
        },
    )
    .unwrap();
    assert!(super::apply_merge(
        &fixture.state.store,
        &fixture.target_project_id,
        &actor,
        apply_request(
            vec![fixture.import_id.clone(), alternate_id],
            Some(1),
            vec![selection("categories", &fixture.import_id)],
        ),
    )
    .unwrap_err()
    .to_string()
    .contains("已变化"));
    assert!(super::list_merges(
        &fixture.state.store,
        &fixture.target_project_id,
        &actor,
        100,
    )
    .unwrap()
    .is_empty());
}

#[test]
fn rollback_restores_or_deletes_profile_and_rejects_later_changes() {
    let populated_fixture = fixture(true);
    let alternate_id = add_import(
        &populated_fixture,
        Some(alternate_preferences()),
        "alternate",
    );
    let actor = populated_fixture.owner_actor();
    let adoption = super::apply_merge(
        &populated_fixture.state.store,
        &populated_fixture.target_project_id,
        &actor,
        apply_request(
            vec![populated_fixture.import_id.clone(), alternate_id],
            Some(1),
            vec![selection("categories", &populated_fixture.import_id)],
        ),
    )
    .unwrap();
    let mut later = adoption.applied_preferences.clone();
    later.tags = vec!["manual".to_string()];
    open_commerce_consumer_preference_service::upsert_profile(
        &populated_fixture.state.store,
        &populated_fixture.target_project_id,
        &actor,
        UpsertConsumerPreferenceProfileRequest { preferences: later },
    )
    .unwrap();
    assert!(super::rollback_merge(
        &populated_fixture.state.store,
        &populated_fixture.target_project_id,
        &adoption.id,
        &actor,
        rollback_request(3),
    )
    .unwrap_err()
    .to_string()
    .contains("拒绝覆盖后续修改"));

    let empty_fixture = fixture(false);
    let empty_alternate_id = add_import(
        &empty_fixture,
        Some(alternate_preferences()),
        "alternate-empty",
    );
    let empty_actor = empty_fixture.owner_actor();
    let created = super::apply_merge(
        &empty_fixture.state.store,
        &empty_fixture.target_project_id,
        &empty_actor,
        apply_request(
            vec![empty_fixture.import_id.clone(), empty_alternate_id],
            None,
            vec![selection("tags", &empty_fixture.import_id)],
        ),
    )
    .unwrap();
    assert_eq!(created.resulting_revision, 1);
    let rolled_back = super::rollback_merge(
        &empty_fixture.state.store,
        &empty_fixture.target_project_id,
        &created.id,
        &empty_actor,
        rollback_request(1),
    )
    .unwrap();
    assert_eq!(rolled_back.status, "rolled_back");
    assert_eq!(rolled_back.rollback_revision, None);
    assert!(empty_fixture.owner_profile().is_none());
    assert!(super::rollback_merge(
        &empty_fixture.state.store,
        &empty_fixture.target_project_id,
        &created.id,
        &empty_actor,
        rollback_request(1),
    )
    .is_err());
}

#[test]
fn single_package_selective_adoption_remains_compatible() {
    let fixture = fixture(true);
    let adoption = crate::open_commerce_portability_adoption_service::apply_preferences(
        &fixture.state.store,
        &fixture.target_project_id,
        &fixture.import_id,
        &fixture.owner_actor(),
        ApplyConsumerPortabilityPreferencesRequest {
            expected_current_revision: Some(1),
            selected_fields: vec!["city".to_string()],
            confirmed_by_user: true,
        },
    )
    .unwrap();
    assert_eq!(adoption.selected_fields, vec!["city"]);
    assert_eq!(
        adoption.applied_preferences.city.as_deref(),
        Some("Shanghai")
    );
}

fn plan_request(import_ids: Vec<String>) -> CreateConsumerPortabilityMergePlanRequest {
    CreateConsumerPortabilityMergePlanRequest { import_ids }
}

fn apply_request(
    import_ids: Vec<String>,
    expected_current_revision: Option<i64>,
    selections: Vec<ConsumerPortabilityFieldSelection>,
) -> ApplyConsumerPortabilityMergeRequest {
    ApplyConsumerPortabilityMergeRequest {
        import_ids,
        expected_current_revision,
        selections,
        confirmed_by_user: true,
    }
}

fn rollback_request(expected_current_revision: i64) -> RollbackConsumerPortabilityMergeRequest {
    RollbackConsumerPortabilityMergeRequest {
        expected_current_revision,
        confirmed_by_user: true,
    }
}

fn selection(field: &str, import_id: &str) -> ConsumerPortabilityFieldSelection {
    ConsumerPortabilityFieldSelection {
        field: field.to_string(),
        import_id: import_id.to_string(),
    }
}

fn field<'a>(
    plan: &'a crate::open_commerce_portability_merge_model::ConsumerPortabilityMergePlan,
    name: &str,
) -> &'a crate::open_commerce_portability_merge_model::ConsumerPortabilityMergeField {
    plan.fields
        .iter()
        .find(|field| field.field == name)
        .unwrap()
}

fn alternate_preferences() -> ConsumerPreferences {
    ConsumerPreferences {
        categories: vec!["bakery".to_string()],
        tags: vec!["quiet".to_string(), "wifi".to_string()],
        city: Some("Guangzhou".to_string()),
        max_unit_price_micros: Some(50_000_000),
        prefer_public: true,
    }
}

fn third_preferences() -> ConsumerPreferences {
    ConsumerPreferences {
        categories: vec!["brunch".to_string()],
        tags: vec!["family".to_string()],
        city: Some("Shenzhen".to_string()),
        max_unit_price_micros: Some(60_000_000),
        prefer_public: false,
    }
}
