use anyhow::{bail, Result};
use serde_json::json;

use crate::{
    open_commerce_data_request_model::{
        CreateConsumerDataErasureRequest, DecideConsumerDataRequest,
        OpenCommerceConsumerDataRequest,
    },
    open_commerce_service::OpenCommerceActor,
    project_auth::can_edit,
    store::Store,
};

pub(crate) fn create_erasure_request(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: CreateConsumerDataErasureRequest,
) -> Result<OpenCommerceConsumerDataRequest> {
    ensure_project_member(actor, "消费者数据请求项目")?;
    let relationship_id = require_id(&request.relationship_id, "relationship_id")?;
    let (data_request, created) = store.create_open_commerce_consumer_data_erasure_request(
        consumer_project_id,
        actor.user_id,
        &relationship_id,
    )?;
    if created {
        store.record_open_commerce_audit(
            consumer_project_id,
            actor.user_id,
            Some(actor.app_id),
            "consumer_data_erasure.requested",
            "consumer_data_request",
            &data_request.id,
            &json!({
                "relationship_id": data_request.relationship_id,
                "merchant_id": data_request.merchant_id,
                "subject_alias": data_request.subject_alias,
                "relationship_revoked": true
            }),
        )?;
    }
    Ok(data_request)
}

pub(crate) fn list_consumer_requests(
    store: &Store,
    consumer_project_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<OpenCommerceConsumerDataRequest>> {
    ensure_project_member(actor, "消费者数据请求项目")?;
    store.list_open_commerce_consumer_data_requests(consumer_project_id, actor.user_id, limit)
}

pub(crate) fn withdraw_request(
    store: &Store,
    consumer_project_id: &str,
    request_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceConsumerDataRequest> {
    ensure_project_member(actor, "消费者数据请求项目")?;
    let request_id = require_id(request_id, "request_id")?;
    let (current, changed) = store.withdraw_open_commerce_consumer_data_request(
        consumer_project_id,
        actor.user_id,
        &request_id,
    )?;
    if changed {
        store.record_open_commerce_audit(
            consumer_project_id,
            actor.user_id,
            Some(actor.app_id),
            "consumer_data_erasure.withdrawn",
            "consumer_data_request",
            &current.id,
            &json!({
                "merchant_id": current.merchant_id,
                "subject_alias": current.subject_alias
            }),
        )?;
    }
    Ok(current)
}

pub(crate) fn list_merchant_requests(
    store: &Store,
    merchant_project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
    limit: usize,
) -> Result<Vec<OpenCommerceConsumerDataRequest>> {
    ensure_project_member(actor, "商户项目")?;
    store.list_open_commerce_merchant_data_requests(merchant_project_id, merchant_id, limit)
}

pub(crate) fn decide_request(
    store: &Store,
    merchant_project_id: &str,
    merchant_id: &str,
    request_id: &str,
    actor: &OpenCommerceActor<'_>,
    decision: DecideConsumerDataRequest,
) -> Result<OpenCommerceConsumerDataRequest> {
    let role = actor
        .project_role
        .ok_or_else(|| anyhow::anyhow!("当前调用方不属于商户项目"))?;
    if !can_edit(role) {
        bail!("只有商户项目编辑者可以处理消费者数据请求");
    }
    let request_id = require_id(request_id, "request_id")?;
    let merchant_id = require_id(merchant_id, "merchant_id")?;
    let action = decision.action.trim();
    if !matches!(action, "accept" | "complete" | "reject") {
        bail!("消费者数据请求处理动作无效");
    }
    let note = normalize_note(action, &decision.note)?;
    let (current, changed) = store.decide_open_commerce_consumer_data_request(
        merchant_project_id,
        &merchant_id,
        &request_id,
        action,
        note.as_deref(),
    )?;
    if changed {
        store.record_open_commerce_audit(
            merchant_project_id,
            actor.user_id,
            Some(actor.app_id),
            &format!("consumer_data_erasure.{action}"),
            "consumer_data_request",
            &current.id,
            &json!({
                "merchant_id": current.merchant_id,
                "subject_alias": current.subject_alias,
                "status": current.status,
                "resolution_kind": current.resolution_kind
            }),
        )?;
    }
    Ok(current)
}

fn ensure_project_member(actor: &OpenCommerceActor<'_>, project_label: &str) -> Result<()> {
    if actor.project_role.is_none() {
        bail!("当前调用方不属于{project_label}");
    }
    Ok(())
}

fn require_id(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 120 {
        bail!("{field} 长度必须为 1 到 120 个字符");
    }
    Ok(value.to_string())
}

fn normalize_note(action: &str, value: &str) -> Result<Option<String>> {
    let value = value.trim();
    if value.chars().count() > 500 {
        bail!("消费者数据请求处理说明不能超过 500 个字符");
    }
    if matches!(action, "complete" | "reject") && value.is_empty() {
        bail!("完成或拒绝消费者数据请求时必须填写说明");
    }
    Ok((!value.is_empty()).then(|| value.to_string()))
}
