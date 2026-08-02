use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde_json::json;

use crate::{
    open_commerce_model::normalize_app_id,
    open_commerce_relationship_model::{
        CreateConsumerRelationshipRequest, OpenCommerceConsumerRelationship,
        RenewConsumerRelationshipRequest, RELATIONSHIP_SCOPE_MEMBERSHIP_LINK,
        RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER,
    },
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

pub(crate) fn create_relationship(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateConsumerRelationshipRequest,
) -> Result<OpenCommerceConsumerRelationship> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于消费者关系项目");
    }
    store.published_open_commerce_merchant_detail(&request.merchant_id)?;
    let merchant = store.open_commerce_merchant(&request.merchant_id)?;
    let source_app_id = validate_source_app(
        store,
        consumer_project_id,
        actor.user_id,
        actor.app_id,
        &request.source_app_id,
    )?;
    let scopes = validate_relationship_scopes(request.scopes)?;
    let purpose = validate_purpose(&request.purpose)?;
    let expires_at = validate_expiration(&request.expires_at)?;
    let relationship = store.replace_open_commerce_consumer_relationship(
        consumer_project_id,
        actor.user_id,
        &merchant.project_id,
        &merchant.id,
        &source_app_id,
        &scopes,
        &purpose,
        &expires_at,
    )?;
    store.record_open_commerce_audit(
        consumer_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_relationship.created",
        "consumer_relationship",
        &relationship.id,
        &json!({
            "merchant_id": relationship.merchant_id,
            "source_app_id": relationship.source_app_id,
            "subject_alias": relationship.subject_alias,
            "scopes": relationship.scopes,
            "expires_at": relationship.expires_at
        }),
    )?;
    Ok(relationship)
}

pub(crate) fn list_consumer_relationships(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<OpenCommerceConsumerRelationship>> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于消费者关系项目");
    }
    store.list_open_commerce_consumer_relationships(consumer_project_id, actor.user_id, limit)
}

pub(crate) fn list_merchant_relationships(
    store: &Store,
    merchant_project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<OpenCommerceConsumerRelationship>> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于商户项目");
    }
    store.list_open_commerce_merchant_relationships(merchant_project_id, merchant_id, limit)
}

pub(crate) fn revoke_relationship(
    store: &Store,
    consumer_project_id: &str,
    relationship_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceConsumerRelationship> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于消费者关系项目");
    }
    let relationship = store.revoke_open_commerce_consumer_relationship(
        consumer_project_id,
        actor.user_id,
        relationship_id,
    )?;
    store.record_open_commerce_audit(
        consumer_project_id,
        actor.user_id,
        Some(actor.app_id),
        "consumer_relationship.revoked",
        "consumer_relationship",
        &relationship.id,
        &json!({
            "merchant_id": relationship.merchant_id,
            "subject_alias": relationship.subject_alias
        }),
    )?;
    Ok(relationship)
}

pub(crate) fn renew_relationship(
    store: &Store,
    consumer_project_id: &str,
    relationship_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: RenewConsumerRelationshipRequest,
) -> Result<OpenCommerceConsumerRelationship> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于消费者关系项目");
    }
    let relationship_id = relationship_id.trim();
    if relationship_id.is_empty() || relationship_id.chars().count() > 120 {
        bail!("消费者关系凭证 ID 长度必须为 1 到 120 个字符");
    }
    let source = store
        .consumer_owned_open_commerce_relationship(
            consumer_project_id,
            actor.user_id,
            relationship_id,
        )?
        .ok_or_else(|| anyhow::anyhow!("消费者关系凭证不存在"))?;
    if let Some(existing) = store.existing_open_commerce_consumer_relationship_renewal(
        consumer_project_id,
        actor.user_id,
        relationship_id,
    )? {
        return Ok(existing);
    }

    store.published_open_commerce_merchant_detail(&source.merchant_id)?;
    let source_app_id = validate_source_app(
        store,
        consumer_project_id,
        actor.user_id,
        actor.app_id,
        &request.source_app_id,
    )?;
    let expires_at = validate_expiration(&request.expires_at)?;
    let (relationship, created) = store.renew_open_commerce_consumer_relationship(
        consumer_project_id,
        actor.user_id,
        relationship_id,
        &source_app_id,
        &expires_at,
    )?;
    if created {
        store.record_open_commerce_audit(
            consumer_project_id,
            actor.user_id,
            Some(actor.app_id),
            "consumer_relationship.renewed",
            "consumer_relationship",
            &relationship.id,
            &json!({
                "renewed_from_relationship_id": source.id,
                "previous_subject_alias": source.subject_alias,
                "merchant_id": relationship.merchant_id,
                "source_app_id": relationship.source_app_id,
                "subject_alias": relationship.subject_alias,
                "expires_at": relationship.expires_at
            }),
        )?;
    }
    Ok(relationship)
}

fn validate_source_app(
    store: &Store,
    consumer_project_id: &str,
    actor_user_id: &str,
    actor_app_id: &str,
    requested_app_id: &str,
) -> Result<String> {
    let source_app_id = if actor_app_id == "pc-web" {
        normalize_app_id(requested_app_id)?
    } else {
        let actor_app_id = normalize_app_id(actor_app_id)?;
        let requested_app_id = normalize_app_id(requested_app_id)?;
        if requested_app_id != "pc-web" && requested_app_id != actor_app_id {
            bail!("AI 代理不能冒充其他 App 创建消费者关系");
        }
        actor_app_id
    };
    if source_app_id == "mcp-client" && actor_app_id != "mcp-client" {
        bail!("pc-web 不能冒充 MCP 系统身份创建消费者关系");
    }
    if source_app_id == "pc-web" || source_app_id == "mcp-client" {
        return Ok(source_app_id);
    }
    let app =
        store.ensure_open_commerce_developer_app_owned_by_user(&source_app_id, actor_user_id)?;
    if app.project_id != consumer_project_id.trim() {
        bail!("来源 App 不属于当前消费者项目");
    }
    Ok(source_app_id)
}

fn validate_relationship_scopes(scopes: Vec<String>) -> Result<Vec<String>> {
    let mut normalized = scopes
        .into_iter()
        .map(|scope| scope.trim().to_string())
        .filter(|scope| !scope.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    if normalized.is_empty() {
        bail!("消费者关系至少需要一个授权范围");
    }
    if normalized.iter().any(|scope| {
        !matches!(
            scope.as_str(),
            RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER | RELATIONSHIP_SCOPE_MEMBERSHIP_LINK
        )
    }) {
        bail!("消费者关系包含未支持的授权范围");
    }
    Ok(normalized)
}

fn validate_purpose(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 500 {
        bail!("消费者关系用途长度必须为 1 到 500 个字符");
    }
    Ok(value.to_string())
}

fn validate_expiration(value: &str) -> Result<String> {
    let parsed = DateTime::parse_from_rfc3339(value.trim())
        .context("expires_at 必须是 RFC 3339 时间")?
        .with_timezone(&Utc);
    let now = Utc::now();
    if parsed <= now {
        bail!("消费者关系过期时间必须晚于当前时间");
    }
    if parsed > now + Duration::days(366) {
        bail!("消费者关系有效期不能超过 366 天");
    }
    Ok(parsed.to_rfc3339())
}
