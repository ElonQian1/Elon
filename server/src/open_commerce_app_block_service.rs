use anyhow::{bail, Result};
use serde_json::json;

use crate::{
    open_commerce_app_block_model::{
        BlockOpenCommerceAppRequest, OpenCommerceAppBlock, OpenCommerceAppBlockOutcome,
        OpenCommerceAppBlocked,
    },
    open_commerce_model::{normalize_app_id, CreateGrantRequest},
    project_auth::can_edit,
    store::Store,
};

pub(crate) fn list_blocks(store: &Store, project_id: &str) -> Result<Vec<OpenCommerceAppBlock>> {
    store.list_project_open_commerce_app_blocks(project_id)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn block_app(
    store: &Store,
    project_id: &str,
    actor_user_id: &str,
    actor_app_id: &str,
    project_role: &str,
    request: BlockOpenCommerceAppRequest,
) -> Result<OpenCommerceAppBlockOutcome> {
    require_editor(project_role)?;
    let outcome = store.block_open_commerce_app(project_id, actor_user_id, request)?;
    store.record_open_commerce_audit(
        project_id,
        actor_user_id,
        Some(actor_app_id),
        "app_block.activated",
        "merchant_app_block",
        &outcome.block.id,
        &json!({
            "merchant_id": outcome.block.merchant_id,
            "requester_app_id": outcome.block.requester_app_id,
            "reason_code": outcome.block.reason_code,
            "revoked_grants": outcome.revoked_grants,
            "canceled_authorization_requests": outcome.canceled_authorization_requests,
            "grants_restored": 0
        }),
    )?;
    Ok(outcome)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn unblock_app(
    store: &Store,
    project_id: &str,
    block_id: &str,
    actor_user_id: &str,
    actor_app_id: &str,
    project_role: &str,
) -> Result<OpenCommerceAppBlockOutcome> {
    require_editor(project_role)?;
    let outcome = store.unblock_open_commerce_app(project_id, block_id, actor_user_id)?;
    store.record_open_commerce_audit(
        project_id,
        actor_user_id,
        Some(actor_app_id),
        "app_block.released",
        "merchant_app_block",
        &outcome.block.id,
        &json!({
            "merchant_id": outcome.block.merchant_id,
            "requester_app_id": outcome.block.requester_app_id,
            "grants_restored": 0,
            "requires_new_authorization": true
        }),
    )?;
    Ok(outcome)
}

pub(crate) fn ensure_app_allowed(
    store: &Store,
    merchant_id: &str,
    requester_app_id: &str,
    bypass_for_project_editor: bool,
) -> Result<()> {
    let requester_app_id = normalize_app_id(requester_app_id)?;
    if bypass_for_project_editor || matches!(requester_app_id.as_str(), "pc-web" | "mcp-client") {
        return Ok(());
    }
    if store
        .active_open_commerce_app_block(merchant_id, &requester_app_id)?
        .is_some()
    {
        return Err(OpenCommerceAppBlocked { requester_app_id }.into());
    }
    Ok(())
}

pub(crate) fn ensure_grant_allowed(store: &Store, request: &CreateGrantRequest) -> Result<()> {
    ensure_app_allowed(store, &request.merchant_id, &request.grantee_app_id, false)
}

fn require_editor(role: &str) -> Result<()> {
    if !can_edit(role) {
        bail!("当前调用方没有项目编辑权限");
    }
    Ok(())
}
