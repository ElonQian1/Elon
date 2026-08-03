use anyhow::{bail, Result};
use serde_json::json;

use crate::{
    open_commerce_capability_source_model::{
        LinkCapabilitySourceRequest, OpenCommerceCapabilitySourceLink,
    },
    open_commerce_service::OpenCommerceActor,
    project_auth::can_edit,
    store::Store,
};

pub(crate) fn link_source(
    store: &Store,
    project_id: &str,
    capability_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: LinkCapabilitySourceRequest,
) -> Result<OpenCommerceCapabilitySourceLink> {
    require_editor(actor)?;
    let link = store.upsert_open_commerce_capability_source_link(
        project_id,
        capability_id,
        actor.user_id,
        request,
    )?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "capability.source_linked",
        "capability_source_link",
        &link.id,
        &json!({
            "merchant_id": link.merchant_id,
            "capability_id": link.capability_id,
            "capability_version": link.capability_version,
            "integration_id": link.integration_id,
            "sync_receipt_id": link.sync_receipt_id,
            "data_domain": link.data_domain,
            "revision": link.revision,
            "externally_verified": false
        }),
    )?;
    Ok(link)
}

pub(crate) fn remove_source(
    store: &Store,
    project_id: &str,
    capability_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<serde_json::Value> {
    require_editor(actor)?;
    let removed = store.remove_open_commerce_capability_source_link(project_id, capability_id)?;
    if removed {
        store.record_open_commerce_audit(
            project_id,
            actor.user_id,
            Some(actor.app_id),
            "capability.source_unlinked",
            "capability",
            capability_id,
            &json!({"externally_verified": false}),
        )?;
    }
    Ok(json!({
        "schema": "open_commerce.capability_source_unlink.v1",
        "capability_id": capability_id,
        "removed": removed
    }))
}

fn require_editor(actor: &OpenCommerceActor<'_>) -> Result<()> {
    if actor.project_role.is_some_and(can_edit) {
        Ok(())
    } else {
        bail!("需要项目编辑权限")
    }
}
