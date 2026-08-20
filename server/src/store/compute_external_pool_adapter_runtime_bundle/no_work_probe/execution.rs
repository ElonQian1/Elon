//! Registering-path preparation plus the transaction-free authenticated no-work exchange.

use anyhow::{bail, Result};
use rusqlite::{Connection, Transaction};

use crate::{
    compute_federation::external_pool_adapter_broker_tls::ExternalPoolAdapterBrokerTlsTarget,
    store::{
        compute_external_pool_adapter_upstream_transport_target::{
            CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
            ExternalPoolAdapterInstallationReopener, PreparedExternalPoolAdapterBrokerTlsChannel,
        },
        Store,
    },
};
use elon_external_pool_adapter_session_core::ExternalPoolAdapterNoWorkProbeHostReceipt;

use super::super::{
    runtime::ExternalPoolAdapterProviderRuntimeReadinessRuntime,
    secret_delivery::{
        CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority,
        CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority,
    },
};
use super::{
    require_preflight_dynamic_and_compatibility_roots,
    CurrentExternalPoolAdapterNoWorkProbeObservationAuthority,
};

/// Completed external evidence. It owns no SQLite connection, transaction, or Prepared authority.
pub(super) struct ExecutedExternalPoolAdapterNoWorkProbe {
    pub(super) receipt: ExternalPoolAdapterNoWorkProbeHostReceipt,
    pub(super) selected_address: std::net::SocketAddr,
    pub(super) cleaned: CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority,
}

/// Performs only the real authenticated application exchange and terminal child cleanup.
///
/// Both inputs are transaction-free, purpose-specific preparations. Callers must establish their
/// own typed database subject before this function and freshly re-prove it afterwards.
pub(super) async fn execute_external_pool_adapter_no_work_probe(
    mut broker: PreparedExternalPoolAdapterBrokerTlsChannel,
    mut delivery: CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority,
    target_id: &str,
    expected_target_digest: &str,
) -> Result<ExecutedExternalPoolAdapterNoWorkProbe> {
    let delivery_target =
        ExternalPoolAdapterBrokerTlsTarget::from_receipt(delivery.binding().upstream_target())?;
    if broker.target() != &delivery_target
        || broker.target().target_id() != target_id
        || broker.target().target_digest() != expected_target_digest
    {
        bail!("no-work probe broker and child roots diverged");
    }

    let request = delivery.receive_no_work_request()?;
    let response = broker
        .exchange_no_work(
            request.request(),
            request.expected_response_bytes(),
            delivery.binding().probe_timeout(),
        )
        .await?;
    let selected_address = broker.selected_address();
    let receipt = delivery.complete_no_work_request(request, &response)?;
    drop(response);
    drop(broker);

    // Final callbacks and durable writes remain unreachable until cleanup consumes the child.
    let cleaned = delivery.shutdown_and_reap()?;
    Ok(ExecutedExternalPoolAdapterNoWorkProbe {
        receipt,
        selected_address,
        cleaned,
    })
}

impl Store {
    #[allow(clippy::too_many_arguments, dead_code)]
    pub(in crate::store) async fn with_current_external_pool_adapter_no_work_probe_observation<
        Pending,
        Output,
    >(
        &self,
        profile_id: &str,
        companion_id: &str,
        expected_companion_digest: &str,
        target_id: &str,
        expected_target_digest: &str,
        runtime_compatibility_verification_receipt_id: &str,
        expected_runtime_compatibility_verification_receipt_digest: &str,
        reopen_prepared: &mut ExternalPoolAdapterInstallationReopener<'_>,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
        preflight_consume: impl FnOnce(
                &Transaction<'_>,
                &CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
                &str,
            ) -> Result<()>
            + Send,
        consume: impl FnOnce(
                &Transaction<'_>,
                &CurrentExternalPoolAdapterNoWorkProbeObservationAuthority<'_, '_, '_>,
            ) -> Result<Pending>
            + Send,
        postcommit: impl FnOnce(&Connection, Pending) -> Result<Output> + Send,
    ) -> Result<Option<Output>> {
        let Some(broker) = self
            .prepare_current_external_pool_adapter_broker_tls_channel(
                target_id,
                expected_target_digest,
                reopen_prepared,
                |transaction, target, checked_at| {
                    require_preflight_dynamic_and_compatibility_roots(
                        transaction,
                        target,
                        runtime_compatibility_verification_receipt_id,
                        expected_runtime_compatibility_verification_receipt_digest,
                        checked_at,
                    )?;
                    preflight_consume(transaction, target, checked_at)
                },
            )
            .await?
        else {
            return Ok(None);
        };

        // Successful registering execution retains the historical six-reopen contract. These are
        // #3 and #4; neither is opened before the broker network await completes.
        let delivery_bundle_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let delivery_session_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let Some(delivery) = self.prepare_current_external_pool_adapter_ephemeral_secret_delivery(
            profile_id,
            companion_id,
            expected_companion_digest,
            delivery_bundle_prepared,
            delivery_session_prepared,
            runtime.bundle_root(),
            runtime.cgroup_parent(),
            runtime.process_custody(),
        )?
        else {
            return Ok(None);
        };
        let executed = execute_external_pool_adapter_no_work_probe(
            broker,
            delivery,
            target_id,
            expected_target_digest,
        )
        .await?;

        // These are successful-path reopens #5 and #6, obtained only after terminal cleanup.
        let reproof_bundle_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let reproof_session_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        self.with_reproved_external_pool_adapter_no_work_roots(
            profile_id,
            companion_id,
            expected_companion_digest,
            runtime_compatibility_verification_receipt_id,
            expected_runtime_compatibility_verification_receipt_digest,
            reproof_bundle_prepared,
            reproof_session_prepared,
            runtime,
            executed.receipt,
            executed.selected_address,
            executed.cleaned,
            consume,
            postcommit,
        )
    }
}
