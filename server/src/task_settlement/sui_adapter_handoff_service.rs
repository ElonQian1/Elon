//! Re-verifies chain-off packages before creating deterministic offline handoff bundles.

use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

use crate::store::Store;

use super::{
    model::{SUI_INTEGRITY_VERIFIED, SUI_NETWORK_NOT_SUBMITTED},
    sui_adapter_handoff_model::{
        SuiAdapterHandoffBundle, SuiAdapterHandoffConstraints, SuiAdapterHandoffPayload,
        SUI_ADAPTER_HANDOFF_SCHEMA,
    },
    sui_correction_projection_service, sui_projection_service,
};

pub(super) fn standard(
    store: &Store,
    project_id: &str,
    projection_id: &str,
) -> Result<SuiAdapterHandoffBundle> {
    let package = sui_projection_service::verify(store, project_id, projection_id)?;
    ensure_exportable(
        &package.integrity_status,
        &package.submission_readiness,
        &package.network_submission,
        package.submission_attempts,
    )?;
    bundle(SuiAdapterHandoffPayload {
        schema: SUI_ADAPTER_HANDOFF_SCHEMA,
        package_kind: "standard",
        project_id: package.project_id,
        projection_package_id: package.id,
        source_id: package.settlement_receipt_id,
        target_network: package.target_network,
        package_schema: package.package_schema,
        projection_digest: package.projection_digest,
        source_digest: package.source_receipt_digest,
        envelope: serde_json::to_value(package.envelope)?,
        shadow_only: true,
        atomic_bundle: false,
        network_submission: package.network_submission,
        submission_attempts: package.submission_attempts,
        package_created_at: package.created_at,
        constraints: offline_constraints(),
    })
}

pub(super) fn correction(
    store: &Store,
    project_id: &str,
    projection_id: &str,
) -> Result<SuiAdapterHandoffBundle> {
    let package = sui_correction_projection_service::verify(store, project_id, projection_id)?;
    ensure_exportable(
        &package.integrity_status,
        &package.submission_readiness,
        &package.network_submission,
        package.submission_attempts,
    )?;
    bundle(SuiAdapterHandoffPayload {
        schema: SUI_ADAPTER_HANDOFF_SCHEMA,
        package_kind: "correction",
        project_id: package.project_id,
        projection_package_id: package.id,
        source_id: package.correction_id,
        target_network: package.target_network,
        package_schema: package.package_schema,
        projection_digest: package.projection_digest,
        source_digest: package.source_bundle_digest,
        envelope: serde_json::to_value(package.envelope)?,
        shadow_only: true,
        atomic_bundle: true,
        network_submission: package.network_submission,
        submission_attempts: package.submission_attempts,
        package_created_at: package.created_at,
        constraints: offline_constraints(),
    })
}

fn ensure_exportable(
    integrity_status: &str,
    submission_readiness: &str,
    network_submission: &str,
    submission_attempts: i64,
) -> Result<()> {
    if integrity_status != SUI_INTEGRITY_VERIFIED {
        bail!("Sui 投影包完整性未通过，不能导出适配器交接包");
    }
    if submission_readiness != "adapter_required" {
        bail!("Sui 投影包存在争议或完整性阻断，不能导出适配器交接包");
    }
    if network_submission != SUI_NETWORK_NOT_SUBMITTED || submission_attempts != 0 {
        bail!("Sui 投影包已进入网络提交生命周期，不能作为离线交接包导出");
    }
    Ok(())
}

fn bundle(payload: SuiAdapterHandoffPayload) -> Result<SuiAdapterHandoffBundle> {
    let handoff_digest = hex::encode(Sha256::digest(serde_json::to_vec(&payload)?));
    Ok(SuiAdapterHandoffBundle {
        payload,
        handoff_digest,
    })
}

fn offline_constraints() -> SuiAdapterHandoffConstraints {
    SuiAdapterHandoffConstraints {
        allowed_adapter_action: "offline_preflight_only",
        signature_present: false,
        transaction_broadcast: false,
        finality_verified: false,
        funds_moved: false,
    }
}
