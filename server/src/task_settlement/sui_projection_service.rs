use anyhow::{anyhow, Result};

use crate::store::Store;

use super::{
    model::{
        CreateSuiProjectionPackage, SuiProjectionPackage, SUI_INTEGRITY_CONFLICT,
        SUI_INTEGRITY_VERIFIED, SUI_PROJECTION_PACKAGE_SCHEMA,
    },
    service::receipt_detail,
    sui_projection,
};

pub(super) fn prepare(
    store: &Store,
    project_id: &str,
    receipt_id: &str,
    actor_user_id: &str,
    target_network: &str,
) -> Result<SuiProjectionPackage> {
    let target_network = sui_projection::normalized_target_network(target_network)?;
    let detail = receipt_detail(store, project_id, receipt_id)?;
    let envelope = sui_projection::envelope(&detail.receipt)?;
    let envelope_json = sui_projection::envelope_json(&envelope)?;
    let projection_digest = sui_projection::projection_digest(target_network, &envelope)?;
    let source_receipt_digest = sui_projection::source_receipt_digest(&detail.receipt)?;
    let package = store.create_task_sui_projection_package(CreateSuiProjectionPackage {
        project_id,
        settlement_receipt_id: receipt_id,
        target_network,
        package_schema: SUI_PROJECTION_PACKAGE_SCHEMA,
        projection_digest: &projection_digest,
        source_receipt_digest: &source_receipt_digest,
        envelope_json: &envelope_json,
        created_by_user_id: actor_user_id,
    })?;
    verify(store, project_id, &package.id)
}

pub(super) fn list(store: &Store, project_id: &str) -> Result<Vec<SuiProjectionPackage>> {
    store.list_task_sui_projection_packages(project_id, 100)
}

pub(super) fn detail(
    store: &Store,
    project_id: &str,
    projection_id: &str,
) -> Result<SuiProjectionPackage> {
    store
        .task_sui_projection_package(project_id, projection_id)?
        .ok_or_else(|| anyhow!("Sui 投影包不存在"))
}

pub(super) fn verify(
    store: &Store,
    project_id: &str,
    projection_id: &str,
) -> Result<SuiProjectionPackage> {
    let package = detail(store, project_id, projection_id)?;
    let receipt = store
        .task_settlement_receipt(project_id, &package.settlement_receipt_id)?
        .ok_or_else(|| anyhow!("Sui 投影包绑定的影子凭证不存在"))?;
    let expected_envelope = sui_projection::envelope(&receipt)?;
    let expected_projection_digest =
        sui_projection::projection_digest(&package.target_network, &expected_envelope)?;
    let expected_source_digest = sui_projection::source_receipt_digest(&receipt)?;
    let matches = package.package_schema == SUI_PROJECTION_PACKAGE_SCHEMA
        && package.envelope == expected_envelope
        && package.projection_digest == expected_projection_digest
        && package.source_receipt_digest == expected_source_digest;
    let error =
        (!matches).then_some("投影包与当前不可变影子凭证或 v1 投影规则不一致；禁止交给网络适配器");
    store.update_task_sui_projection_integrity(
        project_id,
        projection_id,
        if matches {
            SUI_INTEGRITY_VERIFIED
        } else {
            SUI_INTEGRITY_CONFLICT
        },
        error,
    )
}

#[cfg(test)]
#[path = "sui_projection_service_tests.rs"]
mod tests;
