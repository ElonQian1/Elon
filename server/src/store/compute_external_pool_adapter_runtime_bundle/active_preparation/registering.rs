//! Deterministic registering Provider activation before any durable-active preparation.

use std::path::Path;

use anyhow::{ensure, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

use crate::{
    compute_federation::external_pool_adapter_installation::audit_external_pool_adapter_installation,
    store::{
        compute_external_pool_adapter_credential_reattestation::prepare_external_pool_adapter_credential_projected_active_transition_on,
        compute_external_pool_adapter_provider_active_successor::{
            build_external_pool_adapter_atomic_activation_genesis_on,
            finalize_external_pool_adapter_atomic_activation_after_commit_on,
            persist_external_pool_adapter_atomic_activation_closure_on,
            prepare_external_pool_adapter_provider_active_successor_target_on,
            PrepareExternalPoolAdapterProviderActiveSuccessorTarget,
        },
        compute_external_pool_adapter_task_protocol_conformance::{
            build_external_pool_adapter_task_protocol_registering_activation_input_on,
            prepare_external_pool_adapter_task_protocol_planned_active_carrier_on,
        },
        external_pool_adapter_task_protocol_conformance_runtime, new_id, Store,
    },
};

use self::selection::select_external_pool_adapter_registering_activation_candidate_on;
use super::types::{
    ExternalPoolAdapterRegisteringActivationCandidate,
    ExternalPoolAdapterRegisteringActivationDisposition,
};
use crate::store::compute_external_pool_adapter_runtime_bundle::{
    planned_external_pool_adapter_active_no_work_probe_subject_on,
    ExternalPoolAdapterProviderRuntimeReadinessRuntime,
};

mod selection;

impl Store {
    pub(super) async fn activate_external_pool_adapter_registering_candidate(
        &self,
        data_dir: &Path,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
    ) -> Result<ExternalPoolAdapterRegisteringActivationDisposition> {
        let Some(candidate) = self.select_registering_activation_candidate()? else {
            return Ok(ExternalPoolAdapterRegisteringActivationDisposition::NoCandidate);
        };
        let activation_target_updated_at = now();
        let prepared = audit_external_pool_adapter_installation(
            data_dir,
            candidate.installation_binding.clone(),
        )
        .map_err(anyhow::Error::new)?;
        let (task_input, planned) = {
            let mut connection = self.conn()?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let checked_at = now();
            let target = prepare_external_pool_adapter_provider_active_successor_target_on(
                &transaction,
                PrepareExternalPoolAdapterProviderActiveSuccessorTarget {
                    prepared_installation: prepared,
                    companion_id: candidate.companion_id.clone(),
                    expected_companion_digest: candidate.companion_digest.clone(),
                    runtime_compatibility_verification_receipt_id: candidate
                        .runtime_compatibility_verification_receipt_id
                        .clone(),
                    expected_runtime_compatibility_verification_receipt_digest: candidate
                        .runtime_compatibility_verification_receipt_digest
                        .clone(),
                },
                &activation_target_updated_at,
                &checked_at,
            )?;
            ensure!(
                target.source().provider.provider_id == candidate.provider_id
                    && target.activation_root().activation_root.provider_binding_id
                        == candidate.provider_binding_id
                    && target
                        .activation_root()
                        .activation_root
                        .provider_binding_digest
                        == candidate.provider_binding_digest,
                "registering selector changed during typed V274 target preparation"
            );
            let task_input =
                build_external_pool_adapter_task_protocol_registering_activation_input_on(
                    &transaction,
                    &target,
                    &checked_at,
                )?;
            let planned = planned_external_pool_adapter_active_no_work_probe_subject_on(&target)?;
            transaction.commit()?;
            (task_input, planned)
        };
        let task_runtime = external_pool_adapter_task_protocol_conformance_runtime()
            .map_err(anyhow::Error::new)?;
        let installation = candidate.installation_binding.clone();
        let mut reopen_prepared =
            || audit_external_pool_adapter_installation(data_dir, installation.clone());
        let written = self
            .create_external_pool_adapter_task_protocol_conformance_run(
                task_input,
                &mut reopen_prepared,
                &task_runtime,
            )
            .map_err(anyhow::Error::new)?;
        let run_receipt_id = written.run.run_receipt_id.clone();
        let run_receipt_digest = written.run.run_receipt_digest.clone();
        let activated = self
            .with_planned_external_pool_adapter_active_no_work_probe_observation(
                &planned,
                &mut reopen_prepared,
                runtime,
                |transaction, no_work| {
                    let transition =
                        prepare_external_pool_adapter_credential_projected_active_transition_on(
                            transaction,
                            no_work,
                        )?;
                    let task_protocol =
                        prepare_external_pool_adapter_task_protocol_planned_active_carrier_on(
                            transaction,
                            no_work,
                            &run_receipt_id,
                            &run_receipt_digest,
                            &task_runtime,
                        )?;
                    let built = build_external_pool_adapter_atomic_activation_genesis_on(
                        transaction,
                        no_work,
                        &transition,
                        &task_protocol,
                    )?;
                    persist_external_pool_adapter_atomic_activation_closure_on(
                        transaction,
                        runtime.process_custody(),
                        new_id("external_pool_adapter_provider_active_successor"),
                        no_work,
                        &transition,
                        &task_protocol,
                        no_work.preflight().source(),
                        no_work.preflight().target(),
                        built.target_digest(),
                        built.route(),
                        built.receipt(),
                    )
                },
                finalize_external_pool_adapter_atomic_activation_after_commit_on,
            )
            .await?;
        let Some((historical, _committed_successor)) = activated else {
            return Ok(ExternalPoolAdapterRegisteringActivationDisposition::Deferred);
        };
        ensure!(
            historical.receipt().activation.identity.provider_binding_id
                == candidate.provider_binding_id
                && historical
                    .receipt()
                    .activation
                    .identity
                    .provider_binding_digest
                    == candidate.provider_binding_digest
                && historical.active_provider().provider_id == candidate.provider_id,
            "committed V277 activation changed the selected registering identity"
        );
        Ok(ExternalPoolAdapterRegisteringActivationDisposition::Activated)
    }

    fn select_registering_activation_candidate(
        &self,
    ) -> Result<Option<ExternalPoolAdapterRegisteringActivationCandidate>> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let selection_slot = u64::try_from(Utc::now().timestamp().div_euclid(60))?;
        let candidate = select_external_pool_adapter_registering_activation_candidate_on(
            &transaction,
            selection_slot,
        )?;
        transaction.commit()?;
        Ok(candidate)
    }
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
