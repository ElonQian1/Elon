//! One-shot HostReceipt + semantic value ingress into the immutable V273 receipt ledger.

use anyhow::{ensure, Result};
use std::marker::PhantomData;

use rusqlite::Transaction;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::compute_federation::external_pool_adapter_broker_tls::VerifiedExternalPoolAdapterBrokerTaskExchange;
use crate::compute_federation::{
    external_pool_adapter_broker_tls::ExternalPoolAdapterBrokerTaskVerifiedObservation,
    external_pool_adapter_task_protocol_production::{
        canonical_task_production_exchange_receipt_json_and_digest,
        ExternalPoolAdapterTaskExchangeReceiptEnvelope,
        ExternalPoolAdapterTaskExchangeReceiptMaterial, ExternalPoolAdapterTaskProductionBoundary,
        ExternalPoolAdapterTaskProductionEffects, ExternalPoolAdapterTaskProductionReadiness,
        TASK_PRODUCTION_CANONICALIZATION, TASK_PRODUCTION_DIGEST_ALGORITHM,
        TASK_PRODUCTION_EXCHANGE_RECEIPT_SCHEMA, TASK_PRODUCTION_NO_V213_AUTHORITY,
    },
};

use super::{
    historical_cleanup::HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority,
    ingress_obligation::PendingTaskIngressObligation,
    install_external_pool_adapter_task_reachability_pending_plan_on,
    mapping::exchange_receipt_values,
    reachability_pending_plan::{
        ExternalPoolAdapterTaskReachabilityPendingPlan,
        ExternalPoolAdapterTaskReachabilityPendingWrite,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    },
    read::read_exchange_attempt_on,
    types::ExternalPoolAdapterTaskLedgerWriteDisposition,
    write::{
        exchange_receipt_needs_insert_on, insert_external_pool_adapter_task_exchange_receipt_on,
    },
};

