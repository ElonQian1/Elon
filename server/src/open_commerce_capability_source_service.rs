use crate::{
    open_commerce_capability_source_model::{
        LinkCapabilitySourceRequest, OpenCommerceCapabilitySourceLink,
    },
    open_commerce_service::OpenCommerceActor,
    project_auth::can_edit,
    store::Store,
};
use anyhow::{bail, Result};
use serde_json::json;

pub(crate) fn link_source(
    store: &Store,
    project_id: &str,
    capability_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: LinkCapabilitySourceRequest,
) -> Result<OpenCommerceCapabilitySourceLink> {
    require_editor(actor)?;
    store.link_open_commerce_capability_source_with_audit(
        project_id,
        capability_id,
        actor.user_id,
        actor.app_id,
        request,
    )
}

pub(crate) fn remove_source(
    store: &Store,
    project_id: &str,
    capability_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<serde_json::Value> {
    require_editor(actor)?;
    let removed = store.remove_open_commerce_capability_source_link_with_audit(
        project_id,
        capability_id,
        actor.user_id,
        actor.app_id,
    )?;
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

#[cfg(test)]
#[path = "open_commerce_capability_source_tests.rs"]
mod tests;
