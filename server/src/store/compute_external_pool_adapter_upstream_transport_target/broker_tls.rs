use std::time::Duration;

use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::TransactionBehavior;

use crate::compute_federation::{
    external_pool_adapter_broker_tls::{
        connect_external_pool_adapter_broker_tls, exchange_external_pool_adapter_broker_no_work,
        ExternalPoolAdapterBrokerTlsChannel, ExternalPoolAdapterBrokerTlsTarget,
    },
    external_pool_adapter_installation::{
        ExternalPoolAdapterInstallationBinding, PreparedExternalPoolAdapterInstallation,
    },
};
use zeroize::Zeroizing;

use super::{
    current_external_pool_adapter_upstream_transport_target_authority_on,
    CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
};
use crate::store::Store;

/// Store-private, short-lived authority retaining both the exact V258 roots and TLS channel.
/// It is intentionally non-Clone/non-Debug/non-Serde and exposes no network I/O.
pub(in crate::store) struct CurrentExternalPoolAdapterBrokerTlsAuthority {
    current_target: CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
    channel: ExternalPoolAdapterBrokerTlsChannel,
    checked_at: String,
}

/// Transaction-free one-shot channel prepared from exact V258 roots.
/// It retains no database connection, transaction, Prepared installation, or generic I/O seam.
pub(in crate::store) struct PreparedExternalPoolAdapterBrokerTlsChannel {
    channel: ExternalPoolAdapterBrokerTlsChannel,
    checked_at: String,
}

impl PreparedExternalPoolAdapterBrokerTlsChannel {
    pub(in crate::store) fn target(&self) -> &ExternalPoolAdapterBrokerTlsTarget {
        self.channel.target()
    }

    pub(in crate::store) fn selected_address(&self) -> std::net::SocketAddr {
        self.channel.selected_address()
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }

    pub(in crate::store) async fn exchange_no_work(
        &mut self,
        request: &[u8],
        expected_response_bytes: usize,
        timeout: Duration,
    ) -> Result<Zeroizing<Vec<u8>>> {
        exchange_external_pool_adapter_broker_no_work(
            &mut self.channel,
            request,
            expected_response_bytes,
            timeout,
        )
        .await
    }
}

impl CurrentExternalPoolAdapterBrokerTlsAuthority {
    pub(in crate::store) fn target_id(&self) -> &str {
        self.channel.target().target_id()
    }

    pub(in crate::store) fn selected_address(&self) -> std::net::SocketAddr {
        self.channel.selected_address()
    }

    pub(in crate::store) fn checked_at(&self) -> &str {
        &self.checked_at
    }
}

impl Store {
    /// Opens one authenticated channel outside SQLite locks, then re-proves the exact target and
    /// installation under an IMMEDIATE transaction before lending a metadata-only authority.
    pub(in crate::store) async fn with_current_external_pool_adapter_broker_tls(
        &self,
        target_id: &str,
        expected_target_digest: &str,
        preflight_prepared: PreparedExternalPoolAdapterInstallation,
        postflight_prepared: PreparedExternalPoolAdapterInstallation,
        consume: impl FnOnce(&CurrentExternalPoolAdapterBrokerTlsAuthority) -> Result<()>,
    ) -> Result<bool> {
        let (preflight_target, preflight_binding, broker_target) = {
            let mut connection = self.conn()?;
            let transaction = connection.transaction()?;
            let checked_at = canonical_now();
            let Some(authority) =
                current_external_pool_adapter_upstream_transport_target_authority_on(
                    &transaction,
                    target_id,
                    expected_target_digest,
                    preflight_prepared,
                    &checked_at,
                )?
            else {
                return Ok(false);
            };
            let broker_target =
                ExternalPoolAdapterBrokerTlsTarget::from_receipt(authority.target())?;
            let target = authority.target().clone();
            let binding = current_installation_binding(&authority).clone();
            transaction.commit()?;
            (target, binding, broker_target)
        };

        // No database connection, transaction, or Prepared handle crosses this network await.
        let channel = connect_external_pool_adapter_broker_tls(broker_target).await?;

        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let checked_at = canonical_now();
        let Some(current_target) =
            current_external_pool_adapter_upstream_transport_target_authority_on(
                &transaction,
                target_id,
                expected_target_digest,
                postflight_prepared,
                &checked_at,
            )?
        else {
            return Ok(false);
        };
        let postflight_broker_target =
            ExternalPoolAdapterBrokerTlsTarget::from_receipt(current_target.target())?;
        if current_target.target() != &preflight_target
            || current_installation_binding(&current_target) != &preflight_binding
            || channel.target() != &postflight_broker_target
            || channel.target().target_id() != target_id
            || channel.target().target_digest() != expected_target_digest
            || !channel.is_current()
            || current_target.checked_at() != checked_at
        {
            bail!("broker TLS authority changed before use");
        }

        let authority = CurrentExternalPoolAdapterBrokerTlsAuthority {
            current_target,
            channel,
            checked_at,
        };
        consume(&authority)?;
        if authority.current_target.checked_at() != authority.checked_at {
            bail!("broker TLS currentness observation changed during use");
        }
        transaction.commit()?;
        Ok(true)
    }

