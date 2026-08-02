use anyhow::{bail, Result};
use serde_json::json;

use crate::{
    open_commerce_consumer_model::ConsumerPreferences,
    open_commerce_consumer_preference_model::{
        ConsumerPreferenceDisclosure, ConsumerPreferenceProfile,
        DeleteConsumerPreferenceDisclosureResult, DeleteConsumerPreferenceProfileResult,
        DisclosedConsumerPreferences, UpsertConsumerPreferenceDisclosureRequest,
        UpsertConsumerPreferenceProfileRequest, PREFERENCE_FIELD_CATEGORIES, PREFERENCE_FIELD_CITY,
        PREFERENCE_FIELD_MAX_UNIT_PRICE, PREFERENCE_FIELD_TAGS,
    },
    open_commerce_relationship_model::RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER,
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

const MAX_PREFERENCE_VALUE_LEN: usize = 80;
const MAX_CITY_LEN: usize = 120;
const MAX_PRICE_MICROS: i64 = 1_000_000_000_000_000;

pub(crate) fn get_profile(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<Option<ConsumerPreferenceProfile>> {
    ensure_project_actor(actor, "消费者偏好项目")?;
    store.open_commerce_consumer_preference_profile(consumer_project_id, actor.user_id)
}

pub(crate) fn upsert_profile(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: UpsertConsumerPreferenceProfileRequest,
) -> Result<ConsumerPreferenceProfile> {
    ensure_project_actor(actor, "消费者偏好项目")?;
    let preferences = normalize_preferences(request.preferences)?;
    let profile = store.upsert_open_commerce_consumer_preference_profile(
        consumer_project_id,
        actor.user_id,
        &preferences,
    )?;
    store.record_open_commerce_audit(
        consumer_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_preference_profile.upserted",
        "consumer_preference_profile",
        actor.user_id,
        &json!({
            "revision": profile.revision,
            "category_count": profile.preferences.categories.len(),
            "tag_count": profile.preferences.tags.len(),
            "has_city": profile.preferences.city.is_some(),
            "has_max_unit_price": profile.preferences.max_unit_price_micros.is_some()
        }),
    )?;
    Ok(profile)
}

pub(crate) fn delete_profile(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<DeleteConsumerPreferenceProfileResult> {
    ensure_project_actor(actor, "消费者偏好项目")?;
    let (deleted_profile, removed_disclosures) = store
        .delete_open_commerce_consumer_preference_profile(consumer_project_id, actor.user_id)?;
    let result = DeleteConsumerPreferenceProfileResult {
        deleted_profile,
        removed_disclosures,
    };
    store.record_open_commerce_audit(
        consumer_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_preference_profile.deleted",
        "consumer_preference_profile",
        actor.user_id,
        &json!({
            "deleted_profile": result.deleted_profile,
            "removed_disclosures": result.removed_disclosures
        }),
    )?;
    Ok(result)
}

pub(crate) fn get_disclosure(
    store: &Store,
    consumer_project_id: &str,
    relationship_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<Option<ConsumerPreferenceDisclosure>> {
    ensure_project_actor(actor, "消费者偏好项目")?;
    let relationship =
        owned_relationship(store, consumer_project_id, actor.user_id, relationship_id)?;
    store.consumer_owned_open_commerce_preference_disclosure(
        &relationship.id,
        &relationship.subject_alias,
        &relationship.merchant_id,
        &relationship.status,
    )
}

pub(crate) fn upsert_disclosure(
    store: &Store,
    consumer_project_id: &str,
    relationship_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: UpsertConsumerPreferenceDisclosureRequest,
) -> Result<ConsumerPreferenceDisclosure> {
    ensure_project_actor(actor, "消费者偏好项目")?;
    let relationship =
        owned_relationship(store, consumer_project_id, actor.user_id, relationship_id)?;
    if relationship.status != "active" {
        bail!("只有有效的消费者关系才能披露偏好");
    }
    if !relationship
        .scopes
        .iter()
        .any(|scope| scope == RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER)
    {
        bail!("消费者关系未授权 preference.remember");
    }
    let profile = store
        .open_commerce_consumer_preference_profile(consumer_project_id, actor.user_id)?
        .ok_or_else(|| anyhow::anyhow!("请先保存消费者偏好档案"))?;
    let shared_fields = validate_shared_fields(request.shared_fields)?;
    let preferences = disclosure_snapshot(&profile.preferences, &shared_fields)?;
    let disclosure = store.upsert_open_commerce_consumer_preference_disclosure(
        &relationship,
        &shared_fields,
        &preferences,
        profile.revision,
    )?;
    store.record_open_commerce_audit(
        consumer_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_preference_disclosure.upserted",
        "consumer_preference_disclosure",
        &relationship.id,
        &json!({
            "merchant_id": relationship.merchant_id,
            "subject_alias": relationship.subject_alias,
            "shared_fields": disclosure.shared_fields,
            "profile_revision": disclosure.profile_revision
        }),
    )?;
    Ok(disclosure)
}

pub(crate) fn delete_disclosure(
    store: &Store,
    consumer_project_id: &str,
    relationship_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<DeleteConsumerPreferenceDisclosureResult> {
    ensure_project_actor(actor, "消费者偏好项目")?;
    let relationship =
        owned_relationship(store, consumer_project_id, actor.user_id, relationship_id)?;
    let deleted = store.delete_open_commerce_consumer_preference_disclosure(
        consumer_project_id,
        actor.user_id,
        &relationship.id,
    )?;
    let result = DeleteConsumerPreferenceDisclosureResult {
        relationship_id: relationship.id,
        deleted,
    };
    store.record_open_commerce_audit(
        consumer_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_preference_disclosure.deleted",
        "consumer_preference_disclosure",
        &result.relationship_id,
        &json!({
            "merchant_id": relationship.merchant_id,
            "subject_alias": relationship.subject_alias,
            "deleted": result.deleted
        }),
    )?;
    Ok(result)
}

pub(crate) fn list_consumer_disclosures(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<ConsumerPreferenceDisclosure>> {
    ensure_project_actor(actor, "消费者偏好项目")?;
    store.list_open_commerce_consumer_preference_disclosures(
        consumer_project_id,
        actor.user_id,
        limit,
    )
}

pub(crate) fn list_merchant_disclosures(
    store: &Store,
    merchant_project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<ConsumerPreferenceDisclosure>> {
    ensure_project_actor(actor, "商户项目")?;
    store.list_open_commerce_merchant_preference_disclosures(
        merchant_project_id,
        merchant_id,
        limit,
    )
}

pub(crate) fn normalize_preferences(
    mut preferences: ConsumerPreferences,
) -> Result<ConsumerPreferences> {
    preferences.categories = normalize_values(preferences.categories, 20, "偏好类别")?;
    preferences.tags = normalize_values(preferences.tags, 40, "偏好标签")?;
    preferences.city = normalize_optional(preferences.city, MAX_CITY_LEN, "城市")?;
    if let Some(value) = preferences.max_unit_price_micros {
        if !(0..=MAX_PRICE_MICROS).contains(&value) {
            bail!("最大调用价格超出允许范围");
        }
    }
    Ok(preferences)
}

fn ensure_project_actor(actor: &OpenCommerceActor<'_>, label: &str) -> Result<()> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于{label}");
    }
    Ok(())
}

fn owned_relationship(
    store: &Store,
    consumer_project_id: &str,
    consumer_user_id: &str,
    relationship_id: &str,
) -> Result<crate::open_commerce_relationship_model::OpenCommerceConsumerRelationship> {
    let relationship_id = relationship_id.trim();
    if relationship_id.is_empty() || relationship_id.chars().count() > 120 {
        bail!("消费者关系凭证 ID 长度必须为 1 到 120 个字符");
    }
    store
        .consumer_owned_open_commerce_relationship(
            consumer_project_id,
            consumer_user_id,
            relationship_id,
        )?
        .ok_or_else(|| anyhow::anyhow!("消费者关系凭证不存在"))
}

fn normalize_values(values: Vec<String>, max: usize, label: &str) -> Result<Vec<String>> {
    if values.len() > max {
        bail!("{label}数量不能超过 {max}");
    }
    let mut normalized = Vec::with_capacity(values.len());
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > MAX_PREFERENCE_VALUE_LEN || value.chars().any(char::is_control) {
            bail!("{label}单项长度不能超过 {MAX_PREFERENCE_VALUE_LEN} 个字符且不能包含控制字符");
        }
        if !normalized
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(value))
        {
            normalized.push(value.to_string());
        }
    }
    Ok(normalized)
}

fn normalize_optional(value: Option<String>, max: usize, label: &str) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max || value.chars().any(char::is_control) {
        bail!("{label}长度不能超过 {max} 个字符且不能包含控制字符");
    }
    Ok(Some(value.to_string()))
}

fn validate_shared_fields(mut fields: Vec<String>) -> Result<Vec<String>> {
    fields = fields
        .into_iter()
        .map(|field| field.trim().to_string())
        .filter(|field| !field.is_empty())
        .collect();
    fields.sort();
    fields.dedup();
    if fields.is_empty() || fields.len() > 4 {
        bail!("偏好披露必须选择 1 到 4 个字段");
    }
    if fields.iter().any(|field| {
        !matches!(
            field.as_str(),
            PREFERENCE_FIELD_CATEGORIES
                | PREFERENCE_FIELD_TAGS
                | PREFERENCE_FIELD_CITY
                | PREFERENCE_FIELD_MAX_UNIT_PRICE
        )
    }) {
        bail!("偏好披露包含未支持的字段");
    }
    Ok(fields)
}

fn disclosure_snapshot(
    profile: &ConsumerPreferences,
    fields: &[String],
) -> Result<DisclosedConsumerPreferences> {
    let has = |field: &str| fields.iter().any(|value| value == field);
    let categories = if has(PREFERENCE_FIELD_CATEGORIES) {
        if profile.categories.is_empty() {
            bail!("偏好档案没有可披露的类别");
        }
        Some(profile.categories.clone())
    } else {
        None
    };
    let tags = if has(PREFERENCE_FIELD_TAGS) {
        if profile.tags.is_empty() {
            bail!("偏好档案没有可披露的标签");
        }
        Some(profile.tags.clone())
    } else {
        None
    };
    let city = if has(PREFERENCE_FIELD_CITY) {
        Some(
            profile
                .city
                .clone()
                .ok_or_else(|| anyhow::anyhow!("偏好档案没有可披露的城市"))?,
        )
    } else {
        None
    };
    let max_unit_price_micros = if has(PREFERENCE_FIELD_MAX_UNIT_PRICE) {
        Some(
            profile
                .max_unit_price_micros
                .ok_or_else(|| anyhow::anyhow!("偏好档案没有可披露的价格上限"))?,
        )
    } else {
        None
    };
    Ok(DisclosedConsumerPreferences {
        categories,
        tags,
        city,
        max_unit_price_micros,
    })
}
