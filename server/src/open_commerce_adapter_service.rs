use anyhow::{bail, Result};
use chrono::{Duration, Utc};
use serde_json::json;

use crate::{
    open_commerce_adapter_model::{
        OpenCommerceAdapterCredential, OpenCommerceAdapterCredentialIssue,
        OpenCommerceAdapterCredentialList, ADAPTER_CREDENTIAL_LIST_SCHEMA,
        ADAPTER_HANDOFF_CLAIM_SCOPE,
    },
    open_commerce_service::OpenCommerceActor,
    project_auth::can_edit,
    store::Store,
};

const BOUNDARY: [&str; 6] = [
    "适配器 Token 明文只在签发或轮换时返回一次，服务端只保存 SHA-256",
    "凭据始终包含 business_handoff.write，但任务领取与单条结果读取权限默认关闭",
    "撤销凭据或停用所属数据接入后，适配器鉴权立即失败",
    "机器凭据有效期为 1 至 366 天，到期后立即拒绝鉴权，续用必须显式轮换",
    "只有轮换时显式开启后，凭据才会增加 business_handoff.claim",
    "机器凭据只提升回执来源权威，不代表平台独立核验订单、支付、履约或退款",
];

pub(crate) fn list_credentials(
    store: &Store,
    project_id: &str,
) -> Result<OpenCommerceAdapterCredentialList> {
    Ok(OpenCommerceAdapterCredentialList {
        schema: ADAPTER_CREDENTIAL_LIST_SCHEMA,
        project_id: project_id.trim().to_string(),
        credentials: store.list_project_open_commerce_adapter_credentials(project_id)?,
        boundary: BOUNDARY.to_vec(),
    })
}

pub(crate) fn rotate_credential(
    store: &Store,
    project_id: &str,
    integration_id: &str,
    expires_in_days: i64,
    allow_task_claims: bool,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceAdapterCredentialIssue> {
    require_editor(actor.project_role)?;
    let expires_at = expiration_from_days(expires_in_days)?;
    let issue = store.rotate_open_commerce_adapter_credential(
        project_id,
        integration_id,
        actor.user_id,
        &expires_at,
        allow_task_claims,
    )?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "adapter_credential.rotated",
        "adapter_credential",
        &issue.credential.id,
        &json!({
            "integration_id":issue.credential.integration_id,
            "credential_version":issue.credential.credential_version,
            "scopes":issue.credential.scopes,
            "task_claims_enabled":issue.credential.scopes.iter().any(|scope| scope == ADAPTER_HANDOFF_CLAIM_SCOPE),
            "token_visible_once":true,
            "expires_at":issue.credential.expires_at
        }),
    )?;
    Ok(issue)
}

fn expiration_from_days(expires_in_days: i64) -> Result<String> {
    if !(1..=366).contains(&expires_in_days) {
        bail!("适配器凭据有效期必须在 1 至 366 天之间");
    }
    Ok((Utc::now() + Duration::days(expires_in_days)).to_rfc3339())
}

pub(crate) fn revoke_credential(
    store: &Store,
    project_id: &str,
    credential_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceAdapterCredential> {
    require_editor(actor.project_role)?;
    let credential = store.revoke_open_commerce_adapter_credential(project_id, credential_id)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "adapter_credential.revoked",
        "adapter_credential",
        &credential.id,
        &json!({
            "integration_id":credential.integration_id,
            "credential_version":credential.credential_version,
            "status":credential.status
        }),
    )?;
    Ok(credential)
}

fn require_editor(role: Option<&str>) -> Result<()> {
    if !role.is_some_and(can_edit) {
        bail!("只有项目编辑者可以管理适配器机器凭据");
    }
    Ok(())
}
