use std::time::Duration;

use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

use super::{
    current::current_external_pool_adapter_runtime_bundle_authority_on,
    probe_preparation::{materialize_probe_preparation, select_current_probe_preparation_roots_on},
    types::{
        CurrentExternalPoolAdapterProbePreparationAuthority,
        CurrentExternalPoolAdapterRuntimeBundleAuthority, ExternalPoolAdapterRuntimeBundleRoot,
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
            AuthenticatedExternalPoolAdapterSession,
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
    ExternalPoolAdapterEphemeralBundleDeliveryHostReceipt,
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
    checked_at: String,
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
        consume: impl FnOnce(&CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority) -> Result<()>,
    ) -> Result<bool> {
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
            return Ok(false);
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
        materialize_probe_preparation(&bundle, &selected, |preparation| {
            deliver_to_authenticated_child(preparation, &companion, cgroup_parent, consume)
        })?;
        drop(selected);
        drop(companion);
        drop(bundle);
        transaction.commit()?;
        Ok(true)
    }
}

fn deliver_to_authenticated_child(
    preparation: &CurrentExternalPoolAdapterProbePreparationAuthority<'_, '_, '_>,
    companion: &CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
    consume: impl FnOnce(&CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority) -> Result<()>,
) -> Result<()> {
    let bundle = preparation.bundle();
    bundle.revalidate()?;
    let binary_roots = bundle.roots();
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
            capsule.entrypoint_sha256(),
            &delivery_root,
        )?;
        let prepared_session = prepare_external_pool_adapter_supervisor_session(session_roots)?;
        let (host, child_bootstrap) = prepared_session.split();
        let child =
            launch_external_pool_adapter_supervisor_child(cgroup_parent, child_bootstrap, capsule)?;
        let mut session = host.authenticate()?;
        let receipt = delivery.deliver(&mut session, &delivery_root, config, credential)?;
        let mut authority = CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority {
            session,
            child,
            receipt: Some(receipt),
            checked_at: bundle.checked_at().to_string(),
        };
        consume(&authority)?;
        authority.shutdown_and_reap()?;
        Ok(())
    })?;
    bundle.revalidate()?;
    Ok(())
}

fn audit_delivery_roots(
    bundle: &CurrentExternalPoolAdapterRuntimeBundleAuthority<'_, '_>,
    companion: &CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    checked_at: &str,
) -> Result<()> {
    let bundle_profile = bundle.launch_profile().profile();
    let companion_receipt = companion.companion();
    let companion_material = &companion_receipt.companion;
    let target = companion.target();
    let target_profile = target.profile().profile();
    let bundle_installation = bundle
        .launch_profile()
        .candidate()
        .registry()
        .prepared()
        .binding();
    let session_installation = target.profile().candidate().registry().prepared().binding();
    if bundle.checked_at() != checked_at
        || companion.checked_at() != checked_at
        || target.checked_at() != checked_at
        || companion_material.profile_id != bundle_profile.profile_id
        || companion_material.profile_digest != bundle_profile.profile_digest
        || target_profile.profile_id != bundle_profile.profile_id
        || target_profile.profile_digest != bundle_profile.profile_digest
        || companion_material.target_id != target.target().target_id
        || companion_material.target_digest != target.target().target_digest
        || companion_material.provider_binding_id != bundle_profile.profile.provider_binding_id
        || companion_material.provider_binding_digest
            != bundle_profile.profile.provider_binding_digest
        || bundle_installation != session_installation
    {
        bail!("ephemeral secret delivery roots drifted");
    }
    Ok(())
}

impl CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority {
    #[allow(dead_code)]
    pub(in crate::store) fn secret_delivery_ready(&self) -> bool {
        let _retained_runtime = (&self.session, &self.child, &self.receipt, &self.checked_at);
        true
    }

    fn shutdown_and_reap(&mut self) -> Result<()> {
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
        Ok(())
    }
}
