//! Active bundle/session reopens #3/#4 and transaction-free Secret delivery.

use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_linux_supervisor::ExternalPoolAdapterSupervisorCgroupParent,
        external_pool_adapter_supervisor_session::external_pool_adapter_session_roots,
    },
    store::{
        compute_external_pool_adapter_provider_active_successor::current_external_pool_adapter_renewed_route_runtime_carrier_for_binding_on,
        compute_external_pool_adapter_runtime_bundle::{
            current_external_pool_adapter_projected_active_runtime_bundle_authority_on,
            ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
        },
        compute_external_pool_adapter_task_protocol_conformance::current_external_pool_adapter_task_protocol_conformance_leaf_for_renewed_route_carrier_on,
        ExternalPoolAdapterTaskProtocolConformanceRuntime, Store,
    },
};
use elon_external_pool_adapter_session_core::prepare_external_pool_adapter_ephemeral_bundle_delivery;

use super::{
    audit_projected_active_delivery_binding_on, deliver_to_authenticated_child,
    projected_active_delivery_binding, CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority,
    PreparedExternalPoolAdapterEphemeralSecretDeliveryLaunch,
};
use crate::store::compute_external_pool_adapter_runtime_bundle::entrypoint_capsule::{
    external_pool_adapter_entrypoint_capsule_policy_root,
    prepare_external_pool_adapter_entrypoint_capsule, ExternalPoolAdapterEntrypointSource,
    PreparedExternalPoolAdapterEntrypointCapsule,
};
use crate::store::compute_external_pool_adapter_runtime_bundle::types::ExternalPoolAdapterRuntimeBundleRoot;

struct ActiveEntrypointSource<'a> {
    prepared: &'a PreparedExternalPoolAdapterInstallation,
}

impl ExternalPoolAdapterEntrypointSource for ActiveEntrypointSource<'_> {
    fn retained_entrypoint(&self) -> Result<(&std::fs::File, &str, u64)> {
        self.prepared.retained_entrypoint()
    }
}

