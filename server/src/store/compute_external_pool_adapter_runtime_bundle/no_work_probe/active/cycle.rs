//! Six-reopen active broker/Secret/no-work execution and final #5/#6 reproof.

use anyhow::{bail, Result};
use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetReceipt,
    store::{
        compute_external_pool_adapter_provider_active_successor::{
            current_external_pool_adapter_renewed_route_runtime_carrier_for_binding_on,
            CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority,
        },
        compute_external_pool_adapter_runtime_bundle::{
            current_external_pool_adapter_projected_active_runtime_bundle_authority_on,
            runtime::ExternalPoolAdapterPostCleanupCommitmentInput,
            secret_delivery::audit_projected_active_delivery_binding_on,
            ExternalPoolAdapterProviderRuntimeReadinessRuntime,
        },
        compute_external_pool_adapter_task_protocol_conformance::current_external_pool_adapter_task_protocol_conformance_for_renewed_route_carrier_on,
        compute_external_pool_adapter_upstream_transport_target::ExternalPoolAdapterInstallationReopener,
        ExternalPoolAdapterTaskProtocolConformanceRuntime, Store,
    },
};

use super::{
    preflight::durable_external_pool_adapter_active_no_work_probe_subject_on,
    types::{
        CurrentExternalPoolAdapterProjectedActiveNoWorkObservationAuthority,
        DurableExternalPoolAdapterActiveNoWorkProbeSubject,
    },
};
use crate::store::compute_external_pool_adapter_runtime_bundle::no_work_probe::execution::execute_external_pool_adapter_no_work_probe;

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::store) async fn with_projected_active_external_pool_adapter_no_work_observation<
        Pending,
        Output,
    >(
        &self,
        provider_binding_id: &str,
        expected_activation_receipt_id: &str,
        expected_activation_receipt_digest: &str,
        expected_task_protocol_run_receipt_id: &str,
        expected_task_protocol_run_receipt_digest: &str,
        target: &ExternalPoolAdapterUpstreamTransportTargetReceipt,
        reopen_prepared: &mut ExternalPoolAdapterInstallationReopener<'_>,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
        task_protocol_runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
        consume: impl FnOnce(
                &Transaction<'_>,
                &CurrentExternalPoolAdapterProjectedActiveNoWorkObservationAuthority<'_, '_, '_>,
            ) -> Result<Pending>
            + Send,
        postcommit: impl FnOnce(&Connection, Pending) -> Result<Output> + Send,
    ) -> Result<Option<Output>> {
        let mut preflight_subject = None;
        let Some(broker) = self
            .prepare_projected_active_external_pool_adapter_broker_tls_channel(
                target,
                reopen_prepared,
                |transaction, prepared, checked_at| {
                    let Some(carrier) =
                        current_external_pool_adapter_renewed_route_runtime_carrier_for_binding_on(
                            transaction,
                            provider_binding_id,
                            expected_activation_receipt_id,
                            expected_activation_receipt_digest,
                            prepared,
                            checked_at,
                        )?
                    else {
                        return Ok(false);
                    };
                    if carrier.target() != target {
                        bail!("active broker carrier selected a different V258 target");
                    }
                    let Some(task_protocol) = current_external_pool_adapter_task_protocol_conformance_for_renewed_route_carrier_on(
                        transaction,
                        carrier,
                        task_protocol_runtime,
                        checked_at,
                    )? else {
                        return Ok(false);
                    };
                    require_exact_task_protocol_identity(
                        &task_protocol,
                        expected_task_protocol_run_receipt_id,
                        expected_task_protocol_run_receipt_digest,
                    )?;
                    let observed =
                        durable_external_pool_adapter_active_no_work_probe_subject_on(&task_protocol)?;
                    if let Some(expected) = preflight_subject.as_ref() {
                        require_same_subject(expected, &task_protocol)?;
                    } else {
                        preflight_subject = Some(observed);
                    }
                    Ok(true)
                },
            )
            .await?
        else {
            return Ok(None);
        };
        let subject = preflight_subject
            .ok_or_else(|| anyhow::anyhow!("active broker returned without a typed subject"))?;

        let bundle_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let session_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let Some(delivery) = self
            .prepare_projected_active_external_pool_adapter_ephemeral_secret_delivery(
                provider_binding_id,
                expected_activation_receipt_id,
                expected_activation_receipt_digest,
                expected_task_protocol_run_receipt_id,
                expected_task_protocol_run_receipt_digest,
                bundle_prepared,
                session_prepared,
                runtime.bundle_root(),
                runtime.cgroup_parent(),
                runtime.process_custody(),
                task_protocol_runtime,
            )?
        else {
            return Ok(None);
        };
        let executed = execute_external_pool_adapter_no_work_probe(
            broker,
            delivery,
            &target.target_id,
            &target.target_digest,
        )
        .await?;

        let final_bundle_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let final_session_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let checked_at = now();
        let Some(bundle) =
            current_external_pool_adapter_projected_active_runtime_bundle_authority_on(
                &transaction,
                provider_binding_id,
                expected_activation_receipt_id,
                expected_activation_receipt_digest,
                final_bundle_prepared,
                runtime.bundle_root(),
                &checked_at,
            )?
        else {
            return Ok(None);
        };
        let Some(v272_carrier) =
            current_external_pool_adapter_renewed_route_runtime_carrier_for_binding_on(
                &transaction,
                provider_binding_id,
                expected_activation_receipt_id,
                expected_activation_receipt_digest,
                final_session_prepared,
                &checked_at,
            )?
        else {
            return Ok(None);
        };
        let Some(task_protocol) =
            current_external_pool_adapter_task_protocol_conformance_for_renewed_route_carrier_on(
                &transaction,
                v272_carrier,
                task_protocol_runtime,
                &checked_at,
            )?
        else {
            return Ok(None);
        };
        require_exact_task_protocol_identity(
            &task_protocol,
            expected_task_protocol_run_receipt_id,
            expected_task_protocol_run_receipt_digest,
        )?;
        require_same_carrier_subject(&subject, bundle.carrier())?;
        require_same_subject(&subject, &task_protocol)?;
        audit_projected_active_delivery_binding_on(
            &transaction,
            executed.cleaned.binding(),
            task_protocol.carrier(),
            task_protocol.receipt(),
            Some(&bundle),
            &checked_at,
        )?;
        if !runtime
            .process_custody()
            .attests_projected_active_runtime_bundle_identity_commitment(
                &bundle,
                executed
                    .cleaned
                    .binding()
                    .runtime_bundle_identity_commitment(),
            )?
        {
            bail!("active no-work bundle commitment changed before final reproof");
        }
        let bundle_commitment = runtime
            .process_custody()
            .projected_active_runtime_bundle_identity_commitment(&bundle)?;
        let expires_at = active_observation_expiry(&checked_at, &bundle, &task_protocol)?;
        let post_cleanup_commitment = runtime
            .process_custody()
            .post_cleanup_observation_commitment(
                &ExternalPoolAdapterPostCleanupCommitmentInput {
                    runtime_bundle_identity_commitment: &bundle_commitment,
                    receipt: &executed.receipt,
                    binding: executed.cleaned.binding(),
                    selected_address: executed.selected_address,
                    cleaned: &executed.cleaned,
                },
            )?;
        let observation = CurrentExternalPoolAdapterProjectedActiveNoWorkObservationAuthority::new(
            &executed.receipt,
            executed.cleaned.binding(),
            executed.selected_address,
            &bundle,
            &task_protocol,
            &executed.cleaned,
            &bundle_commitment,
            post_cleanup_commitment,
            checked_at.clone(),
            expires_at.clone(),
        );
        if !observation.no_work_observed() || now() >= expires_at {
            bail!("active no-work observation expired before final write");
        }
        let pending = consume(&transaction, &observation)?;
        if now() >= expires_at {
            bail!("active no-work observation expired before commit");
        }
        drop(observation);
        drop(task_protocol);
        drop(bundle);
        transaction.commit()?;
        postcommit(&connection, pending).map(Some)
    }
}

