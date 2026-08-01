//! Merchant-controlled directory publication and sanitized discovery.

use anyhow::{bail, Result};
use serde_json::json;

use crate::{
    open_commerce_directory_model::{
        OpenCommerceDirectoryMerchantDetail, OpenCommerceDirectoryPublication,
    },
    open_commerce_service::OpenCommerceActor,
    project_auth::can_edit,
    store::Store,
};

pub(crate) fn discover_merchants(
    store: &Store,
    query: Option<&str>,
    capability_key: Option<&str>,
    limit: usize,
) -> Result<Vec<OpenCommerceDirectoryMerchantDetail>> {
    store.search_published_open_commerce_merchants(query, capability_key, limit)
}

pub(crate) fn discover_merchant(
    store: &Store,
    merchant_id: &str,
) -> Result<OpenCommerceDirectoryMerchantDetail> {
    store.published_open_commerce_merchant_detail(merchant_id)
}

pub(crate) fn set_publication(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
    published: bool,
) -> Result<OpenCommerceDirectoryPublication> {
    if !actor.project_role.is_some_and(can_edit) {
        bail!("当前调用方没有项目编辑权限");
    }
    let publication = store.set_open_commerce_directory_publication(
        project_id,
        merchant_id,
        actor.user_id,
        published,
    )?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        if published {
            "directory.published"
        } else {
            "directory.unpublished"
        },
        "merchant",
        merchant_id,
        &json!({
            "status": publication.status,
            "directory_revision": publication.revision
        }),
    )?;
    Ok(publication)
}
