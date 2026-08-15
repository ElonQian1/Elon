use std::time::Duration;

use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

mod binding;

pub(super) use binding::{
    audit_delivery_roots, delivery_binding, ExternalPoolAdapterEphemeralSecretDeliveryBinding,
};

use super::{
    current::current_external_pool_adapter_runtime_bundle_authority_on,
    entrypoint_capsule::PreparedExternalPoolAdapterEntrypointCapsule,
    probe_preparation::{
        prepare_owned_probe_capsule, select_current_probe_preparation_roots_on,
        with_owned_probe_preparation,
    },
    runtime::ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
    types::{
        CurrentExternalPoolAdapterProbePreparationAuthority, ExternalPoolAdapterRuntimeBundleRoot,
        PreparedExternalPoolAdapterRuntimeBundle,
    },
};
use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_linux_supervisor::{
            launch_external_pool_adapter_supervisor_child,
            ExternalPoolAdapterSupervisorCgroupParent, ExternalPoolAdapterSupervisorChild,
        },
        external_pool_adapter_supervisor_session::{
            external_pool_adapter_session_roots, prepare_external_pool_adapter_supervisor_session,
            AuthenticatedExternalPoolAdapterSession, ExternalPoolAdapterSessionRoots,
        },
    },
    store::{
        compute_external_pool_adapter_supervisor_session_policy_companion::{
            current_external_pool_adapter_supervisor_session_policy_companion_authority_on,
            CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
        },
        Store,
    },
};
use elon_external_pool_adapter_session_core::{
    prepare_external_pool_adapter_ephemeral_bundle_delivery,
    receive_external_pool_adapter_no_work_probe_request,
    ExternalPoolAdapterEphemeralBundleDeliveryHostReceipt,
    ExternalPoolAdapterNoWorkProbeHostReceipt, ExternalPoolAdapterNoWorkProbeHostRequest,
    PreparedExternalPoolAdapterEphemeralBundleDelivery,
};

const CHILD_EXIT_TIMEOUT: Duration = Duration::from_secs(2);

/// Process-private authority proving that one exact child acknowledged one exact V256 bundle.
///
/// It is intentionally neither Clone, Debug, nor serializable. Drop of the child remains the
/// terminal cleanup fallback if the callback or graceful no-work shutdown fails.
pub(in crate::store) struct CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority {
    session: AuthenticatedExternalPoolAdapterSession,
    child: ExternalPoolAdapterSupervisorChild,
    receipt: Option<ExternalPoolAdapterEphemeralBundleDeliveryHostReceipt>,
    binding: ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    checked_at: String,
}

/// Unforgeable terminal authority. Construction requires authenticated shutdown, pidfd reap, and
/// successful cgroup plus scratch cleanup.
pub(super) struct CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority {
    binding: ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    delivery_checked_at: String,
}

struct PreparedExternalPoolAdapterEphemeralSecretDeliveryLaunch {
    bundle: PreparedExternalPoolAdapterRuntimeBundle,
    capsule: PreparedExternalPoolAdapterEntrypointCapsule,
    delivery: PreparedExternalPoolAdapterEphemeralBundleDelivery,
    session_roots: ExternalPoolAdapterSessionRoots,
    binding: ExternalPoolAdapterEphemeralSecretDeliveryBinding,
    checked_at: String,
}

struct PreparedExternalPoolAdapterEphemeralSecretDeliveryRoots {
    delivery: PreparedExternalPoolAdapterEphemeralBundleDelivery,
    session_roots: ExternalPoolAdapterSessionRoots,
    binding: ExternalPoolAdapterEphemeralSecretDeliveryBinding,
}

