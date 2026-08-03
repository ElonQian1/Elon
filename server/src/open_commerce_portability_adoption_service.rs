use std::collections::BTreeSet;

use anyhow::{anyhow, bail, Result};
use serde_json::{json, to_value};

use crate::{
    open_commerce_consumer_model::ConsumerPreferences,
    open_commerce_consumer_preference_service,
    open_commerce_portability_adoption_model::{
        ApplyConsumerPortabilityPreferencesRequest, ConsumerPortabilityAdoption,
        ConsumerPortabilityAdoptionPlan, ConsumerPortabilityPreferenceChange,
        ConsumerPortabilityRelationshipCandidate, RollbackConsumerPortabilityAdoptionRequest,
        CONSUMER_PORTABILITY_ADOPTION_PLAN_SCHEMA, CONSUMER_PORTABILITY_ADOPTION_SCHEMA,
    },
    open_commerce_portability_import_model::CONSUMER_PORTABILITY_IMPORT_TRUSTED_STATUS,
    open_commerce_portability_import_service,
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

pub(crate) fn adoption_plan(
    store: &Store,
    destination_project_id: &str,
    import_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<ConsumerPortabilityAdoptionPlan> {
    ensure_consumer_project_actor(actor)?;
    let import_record = open_commerce_portability_import_service::get_import(
        store,
        destination_project_id,
        import_id,
        actor,
    )?;
    let current = open_commerce_consumer_preference_service::get_profile(
        store,
        destination_project_id,
        actor,
    )?;
    let imported = import_record
        .package
        .payload
        .preference_profile
        .as_ref()
        .map(|profile| &profile.preferences);
    let current_preferences = current.as_ref().map(|profile| &profile.preferences);
    let preference_changes = imported
        .map(|preferences| preference_changes(current_preferences, preferences))
        .transpose()?
        .unwrap_or_default();
    let trusted_operator = import_record.trust_status == CONSUMER_PORTABILITY_IMPORT_TRUSTED_STATUS;
    let relationship_candidates = import_record
        .package
        .payload
        .relationships
        .iter()
        .map(|relationship| {
            let source_identity_key_ids = import_record
                .package
                .payload
                .merchant_identity_claims
                .iter()
                .find(|claim| claim.source_merchant_id == relationship.merchant_id)
                .map(|claim| claim.key_ids.clone())
                .unwrap_or_default();
            let verified_target_merchant_ids = if trusted_operator {
                store.published_open_commerce_merchant_ids_for_identity_keys(
                    &source_identity_key_ids,
                )?
            } else {
                Vec::new()
            };
            let identity_match_authority = (!verified_target_merchant_ids.is_empty())
                .then(|| "trusted_operator_package_plus_matching_possession_key".to_string());
            Ok(ConsumerPortabilityRelationshipCandidate {
                source_relationship_id: relationship.id.clone(),
                source_merchant_id: relationship.merchant_id.clone(),
                source_status: relationship.status.clone(),
                requested_scopes: relationship.scopes.clone(),
                purpose: relationship.purpose.clone(),
                requires_reauthorization: true,
                source_identity_key_ids,
                verified_target_merchant_ids,
                identity_match_authority,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ConsumerPortabilityAdoptionPlan {
        schema: CONSUMER_PORTABILITY_ADOPTION_PLAN_SCHEMA.to_string(),
        import_id: import_record.id,
        import_trust_status: import_record.trust_status,
        source_package_schema: import_record.source_package_schema,
        imported_profile_available: imported.is_some(),
        current_profile_revision: current.map(|profile| profile.revision),
        preference_changes,
        relationship_candidates,
        automatic_relationship_restore: false,
        automatic_business_write: false,
    })
}

pub(crate) fn apply_preferences(
    store: &Store,
    destination_project_id: &str,
    import_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: ApplyConsumerPortabilityPreferencesRequest,
) -> Result<ConsumerPortabilityAdoption> {
    ensure_consumer_project_actor(actor)?;
    if !request.confirmed_by_user {
        bail!("采用导入偏好必须由用户明确确认");
    }
    let import_record = open_commerce_portability_import_service::get_import(
        store,
        destination_project_id,
        import_id,
        actor,
    )?;
    let imported = import_record
        .package
        .payload
        .preference_profile
        .map(|profile| profile.preferences)
        .ok_or_else(|| anyhow!("导入数据包不包含消费者偏好档案"))?;
    let imported = open_commerce_consumer_preference_service::normalize_preferences(imported)?;
    let current = open_commerce_consumer_preference_service::get_profile(
        store,
        destination_project_id,
        actor,
    )?;
    let current_revision = current.as_ref().map(|profile| profile.revision);
    if current_revision != request.expected_current_revision {
        bail!("消费者偏好档案已变化，请刷新迁移预演后重试");
    }
    let selected_fields = normalize_selected_fields(request.selected_fields)?;
    let preferences = merge_selected_preferences(
        current
            .as_ref()
            .map(|profile| profile.preferences.clone())
            .unwrap_or_default(),
        &imported,
        &selected_fields,
    )?;
    let preferences =
        open_commerce_consumer_preference_service::normalize_preferences(preferences)?;
    let adoption = store.apply_consumer_portability_preferences(
        &import_record.id,
        destination_project_id,
        actor.user_id,
        request.expected_current_revision,
        &preferences,
    )?;
    store.record_open_commerce_audit(
        destination_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_portability.preferences_adopted",
        "consumer_portability_adoption",
        &adoption.id,
        &json!({
            "import_id": adoption.import_id,
            "before_revision": adoption.before_revision,
            "resulting_revision": adoption.resulting_revision,
            "selected_fields": &adoption.selected_fields,
            "source_trust_status": import_record.trust_status,
        }),
    )?;
    Ok(adoption)
}

fn normalize_selected_fields(fields: Vec<String>) -> Result<Vec<String>> {
    let allowed = [
        "categories",
        "tags",
        "city",
        "max_unit_price_micros",
        "prefer_public",
    ];
    let mut selected = BTreeSet::new();
    for field in fields {
        let field = field.trim().to_ascii_lowercase();
        if !allowed.contains(&field.as_str()) {
            bail!("偏好字段 {field} 不支持选择性采用");
        }
        selected.insert(field);
    }
    if selected.is_empty() {
        bail!("至少选择一个需要采用的偏好字段");
    }
    Ok(selected.into_iter().collect())
}

fn merge_selected_preferences(
    mut current: ConsumerPreferences,
    imported: &ConsumerPreferences,
    selected_fields: &[String],
) -> Result<ConsumerPreferences> {
    let mut changed = 0;
    for field in selected_fields {
        match field.as_str() {
            "categories" if current.categories != imported.categories => {
                current.categories = imported.categories.clone();
                changed += 1;
            }
            "tags" if current.tags != imported.tags => {
                current.tags = imported.tags.clone();
                changed += 1;
            }
            "city" if current.city != imported.city => {
                current.city = imported.city.clone();
                changed += 1;
            }
            "max_unit_price_micros"
                if current.max_unit_price_micros != imported.max_unit_price_micros =>
            {
                current.max_unit_price_micros = imported.max_unit_price_micros;
                changed += 1;
            }
            "prefer_public" if current.prefer_public != imported.prefer_public => {
                current.prefer_public = imported.prefer_public;
                changed += 1;
            }
            "categories" | "tags" | "city" | "max_unit_price_micros" | "prefer_public" => {}
            _ => bail!("未知偏好字段"),
        }
    }
    if changed != selected_fields.len() {
        bail!("只能采用预演中真实发生变化的偏好字段，请刷新后重新选择");
    }
    Ok(current)
}

pub(crate) fn list_adoptions(
    store: &Store,
    destination_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<ConsumerPortabilityAdoption>> {
    ensure_consumer_project_actor(actor)?;
    let records =
        store.list_consumer_portability_adoptions(destination_project_id, actor.user_id, limit)?;
    if records.iter().any(|record| {
        record.schema != CONSUMER_PORTABILITY_ADOPTION_SCHEMA
            || record.kind != "preferences"
            || !matches!(record.status.as_str(), "applied" | "rolled_back")
    }) {
        bail!("消费者数据包采用记录无效");
    }
    Ok(records)
}

pub(crate) fn rollback_adoption(
    store: &Store,
    destination_project_id: &str,
    adoption_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: RollbackConsumerPortabilityAdoptionRequest,
) -> Result<ConsumerPortabilityAdoption> {
    ensure_consumer_project_actor(actor)?;
    if !request.confirmed_by_user {
        bail!("回滚导入偏好必须由用户明确确认");
    }
    let adoption_id = adoption_id.trim();
    if adoption_id.is_empty() || adoption_id.chars().count() > 120 {
        bail!("消费者数据包采用记录 ID 长度必须为 1 到 120 个字符");
    }
    let adoption = store.rollback_consumer_portability_adoption(
        adoption_id,
        destination_project_id,
        actor.user_id,
        request.expected_current_revision,
    )?;
    store.record_open_commerce_audit(
        destination_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_portability.preferences_rolled_back",
        "consumer_portability_adoption",
        &adoption.id,
        &json!({
            "import_id": adoption.import_id,
            "resulting_revision": adoption.resulting_revision,
            "rollback_revision": adoption.rollback_revision,
        }),
    )?;
    Ok(adoption)
}

fn preference_changes(
    current: Option<&ConsumerPreferences>,
    imported: &ConsumerPreferences,
) -> Result<Vec<ConsumerPortabilityPreferenceChange>> {
    let empty = ConsumerPreferences::default();
    let current = current.unwrap_or(&empty);
    let fields = [
        (
            "categories",
            to_value(&current.categories)?,
            to_value(&imported.categories)?,
        ),
        ("tags", to_value(&current.tags)?, to_value(&imported.tags)?),
        ("city", to_value(&current.city)?, to_value(&imported.city)?),
        (
            "max_unit_price_micros",
            to_value(current.max_unit_price_micros)?,
            to_value(imported.max_unit_price_micros)?,
        ),
        (
            "prefer_public",
            to_value(current.prefer_public)?,
            to_value(imported.prefer_public)?,
        ),
    ];
    Ok(fields
        .into_iter()
        .map(
            |(field, current_value, imported_value)| ConsumerPortabilityPreferenceChange {
                field: field.to_string(),
                changed: current_value != imported_value,
                current_value,
                imported_value,
            },
        )
        .collect())
}

fn ensure_consumer_project_actor(actor: &OpenCommerceActor<'_>) -> Result<()> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于消费者项目");
    }
    Ok(())
}
