//! Service-free V272 refresh input derived from one exact renewed-route carrier.

use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::external_pool_adapter_task_protocol_conformance::{
        server_task_protocol_conformance_fixture_catalog,
        server_task_protocol_conformance_profile_catalog, TASK_PROTOCOL_CONFORMANCE_CONFIRMATION,
    },
    store::{
        compute_external_pool_adapter_provider_active_successor::CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority,
        compute_external_pool_adapter_sandbox_reattestation::current_external_pool_adapter_sandbox_reattestation_head_authority_on,
    },
};

use super::super::{
    read::run_head_by_release_on, types::CreateExternalPoolAdapterTaskProtocolConformanceRun,
};

pub(in crate::store) fn build_external_pool_adapter_task_protocol_active_refresh_input_on(
    transaction: &Transaction<'_>,
    carrier: &CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'_, '_>,
    checked_at: &str,
) -> Result<CreateExternalPoolAdapterTaskProtocolConformanceRun> {
    if carrier.checked_at() != checked_at || carrier.renewed_route().checked_at() != checked_at {
        bail!("V272 refresh input reused a different checked_at anchor");
    }
    let historical = carrier.historical_activation();
    let root = &historical.activation_root().activation_root;
    let receipt = historical.receipt();
    let sandbox = current_external_pool_adapter_sandbox_reattestation_head_authority_on(
        transaction,
        &root.registry_release_id,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("V272 refresh input lacks current V252"))?;
    let runtime = carrier.runtime_compatibility().verification();
    let profile = server_task_protocol_conformance_profile_catalog()?;
    let fixture = server_task_protocol_conformance_fixture_catalog()?;
    let predecessor = run_head_by_release_on(transaction, &root.registry_release_id)?;
    let predecessor_identity = predecessor.as_ref().map(|stored| {
        (
            stored.receipt.run_receipt_id.clone(),
            stored.receipt.run_receipt_digest.clone(),
        )
    });
    let idempotency_key = predecessor_identity
        .as_ref()
        .map(|(_, digest)| digest.clone())
        .unwrap_or_else(|| receipt.activation_receipt_digest.clone());
    let recorded_by_admin_user_id = receipt.activation.audit.activated_by_actor_user_id.clone();
    let idempotency_scope =
        format!("v272:task-protocol-conformance:create:{recorded_by_admin_user_id}");
    Ok(CreateExternalPoolAdapterTaskProtocolConformanceRun {
        registry_release_id: root.registry_release_id.clone(),
        expected_registry_release_digest: root.registry_release_digest.clone(),
        sandbox_reattestation_receipt_id: sandbox.receipt().reattestation_receipt_id.clone(),
        expected_sandbox_reattestation_receipt_digest: sandbox
            .receipt()
            .reattestation_receipt_digest
            .clone(),
        runtime_compatibility_verification_receipt_id: runtime.verification_receipt_id.clone(),
        expected_runtime_compatibility_verification_receipt_digest: runtime
            .verification_receipt_digest
            .clone(),
        expected_task_protocol_profile_digest: profile.profile_digest.clone(),
        expected_fixture_catalog_digest: fixture.catalog_digest.clone(),
        provider_binding_id: root.provider_binding_id.clone(),
        expected_provider_binding_digest: root.provider_binding_digest.clone(),
        expected_installation_receipt_id: root.installation_receipt_id.clone(),
        expected_installation_receipt_digest: root.installation_receipt_digest.clone(),
        predecessor_run_receipt_id: predecessor_identity.as_ref().map(|(id, _)| id.clone()),
        expected_predecessor_run_receipt_digest: predecessor_identity
            .as_ref()
            .map(|(_, digest)| digest.clone()),
        recorded_by_admin_user_id,
        idempotency_scope,
        idempotency_key,
        confirmation: TASK_PROTOCOL_CONFORMANCE_CONFIRMATION.into(),
    })
}
