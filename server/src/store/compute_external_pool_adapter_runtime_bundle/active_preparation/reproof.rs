//! S3 callback-only composite reproof; owned outcomes never authorize delivery.

use std::{marker::PhantomData, path::Path};

use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Transaction, TransactionBehavior};

use crate::{
    compute_federation::{
        external_pool_adapter_installation::audit_external_pool_adapter_installation,
        route_authority::AuthorizedComputeRouteAuthorization,
    },
    store::{
        compute_external_pool_adapter_provider_active_successor::{
            current_external_pool_adapter_renewed_route_runtime_carrier_for_binding_on,
            external_pool_adapter_provider_active_successor_refresh_needed_on,
            require_current_external_pool_adapter_provider_active_successor_on,
            CurrentExternalPoolAdapterProviderActiveSuccessorAuthority,
        },
        compute_external_pool_adapter_task_protocol_conformance::current_external_pool_adapter_task_protocol_conformance_for_renewed_route_carrier_on,
        external_pool_adapter_task_protocol_conformance_runtime, Store,
    },
};

use super::{
    selection::select_external_pool_adapter_active_preparation_candidate_on,
    types::ExternalPoolAdapterActivePreparationCandidate,
};
use crate::store::compute_external_pool_adapter_runtime_bundle::ExternalPoolAdapterProviderRuntimeReadinessRuntime;

/// Final transaction authority over renewed route + current V253/V268/V272/V274.
/// It is neither Clone/Debug/Serde nor constructible outside this module.
pub(in crate::store) struct ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority<
    'authority,
    'tx,
    'conn,
    'runtime,
> {
    active_successor: CurrentExternalPoolAdapterProviderActiveSuccessorAuthority<'tx, 'conn>,
    runtime: &'runtime ExternalPoolAdapterProviderRuntimeReadinessRuntime,
    checked_at: String,
    authority: PhantomData<&'authority ()>,
}

impl<'authority, 'tx, 'conn, 'runtime>
    ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority<'authority, 'tx, 'conn, 'runtime>
{
    pub(in crate::store) fn route_authorization(&self) -> &AuthorizedComputeRouteAuthorization {
        self.active_successor
            .task_protocol()
            .carrier()
            .renewed_route()
            .route_authorization()
    }

    pub(in crate::store) fn active_successor(
        &self,
    ) -> &CurrentExternalPoolAdapterProviderActiveSuccessorAuthority<'tx, 'conn> {
        &self.active_successor
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }

    pub(in crate::store) fn runtime(&self) -> &ExternalPoolAdapterProviderRuntimeReadinessRuntime {
        self.runtime
    }
}

impl Store {
    pub(crate) fn with_reproved_external_pool_adapter_route_and_active_successor<Output>(
        &self,
        provider_id: &str,
        data_dir: &Path,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
        consume: impl FnOnce(
            &Transaction<'_>,
            &ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority<'_, '_, '_, '_>,
        ) -> Result<Output>,
    ) -> Result<Option<Output>> {
        if provider_id.trim().is_empty() {
            bail!("active-successor reproof requires provider_id");
        }
        let candidate = {
            let mut connection = self.conn()?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let checked_at = now();
            let Some(candidate) = select_external_pool_adapter_active_preparation_candidate_on(
                &transaction,
                Some(provider_id),
                0,
            )?
            else {
                return Ok(None);
            };
            if external_pool_adapter_provider_active_successor_refresh_needed_on(
                &transaction,
                &candidate.identity.provider_binding_id,
                &candidate.identity.activation_root_digest,
                runtime.process_custody(),
                &checked_at,
            )? {
                return Ok(None);
            }
            transaction.commit()?;
            candidate
        };
        let prepared = audit_external_pool_adapter_installation(
            data_dir,
            candidate.installation_binding.clone(),
        )
        .map_err(anyhow::Error::new)?;
        let task_runtime = external_pool_adapter_task_protocol_conformance_runtime()
            .map_err(anyhow::Error::new)?;
        self.with_final_composite(candidate, prepared, runtime, &task_runtime, consume)
    }

    fn with_final_composite<Output>(
        &self,
        candidate: ExternalPoolAdapterActivePreparationCandidate,
        prepared: crate::compute_federation::external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
        task_runtime: &crate::store::ExternalPoolAdapterTaskProtocolConformanceRuntime,
        consume: impl FnOnce(
            &Transaction<'_>,
            &ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority<'_, '_, '_, '_>,
        ) -> Result<Output>,
    ) -> Result<Option<Output>> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let checked_at = now();
        let final_candidate = select_external_pool_adapter_active_preparation_candidate_on(
            &transaction,
            Some(&candidate.identity.provider_id),
            0,
        )?
        .ok_or_else(|| anyhow::anyhow!("active preparation candidate disappeared"))?;
        require_same_candidate(&candidate, &final_candidate)?;
        let Some(carrier) =
            current_external_pool_adapter_renewed_route_runtime_carrier_for_binding_on(
                &transaction,
                &candidate.identity.provider_binding_id,
                &candidate.activation_receipt_id,
                &candidate.activation_receipt_digest,
                prepared,
                &checked_at,
            )?
        else {
            return Ok(None);
        };
        let Some(task_protocol) =
            current_external_pool_adapter_task_protocol_conformance_for_renewed_route_carrier_on(
                &transaction,
                carrier,
                task_runtime,
                &checked_at,
            )?
        else {
            return Ok(None);
        };
        let active_successor = require_current_external_pool_adapter_provider_active_successor_on(
            &transaction,
            task_protocol,
            runtime.process_custody(),
            &checked_at,
        )?;
        let authority = ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority {
            active_successor,
            runtime,
            checked_at,
            authority: PhantomData,
        };
        let output = consume(&transaction, &authority)?;
        drop(authority);
        transaction.commit()?;
        Ok(Some(output))
    }
}

fn require_same_candidate(
    expected: &ExternalPoolAdapterActivePreparationCandidate,
    observed: &ExternalPoolAdapterActivePreparationCandidate,
) -> Result<()> {
    if expected.identity != observed.identity
        || expected.activation_receipt_id != observed.activation_receipt_id
        || expected.activation_receipt_digest != observed.activation_receipt_digest
        || expected.activation_genesis_successor_receipt_id
            != observed.activation_genesis_successor_receipt_id
        || expected.activation_genesis_successor_receipt_digest
            != observed.activation_genesis_successor_receipt_digest
        || expected.installation_binding != observed.installation_binding
        || expected.target != observed.target
    {
        bail!("active preparation candidate changed before final callback");
    }
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
