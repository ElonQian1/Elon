//! Default-off worker-callable active preparation orchestration.

use std::path::Path;

use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

use crate::{
    compute_federation::external_pool_adapter_installation::audit_external_pool_adapter_installation,
    store::{
        compute_external_pool_adapter_credential_reattestation::current_external_pool_adapter_projected_active_credential_recovery_authority_on,
        compute_external_pool_adapter_provider_active_successor::{
            append_external_pool_adapter_provider_active_successor_refresh_on,
            build_external_pool_adapter_provider_active_successor_refresh_material_on,
            current_external_pool_adapter_renewed_route_runtime_carrier_for_binding_on,
            external_pool_adapter_provider_active_successor_refresh_needed_on,
            postcommit_external_pool_adapter_provider_active_successor_refresh_on,
        },
        compute_external_pool_adapter_route_renewal::{
            build_external_pool_adapter_route_renewal_receipt,
            external_pool_adapter_route_renewal_decision_on,
            finalize_external_pool_adapter_route_renewal_after_commit_on,
            historical_external_pool_adapter_route_recovery_authority_on,
            renew_external_pool_adapter_route_on,
            require_current_external_pool_adapter_renewed_route_on,
            ExternalPoolAdapterRouteRenewalDecision,
        },
        compute_external_pool_adapter_task_protocol_conformance::{
            build_external_pool_adapter_task_protocol_active_refresh_input_on,
            create_external_pool_adapter_task_protocol_conformance_run_for_projected_active,
        },
        external_pool_adapter_task_protocol_conformance_runtime, new_id, Store,
    },
};

use super::{
    selection::select_external_pool_adapter_active_preparation_candidate_on,
    types::{
        ExternalPoolAdapterActivePreparationCandidate,
        ExternalPoolAdapterActivePreparationCycleDisposition,
        ExternalPoolAdapterActivePreparationCycleOutcome,
        ExternalPoolAdapterRegisteringActivationDisposition,
    },
};
use crate::store::compute_external_pool_adapter_runtime_bundle::ExternalPoolAdapterProviderRuntimeReadinessRuntime;

impl Store {
    pub(crate) async fn run_external_pool_adapter_active_preparation_cycle(
        &self,
        data_dir: &Path,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
        worker_id: &str,
    ) -> Result<Option<ExternalPoolAdapterActivePreparationCycleOutcome>> {
        if worker_id.trim().is_empty() || worker_id.len() > 160 {
            bail!("active preparation worker_id is invalid");
        }
        match self
            .activate_external_pool_adapter_registering_candidate(data_dir, runtime)
            .await?
        {
            ExternalPoolAdapterRegisteringActivationDisposition::Deferred
            | ExternalPoolAdapterRegisteringActivationDisposition::NoCandidate
            | ExternalPoolAdapterRegisteringActivationDisposition::Activated => {}
        }
        let Some(candidate) = self.select_active_preparation_candidate()? else {
            return Ok(None);
        };
        let Some(renewed) = self.renew_active_candidate_route(&candidate)? else {
            return Ok(None);
        };
        if !self.active_candidate_refresh_needed(&candidate, runtime)? {
            let disposition = if renewed {
                ExternalPoolAdapterActivePreparationCycleDisposition::Renewed
            } else {
                ExternalPoolAdapterActivePreparationCycleDisposition::AlreadyCurrent
            };
            return self.finalize_owned_outcome(candidate, data_dir, runtime, disposition);
        }

        let task_runtime = external_pool_adapter_task_protocol_conformance_runtime()
            .map_err(anyhow::Error::new)?;
        let installation = candidate.installation_binding.clone();
        let mut reopen_prepared =
            || audit_external_pool_adapter_installation(data_dir, installation.clone());
        let (task_protocol_run_receipt_id, task_protocol_run_receipt_digest) =
            self.refresh_active_task_protocol(&candidate, &mut reopen_prepared, &task_runtime)?;
        let refreshed = self
            .with_projected_active_external_pool_adapter_no_work_observation(
                &candidate.identity.provider_binding_id,
                &candidate.activation_receipt_id,
                &candidate.activation_receipt_digest,
                &task_protocol_run_receipt_id,
                &task_protocol_run_receipt_digest,
                &candidate.target,
                &mut reopen_prepared,
                runtime,
                &task_runtime,
                |transaction, observation| {
                    let material =
                        build_external_pool_adapter_provider_active_successor_refresh_material_on(
                            transaction,
                            observation,
                        )?;
                    append_external_pool_adapter_provider_active_successor_refresh_on(
                        transaction,
                        runtime.process_custody(),
                        new_id("external_pool_adapter_provider_active_successor_refresh"),
                        observation.task_protocol(),
                        material,
                    )
                },
                |connection, pending| {
                    postcommit_external_pool_adapter_provider_active_successor_refresh_on(
                        connection, pending,
                    )
                },
            )
            .await?;
        let Some(_committed_refresh) = refreshed else {
            return Ok(None);
        };
        let disposition = if renewed {
            ExternalPoolAdapterActivePreparationCycleDisposition::RenewedAndRefreshed
        } else {
            ExternalPoolAdapterActivePreparationCycleDisposition::Refreshed
        };
        self.finalize_owned_outcome(candidate, data_dir, runtime, disposition)
    }

