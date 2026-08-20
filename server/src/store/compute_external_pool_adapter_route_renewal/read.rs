use anyhow::{bail, ensure, Result};
use chrono::{DateTime, Duration, SecondsFormat};
use rusqlite::{named_params, params, types::Type, OptionalExtension, Transaction};

use crate::{
    compute_federation::{
        external_pool_adapter_atomic_activation::ExternalPoolAdapterAtomicActivationReceipt,
        external_pool_adapter_route_renewal::{
            ExternalPoolAdapterRouteRenewalReceipt, ROUTE_RENEWAL_RENEW_BEFORE_SECONDS,
        },
    },
    store::compute_external_pool_adapter_provider_active_successor::historical_external_pool_adapter_atomic_activation_history_for_binding_on as historical_v277_on,
};

mod route;

pub(super) use route::{
    effective_expires_at, route_leaf_is_current_on, sealed_route_for_receipt_on, sealed_route_on,
};

use super::{
    receipt::receipt_by_id_on,
    types::{
        CurrentExternalPoolAdapterRenewedRouteAuthority, ExternalPoolAdapterRouteRenewalDecision,
        HistoricalExternalPoolAdapterRouteRecoveryAuthority,
    },
};

pub(in crate::store) fn historical_external_pool_adapter_route_recovery_authority_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    activation_receipt_id: &str,
    expected_activation_receipt_digest: &str,
    activation_genesis_successor_receipt_id: &str,
    expected_activation_genesis_successor_receipt_digest: &str,
    checked_at: &str,
) -> Result<Option<HistoricalExternalPoolAdapterRouteRecoveryAuthority<'tx, 'conn>>> {
    canonical_timestamp(checked_at)?;
    let stored = activation_identity_on(transaction, activation_receipt_id)?;
    let Some((receipt, provider_binding_id)) = stored else {
        return Ok(None);
    };
    let activation = &receipt.activation;
    ensure!(
        receipt.activation_receipt_digest == expected_activation_receipt_digest,
        "V278 historical V277 digest is not exact"
    );
    let historical = historical_v277_on(transaction, &provider_binding_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 historical V277 authority disappeared"))?;
    ensure!(
        historical.receipt().activation_receipt_id == activation_receipt_id
            && historical.receipt().activation_receipt_digest == expected_activation_receipt_digest
            && historical.genesis().successor.lineage.successor_sequence == 1
            && historical.genesis().active_successor_receipt_id
                == activation_genesis_successor_receipt_id
            && historical.genesis().receipt_digest
                == expected_activation_genesis_successor_receipt_digest,
        "V278 V277/sequence-one historical witness is not exact"
    );
    Ok(Some(
        HistoricalExternalPoolAdapterRouteRecoveryAuthority::new(
            transaction,
            historical,
            checked_at.to_owned(),
        ),
    ))
}

pub(in crate::store) fn external_pool_adapter_route_renewal_head_identity_on(
    transaction: &Transaction<'_>,
    provider_binding_id: &str,
    expected_activation_receipt_id: &str,
    expected_activation_receipt_digest: &str,
) -> Result<Option<(String, String)>> {
    let rows = lineage_on(transaction, provider_binding_id)?;
    if rows.is_empty() {
        return Ok(None);
    }
    for (index, receipt) in rows.iter().enumerate() {
        let identity = &receipt.renewal.identity;
        ensure!(
            identity.renewal_sequence == index as i64 + 1
                && receipt.renewal.activation_witness.activation_receipt_id
                    == expected_activation_receipt_id
                && receipt.renewal.activation_witness.activation_receipt_digest
                    == expected_activation_receipt_digest,
            "V278 route lineage has a gap or activation drift"
        );
        if index == 0 {
            ensure!(
                identity.predecessor_route_renewal_receipt_id.is_none(),
                "V278 first renewal has a predecessor"
            );
        } else {
            let predecessor = &rows[index - 1];
            ensure!(
                identity.predecessor_route_renewal_receipt_id.as_deref()
                    == Some(&predecessor.route_renewal_receipt_id)
                    && identity.predecessor_route_renewal_receipt_digest.as_deref()
                        == Some(&predecessor.route_renewal_receipt_digest),
                "V278 route lineage predecessor is not exact"
            );
        }
    }
    let head = rows.last().expect("non-empty checked above");
    audit_stable_roots_on(transaction, head)?;
    Ok(Some((
        head.route_renewal_receipt_id.clone(),
        head.route_renewal_receipt_digest.clone(),
    )))
}

pub(in crate::store) fn external_pool_adapter_route_renewal_decision_on(
    transaction: &Transaction<'_>,
    provider_binding_id: &str,
    expected_activation_receipt_id: &str,
    expected_activation_receipt_digest: &str,
    checked_at: &str,
) -> Result<ExternalPoolAdapterRouteRenewalDecision> {
    canonical_timestamp(checked_at)?;
    let head = external_pool_adapter_route_renewal_head_identity_on(
        transaction,
        provider_binding_id,
        expected_activation_receipt_id,
        expected_activation_receipt_digest,
    )?;
    let Some((receipt_id, receipt_digest)) = head else {
        audit_stable_activation_roots_on(
            transaction,
            expected_activation_receipt_id,
            expected_activation_receipt_digest,
        )?;
        return Ok(ExternalPoolAdapterRouteRenewalDecision::RenewalRequired {
            predecessor_route_renewal_receipt_id: None,
            predecessor_route_renewal_receipt_digest: None,
        });
    };
    let receipt = receipt_by_id_on(transaction, &receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 route head disappeared"))?;
    let renew_at = canonical_timestamp(checked_at)?
        .checked_add_signed(Duration::seconds(ROUTE_RENEWAL_RENEW_BEFORE_SECONDS))
        .ok_or_else(|| anyhow::anyhow!("V278 renew-before overflow"))?
        .to_rfc3339_opts(SecondsFormat::Nanos, true);
    if route_leaf_is_current_on(transaction, &receipt, checked_at, &renew_at)? {
        Ok(ExternalPoolAdapterRouteRenewalDecision::Current {
            route_renewal_receipt_id: receipt_id,
            route_renewal_receipt_digest: receipt_digest,
        })
    } else {
        Ok(ExternalPoolAdapterRouteRenewalDecision::RenewalRequired {
            predecessor_route_renewal_receipt_id: Some(receipt_id),
            predecessor_route_renewal_receipt_digest: Some(receipt_digest),
        })
    }
}

pub(in crate::store) fn require_current_external_pool_adapter_renewed_route_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    route_renewal_receipt_id: &str,
    expected_route_renewal_receipt_digest: &str,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterRenewedRouteAuthority<'tx, 'conn>>> {
    canonical_timestamp(checked_at)?;
    let Some(receipt) = receipt_by_id_on(transaction, route_renewal_receipt_id)? else {
        return Ok(None);
    };
    ensure!(
        receipt.route_renewal_receipt_digest == expected_route_renewal_receipt_digest,
        "V278 current receipt digest is not exact"
    );
    let head = external_pool_adapter_route_renewal_head_identity_on(
        transaction,
        &receipt.renewal.identity.provider_binding_id,
        &receipt.renewal.activation_witness.activation_receipt_id,
        &receipt.renewal.activation_witness.activation_receipt_digest,
    )?;
    ensure!(
        head.as_ref()
            .is_some_and(|(id, digest)| id == route_renewal_receipt_id
                && digest == expected_route_renewal_receipt_digest),
        "V278 receipt is not the unique current head"
    );
    if !route_leaf_is_current_on(transaction, &receipt, checked_at, checked_at)? {
        return Ok(None);
    }
    let route = sealed_route_for_receipt_on(transaction, &receipt)?;
    let effective_expires_at = effective_expires_at(&route, &receipt);
    Ok(Some(CurrentExternalPoolAdapterRenewedRouteAuthority::new(
        transaction,
        receipt,
        route,
        checked_at.to_owned(),
        effective_expires_at,
    )))
}

fn lineage_on(
    transaction: &Transaction<'_>,
    provider_binding_id: &str,
) -> Result<Vec<ExternalPoolAdapterRouteRenewalReceipt>> {
    let mut statement = transaction.prepare(
        "SELECT route_renewal_receipt_id
           FROM compute_external_pool_adapter_route_renewal_receipts
          WHERE provider_binding_id=?1 ORDER BY renewal_sequence",
    )?;
    let ids = statement
        .query_map([provider_binding_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.into_iter()
        .map(|id| {
            receipt_by_id_on(transaction, &id)?
                .ok_or_else(|| anyhow::anyhow!("V278 lineage row disappeared"))
        })
        .collect()
}

fn activation_identity_on(
    transaction: &Transaction<'_>,
    activation_receipt_id: &str,
) -> Result<Option<(ExternalPoolAdapterAtomicActivationReceipt, String)>> {
    transaction
        .query_row(
            "SELECT activation_receipt_json,provider_binding_id
               FROM compute_external_pool_adapter_atomic_activation_receipts
              WHERE activation_receipt_id=?1",
            [activation_receipt_id],
            |row| {
                let json: String = row.get(0)?;
                let receipt = serde_json::from_str(&json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
                })?;
                Ok((receipt, row.get(1)?))
            },
        )
        .optional()
        .map_err(Into::into)
}

fn audit_stable_activation_roots_on(
    transaction: &Transaction<'_>,
    activation_receipt_id: &str,
    activation_receipt_digest: &str,
) -> Result<()> {
    let (receipt, _) = activation_identity_on(transaction, activation_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 activation history disappeared"))?;
    ensure!(
        receipt.activation_receipt_digest == activation_receipt_digest,
        "V278 activation digest drifted"
    );
    let target = &receipt
        .activation
        .provider_transition
        .target_active_provider;
    let route = &receipt.activation.route_closure;
    stable_query(
        transaction,
        &receipt.activation.identity.provider_binding_id,
        &receipt.activation.identity.provider_binding_digest,
        &receipt.activation.identity.activation_root_digest,
        &target.provider_id,
        &route.route_adapter_projection_id,
        route.route_adapter_revision,
        &route.route_adapter_digest,
        &route.service_actor_id,
    )
}

fn audit_stable_roots_on(
    transaction: &Transaction<'_>,
    receipt: &ExternalPoolAdapterRouteRenewalReceipt,
) -> Result<()> {
    let r = &receipt.renewal;
    stable_query(
        transaction,
        &r.identity.provider_binding_id,
        &r.identity.provider_binding_digest,
        &r.identity.activation_root_digest,
        &r.active_subject.active_provider_id,
        &r.stable_binding.route_adapter_projection_id,
        r.stable_binding.route_adapter_revision,
        &r.stable_binding.route_adapter_digest,
        &r.renewed_route.service_actor_id,
    )
}

#[allow(clippy::too_many_arguments)]
fn stable_query(
    transaction: &Transaction<'_>,
    provider_binding_id: &str,
    provider_binding_digest: &str,
    activation_root_digest: &str,
    provider_id: &str,
    adapter_id: &str,
    adapter_revision: i64,
    adapter_digest: &str,
    service_actor_id: &str,
) -> Result<()> {
    historical_v277_on(transaction, provider_binding_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 stable V277 history disappeared"))?;
    let exact: bool = transaction.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM compute_providers provider
             JOIN compute_route_adapters adapter ON adapter.adapter_id=:adapter_id
             JOIN compute_external_pool_adapter_atomic_activation_receipts activation
               ON activation.provider_binding_id=:binding_id
              AND activation.provider_binding_digest=:binding_digest
              AND activation.activation_root_digest=:root_digest
             JOIN compute_external_pool_adapter_provider_active_successor_receipts genesis
               ON genesis.activation_witness_id=activation.activation_receipt_id
              AND genesis.activation_witness_digest=activation.activation_receipt_digest
              AND genesis.activation_root_digest=activation.activation_root_digest
              AND genesis.successor_sequence=1
             JOIN compute_external_pool_provider_activation_delegations delegation
               ON delegation.delegation_id=json_extract(genesis.activation_root_json,'$.activation_root.delegation_id')
              AND delegation.delegation_digest=json_extract(genesis.activation_root_json,'$.activation_root.delegation_digest')
            WHERE provider.provider_id=:provider_id AND provider.provider_kind='external_pool'
              AND provider.status='active'
              AND provider.owner_account_id=json_extract(genesis.activation_root_json,'$.activation_root.provider_owner_account_id')
              AND adapter.current_adapter_revision=:adapter_revision
              AND adapter.current_adapter_digest=:adapter_digest AND adapter.status='active'
              AND delegation.service_actor_id=:service_actor_id
              AND NOT EXISTS(SELECT 1 FROM compute_external_pool_provider_activation_delegation_revocations revoked
                              WHERE revoked.delegation_id=delegation.delegation_id
                                AND revoked.delegation_digest=delegation.delegation_digest))",
        named_params! {
            ":binding_id": provider_binding_id, ":binding_digest": provider_binding_digest,
            ":root_digest": activation_root_digest,
            ":provider_id": provider_id, ":adapter_id": adapter_id,
            ":adapter_revision": adapter_revision, ":adapter_digest": adapter_digest,
            ":service_actor_id": service_actor_id,
        },
        |row| row.get(0),
    )?;
    ensure!(
        exact,
        "V278 stable activation/executor route roots drifted or were revoked"
    );
    Ok(())
}

fn canonical_timestamp(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("V278 checked_at is not canonical")
    }
    Ok(parsed)
}