impl Store {
    #[allow(clippy::too_many_arguments, dead_code)]
    pub(in crate::store) fn with_current_external_pool_adapter_ephemeral_secret_delivery(
        &self,
        profile_id: &str,
        companion_id: &str,
        expected_companion_digest: &str,
        bundle_prepared: PreparedExternalPoolAdapterInstallation,
        session_prepared: PreparedExternalPoolAdapterInstallation,
        bundle_root: &ExternalPoolAdapterRuntimeBundleRoot,
        cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
        process_custody: &ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
        consume: impl FnOnce(&CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority) -> Result<()>,
    ) -> Result<bool> {
        let Some(authority) = self
            .prepare_current_external_pool_adapter_ephemeral_secret_delivery(
                profile_id,
                companion_id,
                expected_companion_digest,
                bundle_prepared,
                session_prepared,
                bundle_root,
                cgroup_parent,
                process_custody,
            )?
        else {
            return Ok(false);
        };
        consume(&authority)?;
        let _cleaned = authority.shutdown_and_reap()?;
        Ok(true)
    }

    #[allow(clippy::too_many_arguments, dead_code)]
    pub(super) fn prepare_current_external_pool_adapter_ephemeral_secret_delivery(
        &self,
        profile_id: &str,
        companion_id: &str,
        expected_companion_digest: &str,
        bundle_prepared: PreparedExternalPoolAdapterInstallation,
        session_prepared: PreparedExternalPoolAdapterInstallation,
        bundle_root: &ExternalPoolAdapterRuntimeBundleRoot,
        cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
        process_custody: &ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
    ) -> Result<Option<CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority>> {
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
        let companion =
            current_external_pool_adapter_supervisor_session_policy_companion_authority_on(
                &transaction,
                companion_id,
                expected_companion_digest,
                session_prepared,
                &checked_at,
            )?
            .ok_or_else(|| anyhow::anyhow!("current exact V259 companion was not found"))?;
        audit_delivery_roots(&bundle, &companion, &checked_at)?;
        let selected =
            select_current_probe_preparation_roots_on(&transaction, &bundle, &checked_at)?;
        let capsule = prepare_owned_probe_capsule(&bundle, &selected)?;
        let mut roots = None;
        with_owned_probe_preparation(&bundle, &selected, &capsule, |preparation| {
            roots = Some(prepare_transaction_free_delivery_roots(
                preparation,
                &companion,
                process_custody,
            )?);
            Ok(())
        })?;
        let roots = roots.ok_or_else(|| {
            anyhow::anyhow!("ephemeral secret delivery did not produce exact launch roots")
        })?;
        let prepared = PreparedExternalPoolAdapterEphemeralSecretDeliveryLaunch {
            bundle: bundle.into_prepared_bundle(),
            capsule,
            delivery: roots.delivery,
            session_roots: roots.session_roots,
            binding: roots.binding,
            checked_at,
        };
        drop(selected);
        drop(companion);
        transaction.commit()?;
        drop(connection);
        Ok(Some(deliver_to_authenticated_child(
            prepared,
            cgroup_parent,
        )?))
    }
}

fn prepare_transaction_free_delivery_roots(
    preparation: &CurrentExternalPoolAdapterProbePreparationAuthority<'_, '_, '_>,
    companion: &CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    process_custody: &ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
) -> Result<PreparedExternalPoolAdapterEphemeralSecretDeliveryRoots> {
    let bundle = preparation.bundle();
    bundle.revalidate()?;
    let binary_roots = bundle.roots();
    let runtime_bundle_identity_commitment =
        process_custody.runtime_bundle_identity_commitment(bundle)?;
    let mut prepared = None;
    bundle.with_sensitive_bytes(|config, credential| {
        let delivery = prepare_external_pool_adapter_ephemeral_bundle_delivery(
            binary_roots.bundle_generation(),
            config,
            credential,
        )?;
        let delivery_root = delivery.bundle_root_hex();
        let companion_receipt = companion.companion();
        let companion_material = &companion_receipt.companion;
        let capsule = preparation.capsule();
        let session_roots = external_pool_adapter_session_roots(
            &companion_material.profile_digest,
            &companion_material.target_digest,
            &companion_receipt.companion_digest,
            capsule.launch_sha256(),
            &delivery_root,
        )?;
        let session_root_arguments = session_roots.launch_arguments();
        let binding = delivery_binding(
            preparation,
            companion,
            &delivery_root,
            session_root_arguments.values(),
            runtime_bundle_identity_commitment,
        )?;
        prepared = Some(PreparedExternalPoolAdapterEphemeralSecretDeliveryRoots {
            delivery,
            session_roots,
            binding,
        });
        Ok(())
    })?;
    bundle.revalidate()?;
    prepared.ok_or_else(|| anyhow::anyhow!("authenticated child delivery roots were not retained"))
}

