use anyhow::{bail, Result};
use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

use super::{
    current::current_external_pool_adapter_runtime_bundle_authority_on,
    probe_preparation::{materialize_probe_preparation, select_current_probe_preparation_roots_on},
    secret_delivery::{
        audit_delivery_roots, delivery_binding, ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    },
    types::ExternalPoolAdapterRuntimeBundleRoot,
};
use crate::{
    compute_federation::{
        external_pool_adapter_broker_tls::ExternalPoolAdapterBrokerTlsTarget,
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_linux_supervisor::ExternalPoolAdapterSupervisorCgroupParent,
    },
    store::{
        compute_external_pool_adapter_supervisor_session_policy_companion::current_external_pool_adapter_supervisor_session_policy_companion_authority_on,
        Store,
    },
};
use elon_external_pool_adapter_session_core::ExternalPoolAdapterNoWorkProbeHostReceipt;

/// Process-private proof of one exact no-task response. No raw roots or application bytes escape.
pub(in crate::store) struct CurrentExternalPoolAdapterNoWorkProbeObservationAuthority {
    receipt: ExternalPoolAdapterNoWorkProbeHostReceipt,
    binding: ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    selected_address: std::net::SocketAddr,
    checked_at: String,
    expires_at: String,
}

impl CurrentExternalPoolAdapterNoWorkProbeObservationAuthority {
    pub(in crate::store) fn no_work_observed(&self) -> bool {
        let _retained_exact_authority = (
            &self.receipt,
            &self.binding,
            self.selected_address,
            &self.checked_at,
            &self.expires_at,
        );
        true
    }

    pub(in crate::store) fn request_bytes(&self) -> u32 {
        self.receipt.request_bytes()
    }

    pub(in crate::store) fn response_bytes(&self) -> u32 {
        self.receipt.response_bytes()
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }

    pub(in crate::store) fn expires_at(&self) -> &str {
        &self.expires_at
    }
}

impl Store {
    #[allow(clippy::too_many_arguments, dead_code)]
    pub(in crate::store) async fn with_current_external_pool_adapter_no_work_probe_observation(
        &self,
        profile_id: &str,
        companion_id: &str,
        expected_companion_digest: &str,
        target_id: &str,
        expected_target_digest: &str,
        broker_preflight_prepared: PreparedExternalPoolAdapterInstallation,
        broker_postflight_prepared: PreparedExternalPoolAdapterInstallation,
        delivery_bundle_prepared: PreparedExternalPoolAdapterInstallation,
        delivery_session_prepared: PreparedExternalPoolAdapterInstallation,
        reproof_bundle_prepared: PreparedExternalPoolAdapterInstallation,
        reproof_session_prepared: PreparedExternalPoolAdapterInstallation,
        bundle_root: &ExternalPoolAdapterRuntimeBundleRoot,
        cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
        consume: impl FnOnce(&CurrentExternalPoolAdapterNoWorkProbeObservationAuthority) -> Result<()>,
    ) -> Result<bool> {
        let Some(mut broker) = self
            .prepare_current_external_pool_adapter_broker_tls_channel(
                target_id,
                expected_target_digest,
                broker_preflight_prepared,
                broker_postflight_prepared,
            )
            .await?
        else {
            return Ok(false);
        };

        let Some(mut delivery) = self
            .prepare_current_external_pool_adapter_ephemeral_secret_delivery(
                profile_id,
                companion_id,
                expected_companion_digest,
                delivery_bundle_prepared,
                delivery_session_prepared,
                bundle_root,
                cgroup_parent,
            )?
        else {
            return Ok(false);
        };
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

        let Some((binding, checked_at)) = self.reprove_external_pool_adapter_no_work_roots(
            profile_id,
            companion_id,
            expected_companion_digest,
            reproof_bundle_prepared,
            reproof_session_prepared,
            bundle_root,
            delivery.binding(),
        )?
        else {
            return Ok(false);
        };
        let checked = chrono::DateTime::parse_from_rfc3339(&checked_at)?.with_timezone(&Utc);
        let timeout = ChronoDuration::from_std(binding.probe_timeout())?;
        let expires_at = (checked + timeout).to_rfc3339_opts(SecondsFormat::Nanos, true);
        let observation = CurrentExternalPoolAdapterNoWorkProbeObservationAuthority {
            receipt,
            binding,
            selected_address,
            checked_at,
            expires_at,
        };
        if !observation.no_work_observed() || Utc::now() >= checked + timeout {
            bail!("no-work probe observation expired before consumption");
        }
        consume(&observation)?;
        delivery.shutdown_and_reap()?;
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn reprove_external_pool_adapter_no_work_roots(
        &self,
        profile_id: &str,
        companion_id: &str,
        expected_companion_digest: &str,
        bundle_prepared: PreparedExternalPoolAdapterInstallation,
        session_prepared: PreparedExternalPoolAdapterInstallation,
        bundle_root: &ExternalPoolAdapterRuntimeBundleRoot,
        expected: &ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    ) -> Result<Option<(ExternalPoolAdapterEphemeralSecretDeliveryBinding, String)>> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let Some(bundle) = current_external_pool_adapter_runtime_bundle_authority_on(
            &transaction,
            profile_id,
            bundle_prepared,
            bundle_root,
            &checked_at,
        )?
        else {
            return Ok(None);
        };
        let Some(companion) =
            current_external_pool_adapter_supervisor_session_policy_companion_authority_on(
                &transaction,
                companion_id,
                expected_companion_digest,
                session_prepared,
                &checked_at,
            )?
        else {
            return Ok(None);
        };
        audit_delivery_roots(&bundle, &companion, &checked_at)?;
        let selected =
            select_current_probe_preparation_roots_on(&transaction, &bundle, &checked_at)?;
        let roots = expected.session_root_arguments();
        let mut observed = None;
        materialize_probe_preparation(&bundle, &selected, |preparation| {
            observed = Some(delivery_binding(
                preparation,
                &companion,
                expected.delivery_root(),
                &roots,
            )?);
            Ok(())
        })?;
        let observed = observed
            .ok_or_else(|| anyhow::anyhow!("no-work reproof did not produce exact roots"))?;
        if &observed != expected {
            bail!("no-work probe roots changed after application exchange");
        }
        drop(selected);
        drop(companion);
        drop(bundle);
        transaction.commit()?;
        Ok(Some((observed, checked_at)))
    }
}
