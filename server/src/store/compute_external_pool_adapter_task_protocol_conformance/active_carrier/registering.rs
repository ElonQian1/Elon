//! V272 input and final carrier derived only from the typed registering activation chain.

use anyhow::{bail, ensure, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::external_pool_adapter_task_protocol_conformance::{
        server_task_protocol_conformance_fixture_catalog,
        server_task_protocol_conformance_profile_catalog, TASK_PROTOCOL_CONFORMANCE_CONFIRMATION,
        TASK_PROTOCOL_CONFORMANCE_RELATIONAL_CURRENT_STATUS,
    },
    store::{
        compute_external_pool_adapter_provider_active_successor::PreparedExternalPoolAdapterProviderActiveSuccessorTarget,
        compute_external_pool_adapter_runtime_bundle::ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject,
        compute_external_pool_adapter_sandbox_reattestation::current_external_pool_adapter_sandbox_reattestation_head_authority_on,
        ExternalPoolAdapterTaskProtocolConformanceRuntime,
    },
};

use super::{
    super::{
        current::relational_currentness_on,
        read::{run_by_id_on, run_head_by_release_on},
        roots::{canonical_time, domain_roots_from_parts},
        types::CreateExternalPoolAdapterTaskProtocolConformanceRun,
    },
    types::PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier,
};

pub(in crate::store) fn build_external_pool_adapter_task_protocol_registering_activation_input_on(
    transaction: &Transaction<'_>,
    prepared: &PreparedExternalPoolAdapterProviderActiveSuccessorTarget<'_, '_>,
    checked_at: &str,
) -> Result<CreateExternalPoolAdapterTaskProtocolConformanceRun> {
    ensure!(
        prepared.authority_checked_at() == checked_at,
        "registering V272 input reused a different checked_at anchor"
    );
    let root = &prepared.activation_root().activation_root;
    let sandbox = current_external_pool_adapter_sandbox_reattestation_head_authority_on(
        transaction,
        &root.registry_release_id,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("registering V272 input lacks current V252"))?;
    let runtime = prepared.runtime_compatibility().verification();
    let profile = server_task_protocol_conformance_profile_catalog()?;
    let fixture = server_task_protocol_conformance_fixture_catalog()?;
    ensure!(
        root.task_protocol_profile_digest == profile.profile_digest,
        "registering V272 input task profile differs from the frozen V274 target"
    );
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
        .unwrap_or_else(|| prepared.activation_root().activation_root_digest.clone());
    let recorded_by_admin_user_id = root.provider_owner_account_id.clone();
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
        idempotency_scope: format!(
            "v272:task-protocol-conformance:create:{recorded_by_admin_user_id}"
        ),
        idempotency_key,
        confirmation: TASK_PROTOCOL_CONFORMANCE_CONFIRMATION.into(),
        recorded_by_admin_user_id,
    })
}

pub(in crate::store) fn prepare_external_pool_adapter_task_protocol_planned_active_carrier_on<
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    no_work: &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, 'tx, 'conn>,
    run_receipt_id: &str,
    expected_run_receipt_digest: &str,
    runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
) -> Result<PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier<'tx, 'conn>> {
    let Some(stored) = run_by_id_on(transaction, run_receipt_id)? else {
        bail!("registering V272 receipt disappeared before final V277 append");
    };
    ensure!(
        stored.receipt.run_receipt_digest == expected_run_receipt_digest,
        "registering V272 receipt digest changed before final V277 append"
    );
    let relational = relational_currentness_on(transaction, run_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("registering V272 currentness row disappeared"))?;
    let checked_at = no_work.evidence_checked_at();
    let receipt = &stored.receipt;
    ensure!(
        relational.current_status == TASK_PROTOCOL_CONFORMANCE_RELATIONAL_CURRENT_STATUS
            && canonical_time(&receipt.run.post_cleanup_checked_at)? <= canonical_time(checked_at)?
            && canonical_time(&receipt.run.expires_at)? > canonical_time(checked_at)?
            && runtime
                .process_custody()
                .attests_task_protocol_conformance_seal(
                    &receipt.run_receipt_id,
                    &stored.receipt_integrity_digest,
                    &receipt.run_receipt_digest,
                    &stored.runtime_custody_epoch_digest,
                    &stored.process_hmac_seal,
                    &receipt.run.expires_at,
                )?,
        "registering V272 receipt is historical or lacks exact process custody"
    );

    let observation = no_work.observation();
    let release = observation
        .companion()
        .target()
        .profile()
        .candidate()
        .registry()
        .release();
    let projected = domain_roots_from_parts(
        release,
        observation.vulnerability(),
        observation.sandbox(),
        observation.runtime_compatibility(),
    )?;
    let expected = &receipt.run;
    ensure!(
        projected.registry_release == expected.registry_release
            && projected.vulnerability_reattestation == expected.vulnerability_reattestation
            && projected.sandbox_reattestation == expected.sandbox_reattestation
            && projected.sandbox_verifier_key == expected.sandbox_verifier_key
            && projected.runtime_compatibility == expected.runtime_compatibility,
        "registering V272 receipt roots differ from the final no-work observation"
    );
    let profile = server_task_protocol_conformance_profile_catalog()?;
    let fixture = server_task_protocol_conformance_fixture_catalog()?;
    ensure!(
        expected.task_protocol_profile_id == profile.profile.profile_id
            && expected.task_protocol_profile_revision == profile.profile.profile_revision
            && expected.task_protocol_profile_digest == profile.profile_digest
            && expected.fixture_catalog_id == fixture.catalog.catalog_id
            && expected.fixture_catalog_revision == fixture.catalog.catalog_revision
            && expected.fixture_catalog_digest == fixture.catalog_digest,
        "registering V272 server catalogs changed before final V277 append"
    );
    PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier::new(
        transaction,
        no_work,
        stored.receipt,
    )
}
