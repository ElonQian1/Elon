//! Developer-App manifest validation, submission and review state transitions.

use anyhow::{bail, Result};
use serde_json::json;

use crate::{
    open_commerce_developer_model::{
        OpenCommerceDeveloperApp, ReviewDeveloperAppManifestRequest,
        UpdateDeveloperAppManifestRequest,
    },
    open_commerce_model::normalize_capability_key,
    open_commerce_service::OpenCommerceActor,
    project_auth::can_edit,
    store::Store,
};

pub(crate) fn update_manifest(
    store: &Store,
    project_id: &str,
    app_record_id: &str,
    request: UpdateDeveloperAppManifestRequest,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceDeveloperApp> {
    let current = managed_app(store, project_id, app_record_id, actor)?;
    if current.status != "active" {
        bail!("开发者应用已停用，请先重新启用");
    }
    let homepage_url = optional_https_url(request.homepage_url, "应用主页")?;
    let privacy_policy_url = optional_https_url(request.privacy_policy_url, "隐私政策")?;
    let terms_url = optional_https_url(request.terms_url, "服务条款")?;
    let support_email = optional_support_email(request.support_email)?;
    let requested_scopes = normalize_scopes(request.requested_scopes)?;
    let app = store.update_open_commerce_developer_app_manifest(
        project_id,
        app_record_id,
        request.expected_manifest_revision,
        homepage_url.as_deref(),
        privacy_policy_url.as_deref(),
        terms_url.as_deref(),
        support_email.as_deref(),
        &requested_scopes,
    )?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "developer_app.manifest_updated",
        "developer_app",
        &app.id,
        &json!({
            "app_id": app.app_id,
            "manifest_revision": app.manifest_revision,
            "requested_scopes": app.requested_scopes,
        }),
    )?;
    Ok(app)
}

pub(crate) fn submit_manifest(
    store: &Store,
    project_id: &str,
    app_record_id: &str,
    expected_revision: i64,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceDeveloperApp> {
    let current = managed_app(store, project_id, app_record_id, actor)?;
    require_complete_manifest(&current)?;
    let app = store.submit_open_commerce_developer_app_manifest(
        project_id,
        app_record_id,
        expected_revision,
    )?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "developer_app.manifest_submitted",
        "developer_app",
        &app.id,
        &json!({
            "app_id": app.app_id,
            "manifest_revision": app.manifest_revision,
            "requested_scopes": app.requested_scopes,
        }),
    )?;
    Ok(app)
}

pub(crate) fn review_manifest(
    store: &Store,
    app_record_id: &str,
    request: ReviewDeveloperAppManifestRequest,
    reviewer_user_id: &str,
) -> Result<OpenCommerceDeveloperApp> {
    let decision = request.decision.trim().to_ascii_lowercase();
    if !matches!(decision.as_str(), "approved" | "changes_requested") {
        bail!("审核决定必须为 approved 或 changes_requested");
    }
    let note = normalized_review_note(&request.note, decision == "changes_requested")?;
    let current = store.open_commerce_developer_app_by_record_id(app_record_id)?;
    require_complete_manifest(&current)?;
    let app = store.review_open_commerce_developer_app_manifest(
        app_record_id,
        request.expected_manifest_revision,
        reviewer_user_id,
        &decision,
        note.as_deref(),
    )?;
    store.record_open_commerce_audit(
        &app.project_id,
        reviewer_user_id,
        Some("platform-admin"),
        "developer_app.manifest_reviewed",
        "developer_app",
        &app.id,
        &json!({
            "app_id": app.app_id,
            "decision": decision,
            "manifest_revision": app.manifest_revision,
            "review_note": note,
            "production_credential_issued": false,
        }),
    )?;
    Ok(app)
}

pub(crate) fn managed_app(
    store: &Store,
    project_id: &str,
    app_record_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceDeveloperApp> {
    let role = actor.project_role.unwrap_or_default();
    if !can_edit(role) {
        bail!("当前调用方没有项目编辑权限");
    }
    let app = store.open_commerce_developer_app_for_project(project_id, app_record_id)?;
    if app.owner_user_id != actor.user_id && !matches!(role, "owner" | "admin") {
        bail!("只有 App 所有者或项目管理员可以维护审核资料");
    }
    Ok(app)
}

fn require_complete_manifest(app: &OpenCommerceDeveloperApp) -> Result<()> {
    if app.status != "active" {
        bail!("开发者应用已停用，请先重新启用");
    }
    if app.homepage_url.is_none()
        || app.privacy_policy_url.is_none()
        || app.terms_url.is_none()
        || app.support_email.is_none()
        || app.requested_scopes.is_empty()
    {
        bail!("提交审核前必须填写主页、隐私政策、服务条款、支持邮箱和申请能力");
    }
    if app.domain_verification_status != "verified"
        || app.domain_verification_revision != Some(app.manifest_revision)
    {
        bail!("提交审核前必须验证当前资料修订的应用主页域名");
    }
    Ok(())
}

fn optional_https_url(value: Option<String>, label: &str) -> Result<Option<String>> {
    let value = value.unwrap_or_default().trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 2048 {
        bail!("{label} URL 过长");
    }
    let url = reqwest::Url::parse(&value).map_err(|_| anyhow::anyhow!("{label} URL 无效"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        bail!("{label}必须是无账号信息和片段的 HTTPS URL");
    }
    Ok(Some(url.to_string()))
}

fn optional_support_email(value: Option<String>) -> Result<Option<String>> {
    let value = value.unwrap_or_default().trim().to_ascii_lowercase();
    if value.is_empty() {
        return Ok(None);
    }
    let parts = value.split('@').collect::<Vec<_>>();
    if value.len() > 254
        || parts.len() != 2
        || parts.iter().any(|part| part.is_empty())
        || value.chars().any(char::is_whitespace)
    {
        bail!("支持邮箱格式无效");
    }
    Ok(Some(value))
}

fn normalize_scopes(scopes: Vec<String>) -> Result<Vec<String>> {
    if scopes.len() > 32 {
        bail!("单个 App 最多申请 32 项能力");
    }
    let mut scopes = scopes
        .iter()
        .filter(|scope| !scope.trim().is_empty())
        .map(|scope| normalize_capability_key(scope))
        .collect::<Result<Vec<_>>>()?;
    scopes.sort();
    scopes.dedup();
    Ok(scopes)
}

fn normalized_review_note(value: &str, required: bool) -> Result<Option<String>> {
    let value = value.trim();
    if value.len() > 1000 {
        bail!("审核说明最多 1000 个字符");
    }
    if required && value.is_empty() {
        bail!("要求修改时必须填写审核说明");
    }
    Ok((!value.is_empty()).then(|| value.to_string()))
}
