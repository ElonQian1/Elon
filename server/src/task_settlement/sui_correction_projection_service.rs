use anyhow::{anyhow, Result};

use crate::store::Store;

use super::{
    model::{SUI_INTEGRITY_CONFLICT, SUI_INTEGRITY_VERIFIED},
    sui_correction_model::{
        CreateSuiCorrectionProjectionPackage, SuiCorrectionProjectionPackage,
        SUI_CORRECTION_PACKAGE_SCHEMA,
    },
    sui_correction_projection, sui_projection,
};

pub(super) fn prepare(
    store: &Store,
    project_id: &str,
    correction_id: &str,
    actor_user_id: &str,
    target_network: &str,
) -> Result<SuiCorrectionProjectionPackage> {
    let target_network = sui_projection::normalized_target_network(target_network)?;
    let detail = correction_detail(store, project_id, correction_id)?;
    let (envelope, source_bundle_digest) =
        sui_correction_projection::envelope_and_source_digest(&detail)?;
    let projection_digest =
        sui_correction_projection::projection_digest(target_network, &envelope)?;
    let envelope_json = serde_json::to_string(&envelope)?;
    let reversal_receipt_id = detail
        .correction
        .reversal_receipt_id
        .as_deref()
        .ok_or_else(|| anyhow!("纠正缺少冲销凭证"))?;
    let replacement_receipt_id = detail
        .correction
        .replacement_receipt_id
        .as_deref()
        .ok_or_else(|| anyhow!("纠正缺少替换凭证"))?;
    let package = store.create_task_sui_correction_projection_package(
        CreateSuiCorrectionProjectionPackage {
            project_id,
            correction_id,
            reversal_receipt_id,
            replacement_receipt_id,
            target_network,
            package_schema: SUI_CORRECTION_PACKAGE_SCHEMA,
            projection_digest: &projection_digest,
            source_bundle_digest: &source_bundle_digest,
            envelope_json: &envelope_json,
            created_by_user_id: actor_user_id,
        },
    )?;
    verify(store, project_id, &package.id)
}

pub(super) fn list(store: &Store, project_id: &str) -> Result<Vec<SuiCorrectionProjectionPackage>> {
    store
        .list_task_sui_correction_projection_packages(project_id, 100)?
        .into_iter()
        .map(|package| with_dispute_readiness(store, package))
        .collect()
}

pub(super) fn detail(
    store: &Store,
    project_id: &str,
    projection_id: &str,
) -> Result<SuiCorrectionProjectionPackage> {
    let package = store
        .task_sui_correction_projection_package(project_id, projection_id)?
        .ok_or_else(|| anyhow!("Sui 纠正投影包不存在"))?;
    with_dispute_readiness(store, package)
}

pub(super) fn verify(
    store: &Store,
    project_id: &str,
    projection_id: &str,
) -> Result<SuiCorrectionProjectionPackage> {
    let package = detail(store, project_id, projection_id)?;
    let detail = correction_detail(store, project_id, &package.correction_id)?;
    let (expected_envelope, expected_source_digest) =
        sui_correction_projection::envelope_and_source_digest(&detail)?;
    let expected_projection_digest =
        sui_correction_projection::projection_digest(&package.target_network, &expected_envelope)?;
    let matches = package.package_schema == SUI_CORRECTION_PACKAGE_SCHEMA
        && package.reversal_receipt_id == expected_envelope.reversal.receipt_id
        && package.replacement_receipt_id == expected_envelope.replacement.receipt_id
        && package.envelope == expected_envelope
        && package.projection_digest == expected_projection_digest
        && package.source_bundle_digest == expected_source_digest;
    let error = (!matches).then_some(
        "纠正投影包与当前不可变纠正记录、双腿凭证或 v1 投影规则不一致；禁止交给网络适配器",
    );
    let package = store.update_task_sui_correction_projection_integrity(
        project_id,
        projection_id,
        if matches {
            SUI_INTEGRITY_VERIFIED
        } else {
            SUI_INTEGRITY_CONFLICT
        },
        error,
    )?;
    with_dispute_readiness(store, package)
}

fn correction_detail(
    store: &Store,
    project_id: &str,
    correction_id: &str,
) -> Result<super::model::SettlementCorrectionDetail> {
    store
        .task_settlement_correction_detail(project_id, correction_id)?
        .ok_or_else(|| anyhow!("影子结算纠正流程不存在"))
}

fn with_dispute_readiness(
    store: &Store,
    mut package: SuiCorrectionProjectionPackage,
) -> Result<SuiCorrectionProjectionPackage> {
    if package.submission_readiness == "adapter_required"
        && store.task_settlement_has_blocking_dispute(
            &package.project_id,
            &package.replacement_receipt_id,
        )?
    {
        package.submission_readiness = "dispute_blocked".to_string();
    }
    Ok(package)
}

#[cfg(test)]
#[path = "sui_correction_projection_service_tests.rs"]
mod tests;
