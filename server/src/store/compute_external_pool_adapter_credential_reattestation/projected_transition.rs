use std::marker::PhantomData;

use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::provider::{PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_REGISTERING},
    store::compute_external_pool_adapter_provider_active_successor::PreparedExternalPoolAdapterProviderActiveSuccessorTarget,
};

use super::{
    current::current_external_pool_adapter_credential_reattestation_authority_on,
    types::CurrentExternalPoolAdapterCredentialReattestationAuthority,
};

/// Non-authorizing V253 proof for V275's future atomic registering-to-projection transition.
///
/// It borrows the opaque planned target and retains the exact current registering V253 authority.
/// It cannot be cloned, formatted, serialized, or converted into ordinary active currentness.
pub(in crate::store) struct PreparedExternalPoolAdapterCredentialProjectedActiveTransition<
    'target,
    'tx,
    'conn,
> {
    credential: CurrentExternalPoolAdapterCredentialReattestationAuthority,
    target: &'target PreparedExternalPoolAdapterProviderActiveSuccessorTarget<'tx, 'conn>,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'target, 'tx, 'conn>
    PreparedExternalPoolAdapterCredentialProjectedActiveTransition<'target, 'tx, 'conn>
{
    pub(in crate::store) fn credential(
        &self,
    ) -> &CurrentExternalPoolAdapterCredentialReattestationAuthority {
        &self.credential
    }

    pub(in crate::store) fn target(
        &self,
    ) -> &PreparedExternalPoolAdapterProviderActiveSuccessorTarget<'tx, 'conn> {
        self.target
    }
}

pub(in crate::store) fn prepare_external_pool_adapter_credential_projected_active_transition_on<
    'target,
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    target: &'target PreparedExternalPoolAdapterProviderActiveSuccessorTarget<'tx, 'conn>,
    expected_reattestation_receipt_id: &str,
    expected_reattestation_receipt_digest: &str,
    checked_at: &str,
) -> Result<PreparedExternalPoolAdapterCredentialProjectedActiveTransition<'target, 'tx, 'conn>> {
    if target.checked_at() != checked_at {
        bail!("projected-active transition target uses a different checked_at anchor");
    }
    let activation = &target.activation_root().activation_root;
    let binding_id = &activation.provider_binding_id;
    let credential = current_external_pool_adapter_credential_reattestation_authority_on(
        transaction,
        binding_id,
        expected_reattestation_receipt_id,
        expected_reattestation_receipt_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("projected-active transition lacks exact registering V253"))?;
    let receipt = credential.receipt();
    let observed = &receipt.reattestation.binding;
    let source = &target.source().provider;
    let planned = target.target();
    if credential.checked_at() != checked_at
        || observed.provider_binding_id != activation.provider_binding_id
        || observed.provider_binding_digest != activation.provider_binding_digest
        || observed.provider_id != activation.source_registering_provider_id
        || observed.observed_provider_policy_revision
            != activation.source_registering_provider_policy_revision
        || observed.observed_provider_digest != activation.source_registering_provider_digest
        || observed.observed_provider_status != PROVIDER_STATUS_REGISTERING
        || observed.adapter_id != activation.logical_adapter_id
        || observed.route_adapter_projection_id != activation.route_adapter_projection_id
        || source.status != PROVIDER_STATUS_REGISTERING
        || source.policy_revision != observed.observed_provider_policy_revision
        || planned.status != PROVIDER_STATUS_ACTIVE
        || planned.policy_revision != source.policy_revision.checked_add(1).unwrap_or(0)
        || planned
            .adapter
            .as_ref()
            .map(|adapter| adapter.adapter_id.as_str())
            != Some(activation.route_adapter_projection_id.as_str())
    {
        bail!(
            "projected-active transition does not bind exact logical source to planned projection"
        );
    }
    Ok(
        PreparedExternalPoolAdapterCredentialProjectedActiveTransition {
            credential,
            target,
            transaction: PhantomData,
        },
    )
}