    /// Connects outside SQLite locks, re-proves currentness, commits the postflight transaction,
    /// then returns a one-shot channel that carries no Prepared or database authority.
    pub(in crate::store) async fn prepare_current_external_pool_adapter_broker_tls_channel(
        &self,
        target_id: &str,
        expected_target_digest: &str,
        preflight_prepared: PreparedExternalPoolAdapterInstallation,
        postflight_prepared: PreparedExternalPoolAdapterInstallation,
    ) -> Result<Option<PreparedExternalPoolAdapterBrokerTlsChannel>> {
        let (preflight_target, preflight_binding, broker_target) = {
            let mut connection = self.conn()?;
            let transaction = connection.transaction()?;
            let checked_at = canonical_now();
            let Some(authority) =
                current_external_pool_adapter_upstream_transport_target_authority_on(
                    &transaction,
                    target_id,
                    expected_target_digest,
                    preflight_prepared,
                    &checked_at,
                )?
            else {
                return Ok(None);
            };
            let broker_target =
                ExternalPoolAdapterBrokerTlsTarget::from_receipt(authority.target())?;
            let target = authority.target().clone();
            let binding = current_installation_binding(&authority).clone();
            transaction.commit()?;
            (target, binding, broker_target)
        };

        // No database connection, transaction, or Prepared handle crosses this network await.
        let channel = connect_external_pool_adapter_broker_tls(broker_target).await?;

        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let checked_at = canonical_now();
        let Some(current_target) =
            current_external_pool_adapter_upstream_transport_target_authority_on(
                &transaction,
                target_id,
                expected_target_digest,
                postflight_prepared,
                &checked_at,
            )?
        else {
            return Ok(None);
        };
        let postflight_broker_target =
            ExternalPoolAdapterBrokerTlsTarget::from_receipt(current_target.target())?;
        if current_target.target() != &preflight_target
            || current_installation_binding(&current_target) != &preflight_binding
            || channel.target() != &postflight_broker_target
            || channel.target().target_id() != target_id
            || channel.target().target_digest() != expected_target_digest
            || !channel.is_current()
            || current_target.checked_at() != checked_at
        {
            bail!("prepared broker TLS channel roots changed");
        }
        drop(current_target);
        transaction.commit()?;
        Ok(Some(PreparedExternalPoolAdapterBrokerTlsChannel {
            channel,
            checked_at,
        }))
    }
}

fn current_installation_binding(
    authority: &CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
) -> &ExternalPoolAdapterInstallationBinding {
    authority
        .profile()
        .candidate()
        .registry()
        .prepared()
        .binding()
}

fn canonical_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