impl Store {
    /// #3 resolves bundle bytes and seals a launch capsule. #4 independently reopens the
    /// installation and re-proves the renewed route plus session roots. Only then is the child
    /// launched, with no SQLite/Prepared/authority crossing the external interaction.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_projected_active_external_pool_adapter_ephemeral_secret_delivery(
        &self,
        provider_binding_id: &str,
        expected_activation_receipt_id: &str,
        expected_activation_receipt_digest: &str,
        expected_task_protocol_run_receipt_id: &str,
        expected_task_protocol_run_receipt_digest: &str,
        bundle_prepared: PreparedExternalPoolAdapterInstallation,
        session_prepared: PreparedExternalPoolAdapterInstallation,
        bundle_root: &ExternalPoolAdapterRuntimeBundleRoot,
        cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
        process_custody: &ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
        task_protocol_runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
    ) -> Result<Option<CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority>> {
        let launch = {
            let mut connection = self.conn()?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let checked_at = now();
            let Some(bundle) =
                current_external_pool_adapter_projected_active_runtime_bundle_authority_on(
                    &transaction,
                    provider_binding_id,
                    expected_activation_receipt_id,
                    expected_activation_receipt_digest,
                    bundle_prepared,
                    bundle_root,
                    &checked_at,
                )?
            else {
                return Ok(None);
            };
            let Some(task_protocol) = current_external_pool_adapter_task_protocol_conformance_leaf_for_renewed_route_carrier_on(
                &transaction,
                bundle.carrier(),
                expected_task_protocol_run_receipt_id,
                expected_task_protocol_run_receipt_digest,
                task_protocol_runtime,
                &checked_at,
            )? else {
                return Ok(None);
            };
            let capsule = prepare_active_capsule(&bundle)?;
            let commitment =
                process_custody.projected_active_runtime_bundle_identity_commitment(&bundle)?;
            let mut delivery = None;
            bundle.with_sensitive_bytes(|config, credential| {
                delivery = Some(prepare_external_pool_adapter_ephemeral_bundle_delivery(
                    bundle.roots().bundle_generation(),
                    config,
                    credential,
                )?);
                Ok(())
            })?;
            bundle.revalidate()?;
            let delivery = delivery
                .ok_or_else(|| anyhow::anyhow!("active Secret delivery material was absent"))?;
            let delivery_root = delivery.bundle_root_hex();
            let carrier = bundle.carrier();
            let companion = carrier.companion();
            let session_roots = external_pool_adapter_session_roots(
                &companion.companion.profile_digest,
                &carrier.target().target_digest,
                &companion.companion_digest,
                capsule.launch_sha256(),
                &delivery_root,
            )?;
            let arguments = session_roots.launch_arguments();
            let binding = projected_active_delivery_binding(
                &bundle,
                &capsule,
                &delivery_root,
                arguments.values(),
                commitment,
                task_protocol.receipt(),
            )?;
            drop(task_protocol);
            let launch = PreparedExternalPoolAdapterEphemeralSecretDeliveryLaunch {
                bundle: bundle.into_prepared_bundle(),
                capsule,
                delivery,
                session_roots,
                binding,
                checked_at,
            };
            transaction.commit()?;
            launch
        };

        {
            let mut connection = self.conn()?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let checked_at = now();
            let Some(carrier) =
                current_external_pool_adapter_renewed_route_runtime_carrier_for_binding_on(
                    &transaction,
                    provider_binding_id,
                    expected_activation_receipt_id,
                    expected_activation_receipt_digest,
                    session_prepared,
                    &checked_at,
                )?
            else {
                return Ok(None);
            };
            let Some(task_protocol) = current_external_pool_adapter_task_protocol_conformance_leaf_for_renewed_route_carrier_on(
                &transaction,
                &carrier,
                expected_task_protocol_run_receipt_id,
                expected_task_protocol_run_receipt_digest,
                task_protocol_runtime,
                &checked_at,
            )? else {
                return Ok(None);
            };
            audit_projected_active_delivery_binding_on(
                &transaction,
                &launch.binding,
                &carrier,
                task_protocol.receipt(),
                None,
                &checked_at,
            )?;
            if launch.binding.session_root_arguments()
                != launch.session_roots.launch_arguments().values()
            {
                bail!("active Secret session arguments changed before child launch");
            }
            drop(task_protocol);
            drop(carrier);
            transaction.commit()?;
        }

        deliver_to_authenticated_child(launch, cgroup_parent).map(Some)
    }
}

fn prepare_active_capsule(
    bundle: &crate::store::compute_external_pool_adapter_runtime_bundle::CurrentExternalPoolAdapterProjectedActiveRuntimeBundleAuthority<'_, '_>,
) -> Result<PreparedExternalPoolAdapterEntrypointCapsule> {
    let carrier = bundle.carrier();
    let source = ActiveEntrypointSource {
        prepared: carrier.prepared(),
    };
    bundle.revalidate()?;
    let capsule = prepare_external_pool_adapter_entrypoint_capsule(&source)?;
    let policy = external_pool_adapter_entrypoint_capsule_policy_root()?;
    let profile = &carrier.profile().profile;
    let retained = carrier.prepared().retained_entrypoint()?;
    let run = &carrier
        .runtime_compatibility()
        .run_observation()
        .observation;
    if policy.policy_digest != capsule.policy_digest()
        || capsule.entrypoint_sha256() != profile.entrypoint_sha256
        || capsule.entrypoint_sha256() != retained.1
        || capsule.entrypoint_size_bytes() != profile.entrypoint_size_bytes
        || capsule.entrypoint_size_bytes() != retained.2
        || run.source_capsule_policy_digest != capsule.policy_digest()
        || run.source_capsule_sha256 != capsule.entrypoint_sha256()
        || run.launch_image_sha256 != capsule.launch_sha256()
    {
        bail!("active Secret launch capsule roots drifted");
    }
    bundle.revalidate()?;
    Ok(capsule)
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
