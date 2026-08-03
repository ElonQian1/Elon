//! Developer-App admission attestation, review, and suspension state machine.

use anyhow::{bail, Result};
use chrono::Utc;
use serde_json::json;

use crate::{
    open_commerce_developer_admission_model::{
        DeveloperAppAdmission, DeveloperAppAdmissionReviewItem, ReviewDeveloperAppAdmissionRequest,
        SubmitDeveloperAppAdmissionRequest,
    },
    open_commerce_developer_manifest_service::managed_app,
    open_commerce_service::OpenCommerceActor,
    store::Store,
};

pub(crate) fn current_admission(
    store: &Store,
    project_id: &str,
    app_record_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<Option<DeveloperAppAdmission>> {
    managed_app(store, project_id, app_record_id, actor)?;
    store.open_commerce_developer_app_admission(app_record_id)
}

pub(crate) fn submit_admission(
    store: &Store,
    project_id: &str,
    app_record_id: &str,
    request: SubmitDeveloperAppAdmissionRequest,
    actor: &OpenCommerceActor<'_>,
) -> Result<DeveloperAppAdmission> {
    if !request.information_attested {
        bail!("申请人必须确认主体声明真实且有权提交");
    }
    let app = managed_app(store, project_id, app_record_id, actor)?;
    require_current_approved_manifest(&app, request.expected_manifest_revision)?;
    let organization_name = normalize_claim(&request.organization_name, 2, 160, "主体名称")?;
    let jurisdiction = normalize_claim(&request.jurisdiction, 2, 80, "注册地区")?;
    let registration_id = normalize_claim(&request.registration_id, 2, 120, "登记编号")?;
    let attested_at = Utc::now().to_rfc3339();
    let admission = store.submit_open_commerce_developer_app_admission(
        project_id,
        app_record_id,
        request.expected_manifest_revision,
        &organization_name,
        &jurisdiction,
        &registration_id,
        &attested_at,
    )?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "developer_app.admission_submitted",
        "developer_app_admission",
        &admission.id,
        &json!({
            "app_id": app.app_id,
            "manifest_revision": admission.manifest_revision,
            "organization_claim_present": true,
            "registration_claim_present": true,
            "production_credential_issued": false,
        }),
    )?;
    Ok(admission)
}

pub(crate) fn list_reviewable_admissions(
    store: &Store,
    limit: usize,
) -> Result<Vec<DeveloperAppAdmissionReviewItem>> {
    store
        .list_reviewable_open_commerce_developer_app_admissions(limit)?
        .into_iter()
        .map(|admission| {
            let app = store.open_commerce_developer_app_by_record_id(&admission.app_record_id)?;
            Ok(DeveloperAppAdmissionReviewItem { app, admission })
        })
        .collect()
}

pub(crate) fn review_admission(
    store: &Store,
    app_record_id: &str,
    request: ReviewDeveloperAppAdmissionRequest,
    reviewer_user_id: &str,
) -> Result<DeveloperAppAdmission> {
    let decision = request.decision.trim().to_ascii_lowercase();
    if !matches!(
        decision.as_str(),
        "approved" | "changes_requested" | "suspended"
    ) {
        bail!("准入决定必须为 approved、changes_requested 或 suspended");
    }
    let app = store.open_commerce_developer_app_by_record_id(app_record_id)?;
    require_current_approved_manifest(&app, request.expected_manifest_revision)?;
    let note = normalize_note(
        &request.note,
        matches!(decision.as_str(), "changes_requested" | "suspended"),
    )?;
    let risk_tier = if decision == "approved" {
        Some(normalize_risk_tier(&request.risk_tier)?)
    } else {
        None
    };
    let admission = store.review_open_commerce_developer_app_admission(
        app_record_id,
        request.expected_manifest_revision,
        reviewer_user_id,
        &decision,
        risk_tier.as_deref(),
        note.as_deref(),
    )?;
    store.record_open_commerce_audit(
        &app.project_id,
        reviewer_user_id,
        Some("platform-admin"),
        "developer_app.admission_reviewed",
        "developer_app_admission",
        &admission.id,
        &json!({
            "app_id": app.app_id,
            "manifest_revision": admission.manifest_revision,
            "decision": decision,
            "risk_tier": admission.risk_tier,
            "review_note": note,
            "production_credential_issued": false,
            "network_access_enabled": false,
        }),
    )?;
    Ok(admission)
}

fn require_current_approved_manifest(
    app: &crate::open_commerce_developer_model::OpenCommerceDeveloperApp,
    expected_revision: i64,
) -> Result<()> {
    if app.status != "active" {
        bail!("开发者应用已停用");
    }
    if app.manifest_revision != expected_revision {
        bail!("App 资料已变化，请刷新后重试");
    }
    if app.manifest_status != "approved" {
        bail!("App 当前资料修订尚未通过审核");
    }
    if app.domain_verification_status != "verified"
        || app.domain_verification_revision != Some(expected_revision)
    {
        bail!("App 当前资料修订的主页域名控制证明无效");
    }
    Ok(())
}

fn normalize_claim(value: &str, min: usize, max: usize, label: &str) -> Result<String> {
    let value = value.trim();
    if value.chars().count() < min
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("{label}长度或字符无效");
    }
    Ok(value.to_string())
}

fn normalize_note(value: &str, required: bool) -> Result<Option<String>> {
    let value = value.trim();
    if value.chars().count() > 1000 {
        bail!("准入审核说明最多 1000 个字符");
    }
    if required && value.is_empty() {
        bail!("退回或暂停准入时必须填写说明");
    }
    Ok((!value.is_empty()).then(|| value.to_string()))
}

fn normalize_risk_tier(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if !matches!(value.as_str(), "low" | "standard" | "enhanced") {
        bail!("批准准入时必须选择 low、standard 或 enhanced 风险层级");
    }
    Ok(value)
}