fn require_same_subject(
    expected: &DurableExternalPoolAdapterActiveNoWorkProbeSubject,
    task_protocol: &crate::store::compute_external_pool_adapter_task_protocol_conformance::CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'_, '_>,
) -> Result<()> {
    require_same_carrier_subject(expected, task_protocol.carrier())?;
    if task_protocol.receipt().run_receipt_id != expected.task_protocol_run_receipt_id()
        || task_protocol.receipt().run_receipt_digest != expected.task_protocol_run_receipt_digest()
        || task_protocol.checked_at() != task_protocol.carrier().checked_at()
    {
        bail!("active no-work V272 changed from its pre-connect subject");
    }
    Ok(())
}

fn require_same_carrier_subject(
    expected: &DurableExternalPoolAdapterActiveNoWorkProbeSubject,
    carrier: &CurrentExternalPoolAdapterRenewedRouteRuntimeCarrierAuthority<'_, '_>,
) -> Result<()> {
    let historical = carrier.historical_activation();
    let route = carrier.renewed_route();
    let credential = carrier.credential().receipt();
    let compatibility = carrier.runtime_compatibility().verification();
    if historical.receipt() != expected.activation_receipt()
        || historical.activation_root() != expected.activation_root()
        || historical.active_provider() != expected.active_provider()
        || carrier.target() != expected.transport_target()
        || carrier.companion() != expected.companion()
        || route.receipt().route_renewal_receipt_id != expected.route_renewal_receipt_id()
        || route.receipt().route_renewal_receipt_digest != expected.route_renewal_receipt_digest()
        || route.effective_expires_at() != expected.route_effective_expires_at()
        || credential.reattestation_receipt_id != expected.credential_reattestation_receipt_id()
        || credential.reattestation_receipt_digest
            != expected.credential_reattestation_receipt_digest()
        || compatibility.verification_receipt_id
            != expected.runtime_compatibility_verification_receipt_id()
        || compatibility.verification_receipt_digest
            != expected.runtime_compatibility_verification_receipt_digest()
        || expected.preflight_checked_at() > carrier.checked_at()
    {
        bail!("active no-work carrier changed from its pre-connect subject");
    }
    Ok(())
}