fn deliver_to_authenticated_child(
    prepared: PreparedExternalPoolAdapterEphemeralSecretDeliveryLaunch,
    cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
) -> Result<CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority> {
    let PreparedExternalPoolAdapterEphemeralSecretDeliveryLaunch {
        bundle,
        capsule,
        delivery,
        session_roots,
        binding,
        checked_at,
    } = prepared;
    bundle.revalidate()?;
    let delivery_root = delivery.bundle_root_hex();
    let prepared_session = prepare_external_pool_adapter_supervisor_session(session_roots)?;
    let (host, child_bootstrap) = prepared_session.split();
    let mut child =
        launch_external_pool_adapter_supervisor_child(cgroup_parent, child_bootstrap, &capsule)?;
    let mut session = host.authenticate()?;
    let mut delivery = Some(delivery);
    let mut receipt = None;
    bundle.with_sensitive_bytes(|config, credential| {
        let selected = delivery
            .take()
            .ok_or_else(|| anyhow::anyhow!("ephemeral delivery was already consumed"))?;
        receipt = Some(selected.deliver(&mut session, &delivery_root, config, credential)?);
        Ok(())
    })?;
    bundle.revalidate()?;
    drop(bundle);
    drop(capsule);
    Ok(CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority {
        session,
        child,
        receipt: Some(
            receipt.ok_or_else(|| anyhow::anyhow!("authenticated delivery receipt was absent"))?,
        ),
        binding,
        checked_at,
    })
}

impl CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority {
    #[allow(dead_code)]
    pub(in crate::store) fn secret_delivery_ready(&self) -> bool {
        let _retained_runtime = (
            &self.session,
            &self.child,
            &self.receipt,
            &self.binding,
            &self.checked_at,
        );
        true
    }

    pub(super) fn binding(&self) -> &ExternalPoolAdapterEphemeralSecretDeliveryBinding {
        &self.binding
    }

    pub(super) fn receive_no_work_request(
        &mut self,
    ) -> Result<ExternalPoolAdapterNoWorkProbeHostRequest> {
        receive_external_pool_adapter_no_work_probe_request(
            &mut self.session,
            self.binding.probe_timeout(),
        )
    }

    pub(super) fn complete_no_work_request(
        &mut self,
        request: ExternalPoolAdapterNoWorkProbeHostRequest,
        response: &[u8],
    ) -> Result<ExternalPoolAdapterNoWorkProbeHostReceipt> {
        request.complete(&mut self.session, response)
    }

    pub(super) fn shutdown_and_reap(
        mut self,
    ) -> Result<CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority> {
        let receipt = self
            .receipt
            .take()
            .ok_or_else(|| anyhow::anyhow!("ephemeral secret delivery receipt was consumed"))?;
        receipt.shutdown(&mut self.session)?;
        let exit = self
            .child
            .wait(CHILD_EXIT_TIMEOUT)?
            .ok_or_else(|| anyhow::anyhow!("ephemeral secret delivery child did not exit"))?;
        if exit.exit_code != Some(0) || exit.signal.is_some() {
            bail!("ephemeral secret delivery child failed no-work shutdown");
        }
        Ok(CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority {
            binding: self.binding,
            delivery_checked_at: self.checked_at,
        })
    }
}

impl CleanedExternalPoolAdapterEphemeralSecretDeliveryAuthority {
    pub(super) fn binding(&self) -> &ExternalPoolAdapterEphemeralSecretDeliveryBinding {
        &self.binding
    }

    pub(super) fn delivery_checked_at(&self) -> &str {
        &self.delivery_checked_at
    }

    pub(super) fn authenticated_shutdown_completed(&self) -> bool {
        true
    }

    pub(super) fn pidfd_reaped(&self) -> bool {
        true
    }

    pub(super) fn cgroup_cleaned(&self) -> bool {
        true
    }

    pub(super) fn scratch_cleaned(&self) -> bool {
        true
    }
}
