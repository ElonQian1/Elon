//! Developer App lifecycle and requester-side authorization management.

use anyhow::{bail, Result};
use serde_json::json;

use crate::{
    open_commerce_developer_model::{
        OpenCommerceAuthorizationRequest, OpenCommerceDeveloperApp,
        OpenCommerceDeveloperAppCredential,
    },
    open_commerce_service::OpenCommerceActor,
    project_auth::can_edit,
    store::Store,
};

pub(crate) fn disable_app(
    store: &Store,
    project_id: &str,
    app_record_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceDeveloperApp> {
    require_editor(actor)?;
    let current = store.open_commerce_developer_app_for_project(project_id, app_record_id)?;
    let pending =
        store.list_pending_open_commerce_authorization_requests_for_app(&current.app_id)?;
    let (app, canceled_count) =
        store.disable_open_commerce_developer_app(project_id, app_record_id)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "developer_app.disabled",
        "developer_app",
        &app.id,
        &json!({
            "app_id": app.app_id,
            "canceled_pending_requests": canceled_count
        }),
    )?;
    for request in pending.into_iter() {
        store.record_open_commerce_audit(
            &request.merchant_project_id,
            actor.user_id,
            Some(&app.app_id),
            "authorization.canceled",
            "authorization_request",
            &request.id,
            &json!({"reason":"developer_app_disabled"}),
        )?;
    }
    Ok(app)
}

pub(crate) fn reactivate_app(
    store: &Store,
    project_id: &str,
    app_record_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceDeveloperAppCredential> {
    require_editor(actor)?;
    let credential = store.reactivate_open_commerce_developer_app(project_id, app_record_id)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "developer_app.reactivated",
        "developer_app",
        &credential.app.id,
        &json!({"app_id":credential.app.app_id}),
    )?;
    Ok(credential)
}

pub(crate) fn list_outbound_requests(
    store: &Store,
    project_id: &str,
) -> Result<Vec<OpenCommerceAuthorizationRequest>> {
    store.list_requester_project_open_commerce_authorization_requests(project_id, 100)
}

pub(crate) fn cancel_outbound_request(
    store: &Store,
    project_id: &str,
    request_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceAuthorizationRequest> {
    require_editor(actor)?;
    let previous = store.open_commerce_authorization_request(request_id)?;
    let request =
        store.cancel_requester_open_commerce_authorization_request(project_id, request_id)?;
    if previous.status == "pending" && request.status == "canceled" {
        let details = json!({
            "requester_app_id": request.requester_app_id,
            "reason": "requester_canceled"
        });
        store.record_open_commerce_audit(
            project_id,
            actor.user_id,
            Some(actor.app_id),
            "authorization.request_canceled",
            "authorization_request",
            &request.id,
            &details,
        )?;
        store.record_open_commerce_audit(
            &request.merchant_project_id,
            actor.user_id,
            Some(&request.requester_app_id),
            "authorization.canceled",
            "authorization_request",
            &request.id,
            &details,
        )?;
    }
    Ok(request)
}

fn require_editor(actor: &OpenCommerceActor<'_>) -> Result<()> {
    if actor.project_role.is_some_and(can_edit) {
        Ok(())
    } else {
        bail!("当前调用方没有项目编辑权限")
    }
}