fn require_exact_task_protocol_identity(
    task_protocol: &crate::store::compute_external_pool_adapter_task_protocol_conformance::CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'_, '_>,
    expected_id: &str,
    expected_digest: &str,
) -> Result<()> {
    if task_protocol.receipt().run_receipt_id != expected_id
        || task_protocol.receipt().run_receipt_digest != expected_digest
    {
        bail!("active no-work V272 head changed during six-reopen preparation");
    }
    Ok(())
}

fn active_observation_expiry(
    checked_at: &str,
    bundle: &crate::store::compute_external_pool_adapter_runtime_bundle::CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority<'_, '_>,
    task_protocol: &crate::store::compute_external_pool_adapter_task_protocol_conformance::CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'_, '_>,
) -> Result<String> {
    let checked = canonical_time(checked_at)?;
    let carrier = task_protocol.carrier();
    let mut expires = checked + ChronoDuration::seconds(15);
    for candidate in [
        bundle
            .vulnerability()
            .receipt()
            .reattestation
            .binding
            .intelligence
            .expires_at
            .as_str(),
        bundle
            .sandbox()
            .receipt()
            .reattestation
            .binding
            .report_expires_at
            .as_str(),
        carrier
            .credential()
            .receipt()
            .reattestation
            .binding
            .report_expires_at
            .as_str(),
        carrier
            .runtime_compatibility()
            .verification()
            .verification
            .expires_at
            .as_str(),
        task_protocol.receipt().run.expires_at.as_str(),
        carrier.renewed_route().effective_expires_at(),
    ] {
        expires = expires.min(canonical_time(candidate)?);
    }
    if checked >= expires {
        bail!("active no-work fresh evidence expired at the final anchor");
    }
    Ok(expires.to_rfc3339_opts(SecondsFormat::Nanos, true))
}

fn canonical_time(value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("active no-work timestamp is not canonical UTC nanoseconds");
    }
    Ok(parsed.with_timezone(&Utc))
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
