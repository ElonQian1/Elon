use anyhow::{anyhow, bail, Result};
use serde_json::json;

use crate::{
    open_commerce_consumer,
    open_commerce_developer_model::CreateAuthorizationRequest,
    open_commerce_portability_import_service,
    open_commerce_portability_reauthorization_model::{
        CreatePortabilityReauthorizationRequest, CreatePortabilityRelationshipMappingRequest,
        PortabilityReauthorizationResult, PortabilityRelationshipMapping,
        PORTABILITY_RELATIONSHIP_MAPPING_SCHEMA,
    },
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

pub(crate) fn create_mapping(
    store: &Store,
    destination_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreatePortabilityRelationshipMappingRequest,
) -> Result<PortabilityRelationshipMapping> {
    ensure_consumer_project_actor(actor)?;
    if !request.confirmed_by_user {
        bail!("关系迁移映射必须由用户明确确认");
    }
    let import_id = normalize_id(&request.import_id, "导入记录")?;
    let source_relationship_id = normalize_id(&request.source_relationship_id, "来源关系")?;
    let target_merchant_id = normalize_id(&request.target_merchant_id, "目标商户")?;
    let import_record = open_commerce_portability_import_service::get_import(
        store,
        destination_project_id,
        &import_id,
        actor,
    )?;
    let source_relationship = import_record
        .package
        .payload
        .relationships
        .iter()
        .find(|relationship| relationship.id == source_relationship_id)
        .ok_or_else(|| anyhow!("导入数据包不包含该来源关系"))?;
    let target_merchant = store.open_commerce_merchant(&target_merchant_id)?;
    if target_merchant.status != "active"
        || !store.open_commerce_directory_is_published(&target_merchant.id)?
    {
        bail!("目标商户未在当前开放目录有效发布");
    }
    let (mapping, created) = store.save_portability_relationship_mapping(
        destination_project_id,
        actor.user_id,
        &import_record.id,
        &source_relationship.id,
        &source_relationship.merchant_id,
        &target_merchant.id,
        &target_merchant.project_id,
    )?;
    if created {
        store.record_open_commerce_audit(
            destination_project_id,
            actor.user_id,
            Some(actor.app_id),
            "consumer_portability.relationship_mapping_created",
            "portability_relationship_mapping",
            &mapping.id,
            &json!({
                "import_id": mapping.import_id,
                "source_relationship_id": mapping.source_relationship_id,
                "source_merchant_id": mapping.source_merchant_id,
                "target_merchant_id": mapping.target_merchant_id,
                "source_trust_status": import_record.trust_status,
                "authority": "consumer_confirmed",
            }),
        )?;
    }
    Ok(mapping)
}

pub(crate) fn list_mappings(
    store: &Store,
    destination_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<PortabilityRelationshipMapping>> {
    ensure_consumer_project_actor(actor)?;
    let mappings = store.list_portability_relationship_mappings(
        destination_project_id,
        actor.user_id,
        limit,
    )?;
    if mappings.iter().any(|mapping| {
        mapping.schema != PORTABILITY_RELATIONSHIP_MAPPING_SCHEMA
            || !matches!(mapping.status.as_str(), "active" | "revoked")
    }) {
        bail!("消费者关系迁移映射记录无效");
    }
    Ok(mappings)
}

pub(crate) fn revoke_mapping(
    store: &Store,
    destination_project_id: &str,
    mapping_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<PortabilityRelationshipMapping> {
    ensure_consumer_project_actor(actor)?;
    let mapping_id = normalize_id(mapping_id, "关系迁移映射")?;
    let mapping = store.revoke_portability_relationship_mapping(
        destination_project_id,
        actor.user_id,
        &mapping_id,
    )?;
    store.record_open_commerce_audit(
        destination_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_portability.relationship_mapping_revoked",
        "portability_relationship_mapping",
        &mapping.id,
        &json!({"target_merchant_id": mapping.target_merchant_id}),
    )?;
    Ok(mapping)
}

pub(crate) fn create_reauthorization(
    store: &Store,
    destination_project_id: &str,
    mapping_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreatePortabilityReauthorizationRequest,
) -> Result<PortabilityReauthorizationResult> {
    ensure_consumer_project_actor(actor)?;
    if !request.confirmed_by_user {
        bail!("重新授权申请必须由用户明确确认");
    }
    let mapping_id = normalize_id(mapping_id, "关系迁移映射")?;
    let mapping = store
        .owned_portability_relationship_mapping(destination_project_id, actor.user_id, &mapping_id)?
        .ok_or_else(|| anyhow!("消费者关系迁移映射不存在"))?;
    if mapping.status != "active" {
        bail!("消费者关系迁移映射已撤销");
    }
    let import_record = open_commerce_portability_import_service::get_import(
        store,
        destination_project_id,
        &mapping.import_id,
        actor,
    )?;
    let source_relationship = import_record
        .package
        .payload
        .relationships
        .iter()
        .find(|relationship| relationship.id == mapping.source_relationship_id)
        .ok_or_else(|| anyhow!("导入数据包中的来源关系已不可用"))?;
    if request.scopes.is_empty() {
        bail!("重新授权申请至少需要一个能力范围");
    }
    if request.scopes.iter().any(|scope| {
        !source_relationship
            .scopes
            .iter()
            .any(|source| source == scope)
    }) {
        bail!("重新授权范围不能超出来源关系原有范围");
    }
    let authorization_request = open_commerce_consumer::create_authorization_request(
        store,
        actor.user_id,
        CreateAuthorizationRequest {
            merchant_id: mapping.target_merchant_id.clone(),
            requester_app_id: request.requester_app_id,
            scopes: request.scopes,
            purpose: request.purpose,
        },
    )?;
    store.record_open_commerce_audit(
        destination_project_id,
        actor.user_id,
        Some(&authorization_request.requester_app_id),
        "consumer_portability.reauthorization_requested",
        "portability_relationship_mapping",
        &mapping.id,
        &json!({
            "authorization_request_id": authorization_request.id,
            "target_merchant_id": mapping.target_merchant_id,
            "scopes": authorization_request.scopes,
            "old_grant_restored": false,
        }),
    )?;
    Ok(PortabilityReauthorizationResult {
        schema: "open_commerce.portability_reauthorization_result.v1",
        mapping,
        authorization_request,
        old_grant_restored: false,
    })
}

fn normalize_id(value: &str, label: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 160 || value.chars().any(char::is_control) {
        bail!("{label} ID 长度或格式无效");
    }
    Ok(value.to_string())
}

fn ensure_consumer_project_actor(actor: &OpenCommerceActor<'_>) -> Result<()> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于消费者项目");
    }
    Ok(())
}
