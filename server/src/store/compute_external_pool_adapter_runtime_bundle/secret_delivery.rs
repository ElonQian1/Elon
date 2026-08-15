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
        external_pool_adapter_installation::{
            ExternalPoolAdapterInstallationBinding, PreparedExternalPoolAdapterInstallation,
        },
        external_pool_adapter_linux_supervisor::{
            launch_external_pool_adapter_supervisor_child,
            ExternalPoolAdapterSupervisorCgroupParent, ExternalPoolAdapterSupervisorChild,
        },
        external_pool_adapter_supervisor_session::{
            external_pool_adapter_session_roots, prepare_external_pool_adapter_supervisor_session,
            AuthenticatedExternalPoolAdapterSession,
        },
        external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetReceipt,
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

/// Stable non-secret roots retained across transaction-free network waits.
/// It is intentionally neither Clone, Debug, nor serializable.
pub(super) struct ExternalPoolAdapterEphemeralSecretDeliveryBinding {
    policy_digest: String,
    profile_digest: String,
    target_digest: String,
    companion_digest: String,
    source_capsule_digest: String,
    launch_capsule_digest: String,
    launch_capsule_size_bytes: u64,
    delivery_root: String,
    bundle_material_digest: [u8; 32],
    installation: ExternalPoolAdapterInstallationBinding,
    upstream_target: ExternalPoolAdapterUpstreamTransportTargetReceipt,
    probe_timeout_ms: u64,
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
        let Some(mut authority) = self
            .prepare_current_external_pool_adapter_ephemeral_secret_delivery(
                profile_id,
                companion_id,
                expected_companion_digest,
                bundle_prepared,
                session_prepared,
                bundle_root,
                cgroup_parent,
            )?
        else {
            return Ok(false);
        };
        consume(&authority)?;
        authority.shutdown_and_reap()?;
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
        let mut delivered = None;
        materialize_probe_preparation(&bundle, &selected, |preparation| {
            delivered = Some(deliver_to_authenticated_child(
                preparation,
                &companion,
                cgroup_parent,
            )?);
            Ok(())
        })?;
        drop(selected);
        drop(companion);
        drop(bundle);
        transaction.commit()?;
        Ok(Some(delivered.ok_or_else(|| {
            anyhow::anyhow!("ephemeral secret delivery did not produce an authority")
        })?))
    }
}

fn deliver_to_authenticated_child(
    preparation: &CurrentExternalPoolAdapterProbePreparationAuthority<'_, '_, '_>,
    companion: &CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
) -> Result<CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority> {
    let bundle = preparation.bundle();
    bundle.revalidate()?;
    let binary_roots = bundle.roots();
    let mut delivered = None;
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
        )?;
        let prepared_session = prepare_external_pool_adapter_supervisor_session(session_roots)?;
        let (host, child_bootstrap) = prepared_session.split();
        let child =
            launch_external_pool_adapter_supervisor_child(cgroup_parent, child_bootstrap, capsule)?;
        let mut session = host.authenticate()?;
        let receipt = delivery.deliver(&mut session, &delivery_root, config, credential)?;
        delivered = Some(CurrentExternalPoolAdapterEphemeralSecretDeliveryAuthority {
            session,
            child,
            receipt: Some(receipt),
            binding,
            checked_at: bundle.checked_at().to_string(),
        });
        Ok(())
    })?;
    bundle.revalidate()?;
    delivered.ok_or_else(|| anyhow::anyhow!("authenticated child delivery was not retained"))
}

