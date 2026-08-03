//! Guarded production credential issuance, listing, and emergency revocation.

use anyhow::{bail, Result};
use chrono::{Duration, Utc};
use serde_json::json;

use crate::{
    open_commerce_developer_credential_model::{
        production_credentials_enabled, DeveloperProductionCredential,
        DeveloperProductionCredentialSecret, IssueDeveloperProductionCredentialRequest,
        RevokeDeveloperProductionCredentialRequest,
    },
    open_commerce_developer_manifest_service::managed_app,
    open_commerce_model::normalize_capability_key,
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

pub(crate) fn list_credentials(
    store: &Store,
    project_id: &str,
    app_record_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<Vec<DeveloperProductionCredential>> {
    managed_app(store, project_id, app_record_id, actor)?;
    store.list_open_commerce_developer_production_credentials(project_id, app_record_id, 20)
}

pub(crate) fn issue_credential(
    store: &Store,
    app_record_id: &str,
    request: IssueDeveloperProductionCredentialRequest,
    issuer_user_id: &str,
) -> Result<DeveloperProductionCredentialSecret> {
    if !production_credentials_enabled() {
        bail!("生产开发者凭据功能默认关闭，运营方尚未显式启用");
    }
    let app = store.open_commerce_developer_app_by_record_id(app_record_id)?;
    require_current_app_state(&app, request.expected_manifest_revision)?;
    let admission = store
        .open_commerce_developer_app_admission(app_record_id)?
        .ok_or_else(|| anyhow::anyhow!("App 尚未提交生产准入审查"))?;
    if admission.status != "approved" || admission.manifest_revision != app.manifest_revision {
        bail!("App 当前资料修订尚未通过生产准入审查");
    }
    let scopes = normalize_scopes(request.scopes)?;
    if scopes.is_empty() {
        bail!("生产凭据至少需要一项明确能力范围");
    }
    if scopes.iter().any(|scope| {
        !app.requested_scopes
            .iter()
            .any(|approved| approved == scope)
    }) {
        bail!("生产凭据能力范围不能超出当前已审核 App 资料");
    }
    let max_days = risk_tier_max_days(admission.risk_tier.as_deref())?;
    if !(1..=max_days).contains(&request.expires_in_days) {
        bail!("当前风险层级的生产凭据有效期必须为 1 至 {max_days} 天");
    }
    let expires_at = (Utc::now() + Duration::days(request.expires_in_days)).to_rfc3339();
    let secret = store.issue_open_commerce_developer_production_credential(
        &app,
        &admission.id,
        &scopes,
        issuer_user_id,
        &expires_at,
    )?;
    store.record_open_commerce_audit(
        &app.project_id,
        issuer_user_id,
        Some("platform-admin"),
        "developer_app.production_credential_issued",
        "developer_production_credential",
        &secret.credential.id,
        &json!({
            "app_id": app.app_id,
            "manifest_revision": app.manifest_revision,
            "admission_id": admission.id,
            "scopes": scopes,
            "expires_at": expires_at,
            "token_persisted": false,
            "funds_moved": false,
        }),
    )?;
    Ok(secret)
}

pub(crate) fn revoke_credential(
    store: &Store,
    project_id: &str,
    app_record_id: &str,
    credential_id: &str,
    request: RevokeDeveloperProductionCredentialRequest,
    actor: &OpenCommerceActor<'_>,
) -> Result<DeveloperProductionCredential> {
    let app = managed_app(store, project_id, app_record_id, actor)?;
    let reason = normalize_revocation_reason(&request.reason)?;
    let credential = store.revoke_open_commerce_developer_production_credential(
        project_id,
        app_record_id,
        credential_id,
        &reason,
    )?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "developer_app.production_credential_revoked",
        "developer_production_credential",
        &credential.id,
        &json!({
            "app_id": app.app_id,
            "reason": reason,
            "status": credential.status,
            "funds_moved": false,
        }),
    )?;
    Ok(credential)
}

fn require_current_app_state(
    app: &crate::open_commerce_developer_model::OpenCommerceDeveloperApp,
    expected_revision: i64,
) -> Result<()> {
    if app.status != "active" || app.manifest_revision != expected_revision {
        bail!("App 已停用或资料已变化，请刷新后重试");
    }
    if app.manifest_status != "approved"
        || app.domain_verification_status != "verified"
        || app.domain_verification_revision != Some(expected_revision)
    {
        bail!("App 当前资料审核或主页域名控制证明无效");
    }
    Ok(())
}

fn normalize_scopes(scopes: Vec<String>) -> Result<Vec<String>> {
    if scopes.len() > 32 {
        bail!("单个生产凭据最多包含 32 项能力");
    }
    let mut scopes = scopes
        .into_iter()
        .filter(|scope| !scope.trim().is_empty())
        .map(|scope| normalize_capability_key(&scope))
        .collect::<Result<Vec<_>>>()?;
    scopes.sort();
    scopes.dedup();
    Ok(scopes)
}

fn risk_tier_max_days(risk_tier: Option<&str>) -> Result<i64> {
    match risk_tier {
        Some("low") => Ok(366),
        Some("standard") => Ok(180),
        Some("enhanced") => Ok(90),
        _ => bail!("准入记录缺少有效风险层级"),
    }
}

fn normalize_revocation_reason(reason: &str) -> Result<String> {
    let reason = reason.trim();
    if reason.chars().count() < 4
        || reason.chars().count() > 500
        || reason.chars().any(char::is_control)
    {
        bail!("撤销原因必须为 4 至 500 个有效字符");
    }
    Ok(reason.to_string())
}
