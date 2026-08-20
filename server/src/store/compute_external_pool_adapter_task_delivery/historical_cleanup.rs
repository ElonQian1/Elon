//! Historical cleanup authority derived only from one durable V273 execution exchange.

use std::marker::PhantomData;

use anyhow::{ensure, Result};
use chrono::{DateTime, SecondsFormat};
use rusqlite::{params, Transaction, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_task_protocol_production::ExternalPoolAdapterTaskExchangeAttemptEnvelope,
    store::compute_external_pool_adapter_route_renewal::{
        historical_external_pool_adapter_route_recovery_authority_on,
        HistoricalExternalPoolAdapterRouteRecoveryAuthority,
    },
};

use super::{
    ingress_obligation::{ExternalPoolAdapterTaskIngressSession, PendingTaskIngressObligation},
    read::read_exchange_attempt_on,
    Store,
};

pub(in crate::store) struct HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn> {
    exchange_attempt: ExternalPoolAdapterTaskExchangeAttemptEnvelope,
    route: HistoricalExternalPoolAdapterRouteRecoveryAuthority<'tx, 'conn>,
    cleanup_expires_at: String,
    ingress_session: ExternalPoolAdapterTaskIngressSession,
    _transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'tx, 'conn> HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn> {
    pub(in crate::store) fn exchange_attempt(
        &self,
    ) -> &ExternalPoolAdapterTaskExchangeAttemptEnvelope {
        &self.exchange_attempt
    }

    pub(in crate::store) fn route(
        &self,
    ) -> &HistoricalExternalPoolAdapterRouteRecoveryAuthority<'tx, 'conn> {
        &self.route
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        self.route.checked_at()
    }

    pub(in crate::store) fn cleanup_expires_at(&self) -> &str {
        &self.cleanup_expires_at
    }

    pub(super) fn register_ingress_obligation(
        &self,
        transaction: &Transaction<'_>,
        receipt_id: &str,
    ) -> Result<PendingTaskIngressObligation> {
        self.ingress_session.register(transaction, receipt_id)
    }

    fn ensure_ingress_obligations_resolved(&self, transaction: &Transaction<'_>) -> Result<()> {
        self.ingress_session.ensure_resolved(transaction)
    }
}

impl Store {
    pub(in crate::store) fn with_historical_external_pool_adapter_task_exchange_cleanup<Output>(
        &self,
        exchange_attempt_id: &str,
        checked_at: &str,
        consume: impl FnOnce(
            &Transaction<'_>,
            &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'_, '_>,
        ) -> Result<Output>,
    ) -> Result<Option<Output>> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(exchange_attempt) = read_exchange_attempt_on(&transaction, exchange_attempt_id)?
        else {
            return Ok(None);
        };
        let identity = &exchange_attempt.attempt.identity;
        ensure!(
            identity.source.source_kind == "start_outbox_send_attempt"
                && matches!(
                    identity.operation_kind.as_str(),
                    "prepare" | "idempotent_commit" | "cancel_no_start"
                ),
            "V278 historical cleanup source is not a durable execution send"
        );
        let witness = route_witness_on(&transaction, &exchange_attempt, checked_at)?;
        let Some(route) = historical_external_pool_adapter_route_recovery_authority_on(
            &transaction,
            &witness.activation_receipt_id,
            &witness.activation_receipt_digest,
            &witness.genesis_receipt_id,
            &witness.genesis_receipt_digest,
            checked_at,
        )?
        else {
            return Ok(None);
        };
        let authority = HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority {
            exchange_attempt,
            route,
            cleanup_expires_at: witness.cleanup_expires_at,
            ingress_session: ExternalPoolAdapterTaskIngressSession::new(&transaction),
            _transaction: PhantomData,
        };
        let output = consume(&transaction, &authority)?;
        authority.ensure_ingress_obligations_resolved(&transaction)?;
        drop(authority);
        transaction.commit()?;
        Ok(Some(output))
    }
}

struct RouteWitness {
    activation_receipt_id: String,
    activation_receipt_digest: String,
    genesis_receipt_id: String,
    genesis_receipt_digest: String,
    cleanup_expires_at: String,
}

