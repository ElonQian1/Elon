use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, bail, Result};
use serde_json::{json, to_value, Value};

use crate::{
    open_commerce_consumer_model::ConsumerPreferences,
    open_commerce_consumer_preference_service,
    open_commerce_portability_import_model::ConsumerPortabilityImport,
    open_commerce_portability_import_service,
    open_commerce_portability_merge_model::{
        ApplyConsumerPortabilityMergeRequest, ConsumerPortabilityFieldSelection,
        ConsumerPortabilityFieldSource, ConsumerPortabilityMergeAdoption,
        ConsumerPortabilityMergeCandidate, ConsumerPortabilityMergeField,
        ConsumerPortabilityMergePlan, ConsumerPortabilityMergeSource,
        CreateConsumerPortabilityMergePlanRequest, RollbackConsumerPortabilityMergeRequest,
        CONSUMER_PORTABILITY_MERGE_ADOPTION_SCHEMA, CONSUMER_PORTABILITY_MERGE_PLAN_SCHEMA,
    },
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

const PREFERENCE_FIELDS: [&str; 5] = [
    "categories",
    "tags",
    "city",
    "max_unit_price_micros",
    "prefer_public",
];

struct MergeSourceProfile {
    import: ConsumerPortabilityImport,
    preferences: ConsumerPreferences,
}

pub(crate) fn merge_plan(
    store: &Store,
    destination_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateConsumerPortabilityMergePlanRequest,
) -> Result<ConsumerPortabilityMergePlan> {
    ensure_consumer_project_actor(actor)?;
    let sources = load_sources(store, destination_project_id, actor, request.import_ids)?;
    let current = open_commerce_consumer_preference_service::get_profile(
        store,
        destination_project_id,
        actor,
    )?;
    let empty = ConsumerPreferences::default();
    let current_preferences = current
        .as_ref()
        .map(|profile| &profile.preferences)
        .unwrap_or(&empty);
    let fields = PREFERENCE_FIELDS
        .iter()
        .map(|field| merge_field_plan(field, current_preferences, &sources))
        .collect::<Result<Vec<_>>>()?;
    Ok(ConsumerPortabilityMergePlan {
        schema: CONSUMER_PORTABILITY_MERGE_PLAN_SCHEMA.to_string(),
        current_profile_revision: current.map(|profile| profile.revision),
        sources: sources.iter().map(source_summary).collect(),
        fields,
        automatic_conflict_resolution: false,
        automatic_relationship_restore: false,
        automatic_business_write: false,
    })
}

pub(crate) fn apply_merge(
    store: &Store,
    destination_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: ApplyConsumerPortabilityMergeRequest,
) -> Result<ConsumerPortabilityMergeAdoption> {
    ensure_consumer_project_actor(actor)?;
    if !request.confirmed_by_user {
        bail!("多来源偏好合并必须由用户明确确认");
    }
    let sources = load_sources(store, destination_project_id, actor, request.import_ids)?;
    let current = open_commerce_consumer_preference_service::get_profile(
        store,
        destination_project_id,
        actor,
    )?;
    let current_revision = current.as_ref().map(|profile| profile.revision);
    if current_revision != request.expected_current_revision {
        bail!("消费者偏好档案已变化，请刷新多来源合并预演后重试");
    }
    let selections = normalize_selections(request.selections)?;
    let mut preferences = current
        .as_ref()
        .map(|profile| profile.preferences.clone())
        .unwrap_or_default();
    let source_by_id = sources
        .iter()
        .map(|source| (source.import.id.as_str(), source))
        .collect::<BTreeMap<_, _>>();
    let mut field_sources = Vec::with_capacity(selections.len());
    for selection in selections.values() {
        let source = source_by_id
            .get(selection.import_id.as_str())
            .ok_or_else(|| anyhow!("字段来源不属于本次选择的数据包"))?;
        apply_selected_field(
            &mut preferences,
            &source.preferences,
            selection.field.as_str(),
        )?;
        field_sources.push(ConsumerPortabilityFieldSource {
            field: selection.field.clone(),
            import_id: source.import.id.clone(),
            source_operator: source.import.source_operator.clone(),
            source_package_id: source.import.source_package_id.clone(),
            envelope_sha256: source.import.envelope_sha256.clone(),
            payload_sha256: source.import.payload_sha256.clone(),
            trust_status: source.import.trust_status.clone(),
        });
    }
    let preferences =
        open_commerce_consumer_preference_service::normalize_preferences(preferences)?;
    let source_import_ids = sources
        .iter()
        .map(|source| source.import.id.clone())
        .collect::<Vec<_>>();
    let adoption = store.apply_consumer_portability_preference_merge(
        &source_import_ids,
        &field_sources,
        destination_project_id,
        actor.user_id,
        request.expected_current_revision,
        &preferences,
    )?;
    store.record_open_commerce_audit(
        destination_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_portability.preferences_merged",
        "consumer_portability_merge_adoption",
        &adoption.id,
        &json!({
            "source_import_ids": &adoption.source_import_ids,
            "field_sources": &adoption.field_sources,
            "before_revision": adoption.before_revision,
            "resulting_revision": adoption.resulting_revision,
        }),
    )?;
    Ok(adoption)
}

pub(crate) fn list_merges(
    store: &Store,
    destination_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<ConsumerPortabilityMergeAdoption>> {
    ensure_consumer_project_actor(actor)?;
    let records = store.list_consumer_portability_preference_merges(
        destination_project_id,
        actor.user_id,
        limit,
    )?;
    if records.iter().any(|record| {
        record.schema != CONSUMER_PORTABILITY_MERGE_ADOPTION_SCHEMA
            || !matches!(record.status.as_str(), "applied" | "rolled_back")
            || record.source_import_ids.len() < 2
            || record.field_sources.is_empty()
    }) {
        bail!("消费者多来源偏好合并记录无效");
    }
    Ok(records)
}

pub(crate) fn rollback_merge(
    store: &Store,
    destination_project_id: &str,
    adoption_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: RollbackConsumerPortabilityMergeRequest,
) -> Result<ConsumerPortabilityMergeAdoption> {
    ensure_consumer_project_actor(actor)?;
    if !request.confirmed_by_user {
        bail!("回滚多来源偏好合并必须由用户明确确认");
    }
    let adoption_id = adoption_id.trim();
    if adoption_id.is_empty() || adoption_id.chars().count() > 120 {
        bail!("消费者多来源偏好合并记录 ID 长度必须为 1 到 120 个字符");
    }
    let adoption = store.rollback_consumer_portability_preference_merge(
        adoption_id,
        destination_project_id,
        actor.user_id,
        request.expected_current_revision,
    )?;
    store.record_open_commerce_audit(
        destination_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_portability.preferences_merge_rolled_back",
        "consumer_portability_merge_adoption",
        &adoption.id,
        &json!({
            "source_import_ids": &adoption.source_import_ids,
            "resulting_revision": adoption.resulting_revision,
            "rollback_revision": adoption.rollback_revision,
        }),
    )?;
    Ok(adoption)
}

fn load_sources(
    store: &Store,
    destination_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    import_ids: Vec<String>,
) -> Result<Vec<MergeSourceProfile>> {
    let import_ids = normalize_import_ids(import_ids)?;
    import_ids
        .into_iter()
        .map(|import_id| {
            let import = open_commerce_portability_import_service::get_import(
                store,
                destination_project_id,
                &import_id,
                actor,
            )?;
            let preferences = import
                .package
                .payload
                .preference_profile
                .as_ref()
                .map(|profile| profile.preferences.clone())
                .ok_or_else(|| anyhow!("数据包 {import_id} 不包含消费者偏好档案"))?;
            let preferences =
                open_commerce_consumer_preference_service::normalize_preferences(preferences)?;
            Ok(MergeSourceProfile {
                import,
                preferences,
            })
        })
        .collect()
}

fn normalize_import_ids(import_ids: Vec<String>) -> Result<Vec<String>> {
    let mut normalized = BTreeSet::new();
    for import_id in import_ids {
        let import_id = import_id.trim().to_string();
        if import_id.is_empty() || import_id.chars().count() > 120 {
            bail!("消费者数据包导入记录 ID 长度必须为 1 到 120 个字符");
        }
        if !normalized.insert(import_id) {
            bail!("多来源合并不能重复选择同一数据包");
        }
    }
    if !(2..=10).contains(&normalized.len()) {
        bail!("多来源合并必须选择 2 到 10 个数据包");
    }
    Ok(normalized.into_iter().collect())
}

fn normalize_selections(
    selections: Vec<ConsumerPortabilityFieldSelection>,
) -> Result<BTreeMap<String, ConsumerPortabilityFieldSelection>> {
    let mut normalized = BTreeMap::new();
    for selection in selections {
        let field = selection.field.trim().to_ascii_lowercase();
        if !PREFERENCE_FIELDS.contains(&field.as_str()) {
            bail!("偏好字段 {field} 不支持多来源合并");
        }
        let import_id = selection.import_id.trim().to_string();
        if import_id.is_empty() || import_id.chars().count() > 120 {
            bail!("字段来源数据包 ID 长度必须为 1 到 120 个字符");
        }
        if normalized
            .insert(
                field.clone(),
                ConsumerPortabilityFieldSelection { field, import_id },
            )
            .is_some()
        {
            bail!("同一偏好字段只能选择一个来源");
        }
    }
    if normalized.is_empty() {
        bail!("至少选择一个需要合并的偏好字段来源");
    }
    Ok(normalized)
}

fn source_summary(source: &MergeSourceProfile) -> ConsumerPortabilityMergeSource {
    ConsumerPortabilityMergeSource {
        import_id: source.import.id.clone(),
        source_operator: source.import.source_operator.clone(),
        source_package_id: source.import.source_package_id.clone(),
        source_package_schema: source.import.source_package_schema.clone(),
        envelope_sha256: source.import.envelope_sha256.clone(),
        payload_sha256: source.import.payload_sha256.clone(),
        trust_status: source.import.trust_status.clone(),
    }
}

fn merge_field_plan(
    field: &str,
    current: &ConsumerPreferences,
    sources: &[MergeSourceProfile],
) -> Result<ConsumerPortabilityMergeField> {
    let current_value = preference_value(current, field)?;
    let candidates = sources
        .iter()
        .map(|source| {
            let imported_value = preference_value(&source.preferences, field)?;
            Ok(ConsumerPortabilityMergeCandidate {
                import_id: source.import.id.clone(),
                source_operator: source.import.source_operator.clone(),
                source_package_id: source.import.source_package_id.clone(),
                trust_status: source.import.trust_status.clone(),
                differs_from_current: imported_value != current_value,
                imported_value,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let distinct_candidate_count = candidates
        .iter()
        .map(|candidate| serde_json::to_string(&candidate.imported_value))
        .collect::<serde_json::Result<BTreeSet<_>>>()?
        .len();
    Ok(ConsumerPortabilityMergeField {
        field: field.to_string(),
        current_value,
        candidates,
        distinct_candidate_count,
        conflict: distinct_candidate_count > 1,
    })
}

fn preference_value(preferences: &ConsumerPreferences, field: &str) -> Result<Value> {
    match field {
        "categories" => Ok(to_value(&preferences.categories)?),
        "tags" => Ok(to_value(&preferences.tags)?),
        "city" => Ok(to_value(&preferences.city)?),
        "max_unit_price_micros" => Ok(to_value(preferences.max_unit_price_micros)?),
        "prefer_public" => Ok(to_value(preferences.prefer_public)?),
        _ => bail!("未知偏好字段"),
    }
}

fn apply_selected_field(
    current: &mut ConsumerPreferences,
    source: &ConsumerPreferences,
    field: &str,
) -> Result<()> {
    if preference_value(current, field)? == preference_value(source, field)? {
        bail!("只能采用相对当前档案真实发生变化的偏好字段");
    }
    match field {
        "categories" => current.categories = source.categories.clone(),
        "tags" => current.tags = source.tags.clone(),
        "city" => current.city = source.city.clone(),
        "max_unit_price_micros" => {
            current.max_unit_price_micros = source.max_unit_price_micros;
        }
        "prefer_public" => current.prefer_public = source.prefer_public,
        _ => bail!("未知偏好字段"),
    }
    Ok(())
}

fn ensure_consumer_project_actor(actor: &OpenCommerceActor<'_>) -> Result<()> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于消费者项目");
    }
    Ok(())
}
