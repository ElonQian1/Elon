use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_provider_active_successor::ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
        external_pool_adapter_task_protocol_conformance::server_task_protocol_conformance_profile_catalog,
        provider::ComputeProvider,
    },
    store::{
        compute_external_pool_adapter_runtime_compatibility_verification::current_external_pool_adapter_runtime_compatibility_verification_authority_on,
        compute_external_pool_adapter_supervisor_session_policy_companion::current_external_pool_adapter_supervisor_session_policy_companion_authority_on,
        compute_provider_registry::{
            current_registered_provider_on, ComputeProviderRegistrationReceipt,
        },
    },
};

use super::{
    audit::audited_structural_input, provider_target::derive_target,
    types::PreparedExternalPoolAdapterProviderActiveSuccessorTarget,
};

/// Exact CAS inputs for a non-authorizing structural target. It contains no V277 witness.
pub(in crate::store) struct PrepareExternalPoolAdapterProviderActiveSuccessorTarget {
    pub(in crate::store) prepared_installation: PreparedExternalPoolAdapterInstallation,
    pub(in crate::store) companion_id: String,
    pub(in crate::store) expected_companion_digest: String,
    pub(in crate::store) runtime_compatibility_verification_receipt_id: String,
    pub(in crate::store) expected_runtime_compatibility_verification_receipt_digest: String,
}

/// Builds only the registering source plus planned adjacent projected Provider target/root.
/// It neither performs external I/O nor mints/remembers a process seal.
pub(in crate::store) fn prepare_external_pool_adapter_provider_active_successor_target_on<
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    input: PrepareExternalPoolAdapterProviderActiveSuccessorTarget,
    activation_target_updated_at: &str,
    authority_checked_at: &str,
) -> Result<PreparedExternalPoolAdapterProviderActiveSuccessorTarget<'tx, 'conn>> {
    if activation_target_updated_at > authority_checked_at {
        bail!("provider active-successor target time is later than its authority reproof");
    }
    let companion = current_external_pool_adapter_supervisor_session_policy_companion_authority_on(
        transaction,
        &input.companion_id,
        &input.expected_companion_digest,
        input.prepared_installation,
        authority_checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("provider active-successor lost exact current V259"))?;
    let compatibility =
        current_external_pool_adapter_runtime_compatibility_verification_authority_on(
            transaction,
            &input.runtime_compatibility_verification_receipt_id,
            &input.expected_runtime_compatibility_verification_receipt_digest,
            authority_checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("provider active-successor lost exact current V268"))?;
    let (source, target, activation_root) = derive_current_target_on(
        transaction,
        &companion,
        &compatibility,
        activation_target_updated_at,
        authority_checked_at,
    )?;
    Ok(
        PreparedExternalPoolAdapterProviderActiveSuccessorTarget::new(
            transaction,
            source,
            target,
            activation_root,
            companion,
            compatibility,
            authority_checked_at.into(),
            activation_target_updated_at.into(),
        ),
    )
}

/// Rebuilds the exact planned target/root from final-transaction authorities after external I/O.
/// This returns no independently usable authority; a typed caller must retain its transaction-
/// bound observation while consuming the successful comparison.
#[allow(clippy::too_many_arguments)]
pub(in crate::store) fn reprove_external_pool_adapter_provider_active_successor_target_on(
    transaction: &Transaction<'_>,
    expected_source: &ComputeProviderRegistrationReceipt,
    expected_target: &ComputeProvider,
    expected_activation_root: &ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
    companion: &crate::store::compute_external_pool_adapter_supervisor_session_policy_companion::CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    compatibility: &crate::store::compute_external_pool_adapter_runtime_compatibility_verification::CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'_, '_>,
    activation_target_updated_at: &str,
    authority_checked_at: &str,
) -> Result<()> {
    let (source, target, activation_root) = derive_current_target_on(
        transaction,
        companion,
        compatibility,
        activation_target_updated_at,
        authority_checked_at,
    )?;
    if &source != expected_source
        || &target != expected_target
        || &activation_root != expected_activation_root
    {
        bail!("final provider active-successor target/root differs from its pre-I/O plan");
    }
    Ok(())
}

fn derive_current_target_on(
    transaction: &Transaction<'_>,
    companion: &crate::store::compute_external_pool_adapter_supervisor_session_policy_companion::CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority,
    compatibility: &crate::store::compute_external_pool_adapter_runtime_compatibility_verification::CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority<'_, '_>,
    activation_target_updated_at: &str,
    authority_checked_at: &str,
) -> Result<(
    ComputeProviderRegistrationReceipt,
    ComputeProvider,
    ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
)> {
    let transport_target = companion.target();
    let profile = transport_target.profile();
    let candidate = profile.candidate();
    let binding = candidate.registry();
    if companion.checked_at() != authority_checked_at
        || transport_target.checked_at() != authority_checked_at
        || profile.checked_at() != authority_checked_at
        || candidate.checked_at() != authority_checked_at
        || binding.checked_at() != authority_checked_at
        || compatibility.checked_at() != authority_checked_at
    {
        bail!("provider active-successor structural roots were not checked at one instant");
    }
    let binding_material = &binding.binding().binding;
    let task_protocol = server_task_protocol_conformance_profile_catalog()?;
    let structural = audited_structural_input(
        binding,
        candidate.delegation(),
        candidate.candidate(),
        profile.profile(),
        transport_target.target(),
        companion.companion(),
        compatibility,
        &task_protocol.profile_digest,
        authority_checked_at,
    )?;
    let source = current_registered_provider_on(transaction, &binding_material.provider_id)?
        .ok_or_else(|| anyhow::anyhow!("provider active-successor source Provider disappeared"))?;
    if source.provider_digest != binding_material.provider_digest
        || source.provider.policy_revision != binding_material.provider_policy_revision
    {
        bail!("provider active-successor source Provider is not exact V249 registering history");
    }
    let (target, activation_root) =
        derive_target(&source, structural, activation_target_updated_at)?;
    Ok((source, target, activation_root))
}