fn route_witness_on(
    transaction: &Transaction<'_>,
    exchange: &ExternalPoolAdapterTaskExchangeAttemptEnvelope,
    checked_at: &str,
) -> Result<RouteWitness> {
    let parsed = DateTime::parse_from_rfc3339(checked_at)?;
    ensure!(
        parsed.offset().local_minus_utc() == 0
            && parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) == checked_at,
        "V278 historical cleanup time is not canonical UTC nanos"
    );
    let identity = &exchange.attempt.identity;
    let route_credential_revision = i64::try_from(identity.route.route_credential_revision)?;
    let adapter_revision = i64::try_from(identity.adapter.adapter_revision)?;
    let parameters = params![
        identity.route.route_authorization_id,
        identity.route.route_authorization_digest,
        identity.route.route_credential_id,
        route_credential_revision,
        identity.route.route_credential_digest,
        identity.adapter.provider_id,
        identity.adapter.adapter_id,
        adapter_revision,
        identity.adapter.adapter_registry_digest,
        identity.executor_binding_digest,
        checked_at,
    ];
    let mut renewed = transaction.prepare(
        "SELECT activation_receipt_id,activation_receipt_digest,
                activation_genesis_successor_receipt_id,
                activation_genesis_successor_receipt_digest,cleanup_expires_at
           FROM compute_external_pool_adapter_route_renewal_receipts
          WHERE route_authorization_id=?1 AND route_authorization_digest=?2
            AND route_credential_id=?3 AND route_credential_revision=?4
            AND route_credential_digest=?5 AND active_provider_id=?6
            AND route_adapter_projection_id=?7 AND route_adapter_revision=?8
            AND route_adapter_digest=?9 AND stable_executor_binding_digest=?10
            AND ?11<cleanup_expires_at",
    )?;
    let mut witnesses = renewed
        .query_map(parameters, route_witness_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut genesis = transaction.prepare(
        "SELECT activation.activation_receipt_id,activation.activation_receipt_digest,
                successor.active_successor_receipt_id,successor.receipt_digest,
                authorization.cleanup_expires_at
           FROM compute_external_pool_adapter_atomic_activation_receipts activation
           JOIN compute_external_pool_adapter_provider_active_successor_receipts successor
             ON successor.successor_sequence=1
            AND successor.activation_witness_id=activation.activation_receipt_id
            AND successor.activation_witness_digest=activation.activation_receipt_digest
           JOIN compute_route_authorization_receipts authorization
             ON authorization.route_authorization_id=activation.route_authorization_id
            AND authorization.route_authorization_revision=activation.route_authorization_revision
            AND authorization.route_authorization_digest=activation.route_authorization_digest
            AND authorization.credential_id=activation.route_credential_id
            AND authorization.credential_revision=activation.route_credential_revision
            AND authorization.credential_digest=activation.route_credential_digest
          WHERE activation.route_authorization_id=?1
            AND activation.route_authorization_digest=?2
            AND activation.route_credential_id=?3
            AND activation.route_credential_revision=?4
            AND activation.route_credential_digest=?5
            AND activation.target_active_provider_id=?6
            AND activation.route_adapter_projection_id=?7
            AND activation.route_adapter_revision=?8
            AND activation.route_adapter_digest=?9
            AND activation.stable_executor_binding_digest=?10
            AND ?11<authorization.cleanup_expires_at",
    )?;
    witnesses.extend(
        genesis
            .query_map(
                params![
                    identity.route.route_authorization_id,
                    identity.route.route_authorization_digest,
                    identity.route.route_credential_id,
                    route_credential_revision,
                    identity.route.route_credential_digest,
                    identity.adapter.provider_id,
                    identity.adapter.adapter_id,
                    adapter_revision,
                    identity.adapter.adapter_registry_digest,
                    identity.executor_binding_digest,
                    checked_at,
                ],
                route_witness_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?,
    );
    ensure!(
        witnesses.len() == 1,
        "V278 historical exchange route closure has zero or multiple exact sources"
    );
    witnesses
        .pop()
        .ok_or_else(|| anyhow::anyhow!("V278 exact historical route witness disappeared"))
}

fn route_witness_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RouteWitness> {
    Ok(RouteWitness {
        activation_receipt_id: row.get(0)?,
        activation_receipt_digest: row.get(1)?,
        genesis_receipt_id: row.get(2)?,
        genesis_receipt_digest: row.get(3)?,
        cleanup_expires_at: row.get(4)?,
    })
}