pub(super) fn delivery_binding(
    preparation: &CurrentExternalPoolAdapterProbePreparationAuthority<'_, '_, '_>,
    companion: &CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    delivery_root: &str,
    session_root_arguments: &[String; 6],
) -> Result<ExternalPoolAdapterEphemeralSecretDeliveryBinding> {
    use sha2::{Digest, Sha256};

    let bundle = preparation.bundle();
    let roots = bundle.roots();
    let companion_receipt = companion.companion();
    let material = &companion_receipt.companion;
    let profile = &bundle.launch_profile().profile().profile;
    let capsule = preparation.capsule();
    let probe_timeout_ms = profile.launch_policy.probe_timeout_ms;
    if profile.launch_policy.probe_contract != "authenticated_no_work_readiness_v1"
        || probe_timeout_ms == 0
        || probe_timeout_ms != material.supervisor_session_policy.state.probe_timeout_ms
        || session_root_arguments[0] != material.supervisor_session_policy_digest
        || session_root_arguments[1] != material.profile_digest
        || session_root_arguments[2] != material.target_digest
        || session_root_arguments[3] != companion_receipt.companion_digest
        || capsule.launch_size_bytes() == 0
        || capsule.launch_sha256() == capsule.entrypoint_sha256()
        || session_root_arguments[4] != capsule.launch_sha256()
        || session_root_arguments[5] != delivery_root
    {
        bail!("ephemeral secret delivery no-work roots rejected");
    }
    let mut digest = Sha256::new();
    digest.update(b"elon.external_pool_adapter.bundle_material.v1\0");
    digest.update(roots.bundle_generation().to_be_bytes());
    digest.update(roots.config_size_bytes().to_be_bytes());
    digest.update(roots.config_sha256());
    digest.update(roots.credential_size_bytes().to_be_bytes());
    digest.update(roots.credential_sha256());
    Ok(ExternalPoolAdapterEphemeralSecretDeliveryBinding {
        policy_digest: session_root_arguments[0].clone(),
        profile_digest: session_root_arguments[1].clone(),
        target_digest: session_root_arguments[2].clone(),
        companion_digest: session_root_arguments[3].clone(),
        source_capsule_digest: capsule.entrypoint_sha256().to_string(),
        launch_capsule_digest: session_root_arguments[4].clone(),
        launch_capsule_size_bytes: capsule.launch_size_bytes(),
        delivery_root: session_root_arguments[5].clone(),
        bundle_material_digest: digest.finalize().into(),
        installation: bundle
            .launch_profile()
            .candidate()
            .registry()
            .prepared()
            .binding()
            .clone(),
        upstream_target: companion.target().target().clone(),
        probe_timeout_ms,
    })
}

pub(super) fn audit_delivery_roots(
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

    pub(super) fn shutdown_and_reap(&mut self) -> Result<()> {
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

impl PartialEq for ExternalPoolAdapterEphemeralSecretDeliveryBinding {
    fn eq(&self, other: &Self) -> bool {
        self.policy_digest == other.policy_digest
            && self.profile_digest == other.profile_digest
            && self.target_digest == other.target_digest
            && self.companion_digest == other.companion_digest
            && self.source_capsule_digest == other.source_capsule_digest
            && self.launch_capsule_digest == other.launch_capsule_digest
            && self.launch_capsule_size_bytes == other.launch_capsule_size_bytes
            && self.delivery_root == other.delivery_root
            && self.bundle_material_digest == other.bundle_material_digest
            && self.installation == other.installation
            && self.upstream_target == other.upstream_target
            && self.probe_timeout_ms == other.probe_timeout_ms
    }
}

impl Eq for ExternalPoolAdapterEphemeralSecretDeliveryBinding {}

impl ExternalPoolAdapterEphemeralSecretDeliveryBinding {
    pub(super) fn upstream_target(&self) -> &ExternalPoolAdapterUpstreamTransportTargetReceipt {
        &self.upstream_target
    }

    pub(super) fn delivery_root(&self) -> &str {
        &self.delivery_root
    }

    pub(super) fn session_root_arguments(&self) -> [String; 6] {
        [
            self.policy_digest.clone(),
            self.profile_digest.clone(),
            self.target_digest.clone(),
            self.companion_digest.clone(),
            self.launch_capsule_digest.clone(),
            self.delivery_root.clone(),
        ]
    }

    pub(super) fn probe_timeout(&self) -> Duration {
        Duration::from_millis(self.probe_timeout_ms)
    }
}