/// Receipt and semantic observation remain paired and non-Clone until a typed ingress consumes it.
pub(in crate::store) struct PendingExternalPoolAdapterTaskReceiptIngress<'tx, 'conn, T> {
    envelope: ExternalPoolAdapterTaskExchangeReceiptEnvelope,
    semantic: T,
    disposition: ExternalPoolAdapterTaskLedgerWriteDisposition,
    cleanup_expires_at: String,
    obligation: PendingTaskIngressObligation,
    connection_key: usize,
    _transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'tx, 'conn, T> PendingExternalPoolAdapterTaskReceiptIngress<'tx, 'conn, T> {
    pub(in crate::store) fn receipt(&self) -> &ExternalPoolAdapterTaskExchangeReceiptEnvelope {
        &self.envelope
    }

    pub(super) fn into_parts_on(
        self,
        transaction: &Transaction<'_>,
    ) -> Result<(
        ExternalPoolAdapterTaskExchangeReceiptEnvelope,
        T,
        PendingTaskIngressObligation,
    )> {
        ensure!(
            self.connection_key == connection_key(transaction),
            "V278 receipt ingress changed SQLite transaction connection"
        );
        Ok((self.envelope, self.semantic, self.obligation))
    }

    pub(super) fn disposition(&self) -> ExternalPoolAdapterTaskLedgerWriteDisposition {
        self.disposition
    }

    pub(super) fn cleanup_expires_at(&self) -> &str {
        &self.cleanup_expires_at
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) fn insert_external_pool_adapter_task_receipt_ingress_on<
    'tx,
    'conn,
    T: ExternalPoolAdapterBrokerTaskVerifiedObservation,
>(
    connection: &'tx Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    exchange_attempt_id: &str,
    verified_exchange: VerifiedExternalPoolAdapterBrokerTaskExchange<T>,
    authenticated_at: &str,
    received_at: &str,
    recorded_at: &str,
) -> Result<PendingExternalPoolAdapterTaskReceiptIngress<'tx, 'conn, T>> {
    let (host_receipt, semantic) = verified_exchange.into_parts();
    ensure!(
        recorded_at == authority.checked_at(),
        "V278 receipt recorded_at differs from its historical reproof"
    );
    let attempt = read_exchange_attempt_on(connection, exchange_attempt_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 receipt ingress attempt is missing"))?;
    let identity = &attempt.attempt.identity;
    let historical = &authority.exchange_attempt().attempt.identity;
    ensure!(
        identity.command == historical.command
            && identity.route == historical.route
            && identity.adapter == historical.adapter
            && identity.executor_binding_digest == historical.executor_binding_digest
            && identity.fencing_generation == historical.fencing_generation
            && identity.fence_digest == historical.fence_digest
            && host_receipt.operation().as_str() == identity.operation_kind
            && host_receipt.command_digest_hex() == identity.command.command_digest
            && host_receipt.outbox_operation_digest_hex() == identity.command.outbox_digest
            && host_receipt.delivery_attempt_digest_hex() == identity.delivery_attempt_digest
            && host_receipt.route_authorization_digest_hex()
                == identity.route.route_authorization_digest
            && host_receipt.executor_binding_digest_hex() == identity.executor_binding_digest
            && host_receipt.fence_digest_hex() == identity.fence_digest
            && host_receipt.request_digest_hex() == identity.request_digest
            && host_receipt.session_transcript_digest_hex()
                == identity.session.session_transcript_digest,
        "V278 HostReceipt does not bind the exact durable exchange attempt"
    );
    let mut envelope = ExternalPoolAdapterTaskExchangeReceiptEnvelope {
        schema: TASK_PRODUCTION_EXCHANGE_RECEIPT_SCHEMA.to_string(),
        exchange_receipt_id: format!(
            "external_pool_task_receipt_{}",
            host_receipt.exchange_root_hex()
        ),
        exchange_receipt_digest: String::new(),
        canonicalization: TASK_PRODUCTION_CANONICALIZATION.to_string(),
        digest_algorithm: TASK_PRODUCTION_DIGEST_ALGORITHM.to_string(),
        receipt: ExternalPoolAdapterTaskExchangeReceiptMaterial {
            exchange_attempt_id: attempt.exchange_attempt_id.clone(),
            exchange_attempt_digest: attempt.exchange_attempt_digest.clone(),
            identity: identity.clone(),
            exchange_ordinal: host_receipt.ordinal(),
            exchange_nonce_digest: host_receipt.exchange_nonce_digest_hex(),
            upstream_request_bytes: u64::from(host_receipt.upstream_request_bytes()),
            upstream_request_sha256: host_receipt.upstream_request_sha256_hex(),
            upstream_response_bytes: u64::from(host_receipt.upstream_response_bytes()),
            upstream_response_sha256: host_receipt.upstream_response_sha256_hex(),
            semantic_observation_bytes: u64::from(host_receipt.semantic_observation_bytes()),
            semantic_observation_sha256: host_receipt.semantic_observation_sha256_hex(),
            session_transcript_digest: host_receipt.session_transcript_digest_hex(),
            exchange_root: host_receipt.exchange_root_hex(),
            authenticated_at: authenticated_at.to_string(),
            received_at: received_at.to_string(),
            recorded_at: recorded_at.to_string(),
            boundary: ExternalPoolAdapterTaskProductionBoundary {
                authority_status: TASK_PRODUCTION_NO_V213_AUTHORITY.to_string(),
                effects: ExternalPoolAdapterTaskProductionEffects::none(),
                readiness: ExternalPoolAdapterTaskProductionReadiness::none(),
            },
        },
    };
    envelope.exchange_receipt_digest =
        canonical_task_production_exchange_receipt_json_and_digest(&envelope)?.1;
    let obligation =
        authority.register_ingress_obligation(connection, &envelope.exchange_receipt_id)?;
    if !exchange_receipt_needs_insert_on(connection, &envelope)? {
        insert_external_pool_adapter_task_exchange_receipt_on(connection, None, &envelope)?;
        return Ok(PendingExternalPoolAdapterTaskReceiptIngress {
            envelope,
            semantic,
            disposition: ExternalPoolAdapterTaskLedgerWriteDisposition::ExactReplay,
            cleanup_expires_at: authority.cleanup_expires_at().to_string(),
            obligation,
            connection_key: connection_key(connection),
            _transaction: PhantomData,
        });
    }
    let values = exchange_receipt_values(&envelope)?;
    let pending = ExternalPoolAdapterTaskReachabilityPendingPlan::new(vec![
        ExternalPoolAdapterTaskReachabilityPendingWrite::new(
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::ExchangeReceipt,
            values,
        )?,
    ])?;
    let pending =
        install_external_pool_adapter_task_reachability_pending_plan_on(connection, pending)?;
    insert_external_pool_adapter_task_exchange_receipt_on(connection, Some(&pending), &envelope)?;
    pending.ensure_fully_consumed()?;
    Ok(PendingExternalPoolAdapterTaskReceiptIngress {
        envelope,
        semantic,
        disposition: ExternalPoolAdapterTaskLedgerWriteDisposition::Inserted,
        cleanup_expires_at: authority.cleanup_expires_at().to_string(),
        obligation,
        connection_key: connection_key(connection),
        _transaction: PhantomData,
    })
}

fn connection_key(connection: &rusqlite::Connection) -> usize {
    // SAFETY: the handle is used only as identity while the transaction borrow is alive.
    unsafe { connection.handle() as usize }
}
