//! V253 purpose split for route recovery and renewed-route runtime consumption.

use std::marker::PhantomData;

use anyhow::{bail, ensure, Result};
use rusqlite::{params, Transaction};

use crate::store::{
    compute_external_pool_adapter_route_renewal::HistoricalExternalPoolAdapterRouteRecoveryAuthority,
    compute_provider_registry::current_registered_provider_on,
};

use super::{
    active_subject::current_external_pool_adapter_projected_active_credential_reattestation_authority_on,
    types::CurrentExternalPoolAdapterCredentialReattestationAuthority,
};

/// Fresh active V253 that may only supply credential input to route renewal.
/// It contains no current route, V268, V272, V274, or dispatch conversion.
pub(in crate::store) struct CurrentExternalPoolAdapterProjectedActiveCredentialRecoveryAuthority<
    'tx,
    'conn,
> {
    credential: CurrentExternalPoolAdapterCredentialReattestationAuthority,
    checked_at: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'tx, 'conn> CurrentExternalPoolAdapterProjectedActiveCredentialRecoveryAuthority<'tx, 'conn> {
    pub(in crate::store) fn credential_for_route_renewal(
        &self,
    ) -> &CurrentExternalPoolAdapterCredentialReattestationAuthority {
        &self.credential
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

pub(in crate::store) fn current_external_pool_adapter_projected_active_credential_recovery_authority_on<
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    historical: &HistoricalExternalPoolAdapterRouteRecoveryAuthority<'tx, 'conn>,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterProjectedActiveCredentialRecoveryAuthority<'tx, 'conn>>>
{
    ensure!(
        historical.checked_at() == checked_at,
        "route recovery and V253 use different checked_at anchors"
    );
    let activation = historical.activation();
    let root = &activation.activation_root().activation_root;
    let active = activation.active_provider();
    let current = current_registered_provider_on(transaction, &active.provider_id)?
        .ok_or_else(|| anyhow::anyhow!("route recovery active Provider disappeared"))?;
    let (receipt_id, receipt_digest) = credential_head_on(transaction, &root.provider_binding_id)?
        .ok_or_else(|| anyhow::anyhow!("route recovery lacks an active V253 head"))?;
    let Some(credential) =
        current_external_pool_adapter_projected_active_credential_reattestation_authority_on(
            transaction,
            &root.provider_binding_id,
            &receipt_id,
            &receipt_digest,
            checked_at,
        )?
    else {
        return Ok(None);
    };
    let binding = &credential.receipt().reattestation.binding;
    if current.provider != *active
        || binding.provider_binding_id != root.provider_binding_id
        || binding.provider_binding_digest != root.provider_binding_digest
        || binding.provider_id != active.provider_id
        || binding.observed_provider_policy_revision != active.policy_revision
        || binding.observed_provider_digest != current.provider_digest
        || binding.observed_provider_status != active.status
        || binding.route_adapter_projection_id != root.route_adapter_projection_id
        || credential.checked_at() != checked_at
    {
        bail!("route-recovery V253 is not exact for the durable active subject");
    }
    Ok(Some(
        CurrentExternalPoolAdapterProjectedActiveCredentialRecoveryAuthority {
            credential,
            checked_at: checked_at.into(),
            transaction: PhantomData,
        },
    ))
}

fn credential_head_on(
    transaction: &Transaction<'_>,
    provider_binding_id: &str,
) -> Result<Option<(String, String)>> {
    let (count, id, digest): (i64, Option<String>, Option<String>) = transaction.query_row(
        "SELECT COUNT(*),MIN(candidate.reattestation_receipt_id),
                MIN(candidate.reattestation_receipt_digest)
           FROM compute_external_pool_adapter_credential_reattestation_receipts candidate
          WHERE candidate.provider_binding_id=?1 AND NOT EXISTS(
                SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts next
                 WHERE next.predecessor_receipt_id=candidate.reattestation_receipt_id)",
        params![provider_binding_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    ensure!(count <= 1, "route recovery found multiple V253 heads");
    Ok(id.zip(digest))
}