    fn select_active_preparation_candidate(
        &self,
    ) -> Result<Option<ExternalPoolAdapterActivePreparationCandidate>> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let selection_slot = u64::try_from(Utc::now().timestamp().div_euclid(60))?;
        let candidate = select_external_pool_adapter_active_preparation_candidate_on(
            &transaction,
            None,
            selection_slot,
        )?;
        transaction.commit()?;
        Ok(candidate)
    }

    fn renew_active_candidate_route(
        &self,
        candidate: &ExternalPoolAdapterActivePreparationCandidate,
    ) -> Result<Option<bool>> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let checked_at = now();
        let decision = external_pool_adapter_route_renewal_decision_on(
            &transaction,
            &candidate.identity.provider_binding_id,
            &candidate.activation_receipt_id,
            &candidate.activation_receipt_digest,
            &checked_at,
        )?;
        if let ExternalPoolAdapterRouteRenewalDecision::Current {
            route_renewal_receipt_id,
            route_renewal_receipt_digest,
        } = &decision
        {
            require_current_external_pool_adapter_renewed_route_on(
                &transaction,
                route_renewal_receipt_id,
                route_renewal_receipt_digest,
                &checked_at,
            )?
            .ok_or_else(|| anyhow::anyhow!("current route disappeared during cycle decision"))?;
            transaction.commit()?;
            return Ok(Some(false));
        }
        let historical = historical_external_pool_adapter_route_recovery_authority_on(
            &transaction,
            &candidate.activation_receipt_id,
            &candidate.activation_receipt_digest,
            &candidate.activation_genesis_successor_receipt_id,
            &candidate.activation_genesis_successor_receipt_digest,
            &checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("route renewal lost historical recovery authority"))?;
        let Some(credential) =
            current_external_pool_adapter_projected_active_credential_recovery_authority_on(
                &transaction,
                &historical,
                &checked_at,
            )?
        else {
            return Ok(None);
        };
        let receipt = build_external_pool_adapter_route_renewal_receipt(
            &transaction,
            &historical,
            credential.credential_for_route_renewal(),
            &decision,
            &checked_at,
        )?;
        let pending = renew_external_pool_adapter_route_on(
            &transaction,
            &historical,
            credential.credential_for_route_renewal(),
            &decision,
            &receipt,
        )?;
        drop(credential);
        drop(historical);
        transaction.commit()?;
        let committed =
            finalize_external_pool_adapter_route_renewal_after_commit_on(&connection, pending)?;
        if committed.receipt().renewal.identity.provider_binding_id
            != candidate.identity.provider_binding_id
            || committed
                .receipt()
                .renewal
                .activation_witness
                .activation_receipt_id
                != candidate.activation_receipt_id
        {
            bail!("committed route renewal changed candidate identity");
        }
        Ok(Some(true))
    }

    fn active_candidate_refresh_needed(
        &self,
        candidate: &ExternalPoolAdapterActivePreparationCandidate,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
    ) -> Result<bool> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let needed = external_pool_adapter_provider_active_successor_refresh_needed_on(
            &transaction,
            &candidate.identity.provider_binding_id,
            &candidate.identity.activation_root_digest,
            runtime.process_custody(),
            &now(),
        )?;
        transaction.commit()?;
        Ok(needed)
    }

    fn refresh_active_task_protocol(
        &self,
        candidate: &ExternalPoolAdapterActivePreparationCandidate,
        reopen_prepared: &mut crate::store::compute_external_pool_adapter_upstream_transport_target::ExternalPoolAdapterInstallationReopener<'_>,
        runtime: &crate::store::ExternalPoolAdapterTaskProtocolConformanceRuntime,
    ) -> Result<(String, String)> {
        let prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let input = {
            let mut connection = self.conn()?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let checked_at = now();
            let carrier =
                current_external_pool_adapter_renewed_route_runtime_carrier_for_binding_on(
                    &transaction,
                    &candidate.identity.provider_binding_id,
                    &candidate.activation_receipt_id,
                    &candidate.activation_receipt_digest,
                    prepared,
                    &checked_at,
                )?
                .ok_or_else(|| anyhow::anyhow!("V272 refresh lacks renewed-route carrier"))?;
            let input = build_external_pool_adapter_task_protocol_active_refresh_input_on(
                &transaction,
                &carrier,
                &checked_at,
            )?;
            drop(carrier);
            transaction.commit()?;
            input
        };
        let written =
            create_external_pool_adapter_task_protocol_conformance_run_for_projected_active(
                self,
                input,
                reopen_prepared,
                runtime,
            )
            .map_err(anyhow::Error::new)?;
        Ok((written.run.run_receipt_id, written.run.run_receipt_digest))
    }

    fn finalize_owned_outcome(
        &self,
        candidate: ExternalPoolAdapterActivePreparationCandidate,
        data_dir: &Path,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
        disposition: ExternalPoolAdapterActivePreparationCycleDisposition,
    ) -> Result<Option<ExternalPoolAdapterActivePreparationCycleOutcome>> {
        let provider_id = candidate.identity.provider_id.clone();
        let identity = candidate.identity;
        let Some(()) = self.with_reproved_external_pool_adapter_route_and_active_successor(
            &provider_id,
            data_dir,
            runtime,
            |_transaction, _authority| Ok(()),
        )?
        else {
            return Ok(None);
        };
        Ok(Some(ExternalPoolAdapterActivePreparationCycleOutcome {
            identity,
            disposition,
        }))
    }
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
