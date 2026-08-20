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
        ExternalPoolAdapterInstallationBinding, ExternalPoolAdapterInstallationFsError,
        PreparedExternalPoolAdapterInstallation,
    },
    external_pool_adapter_upstream_transport_target::ExternalPoolAdapterUpstreamTransportTargetReceipt,
};
use zeroize::Zeroizing;

use super::{
    current_external_pool_adapter_upstream_transport_target_authority_on,
    CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
};
use crate::store::Store;

pub(in crate::store) type ExternalPoolAdapterInstallationReopener<'a> =
    dyn FnMut() -> std::result::Result<
            PreparedExternalPoolAdapterInstallation,
            ExternalPoolAdapterInstallationFsError,
        > + Send
        + 'a;

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
    /// Active path #1/#2: re-proves a caller-selected renewed-route carrier on both sides of the
    /// network await. The callback receives each fresh Prepared installation and cannot retain a
    /// transaction. The returned channel retains neither callback authority nor a database handle.
    pub(in crate::store) async fn prepare_projected_active_external_pool_adapter_broker_tls_channel(
        &self,
        target: &ExternalPoolAdapterUpstreamTransportTargetReceipt,
        reopen_prepared: &mut ExternalPoolAdapterInstallationReopener<'_>,
        mut reprove: impl FnMut(
                &rusqlite::Transaction<'_>,
                PreparedExternalPoolAdapterInstallation,
                &str,
            ) -> Result<bool>
            + Send,
    ) -> Result<Option<PreparedExternalPoolAdapterBrokerTlsChannel>> {
        let preflight_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let preflight_target = target.clone();
        let broker_target = {
            let mut connection = self.conn()?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let checked_at = canonical_now();
            if !reprove(&transaction, preflight_prepared, &checked_at)? {
                return Ok(None);
            }
            let broker_target = ExternalPoolAdapterBrokerTlsTarget::from_receipt(target)?;
            transaction.commit()?;
            broker_target
        };

        // No SQLite connection, transaction, Prepared installation, or authority crosses await.
        let channel = connect_external_pool_adapter_broker_tls(broker_target).await?;

        let postflight_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let checked_at = canonical_now();
        if !reprove(&transaction, postflight_prepared, &checked_at)? {
            return Ok(None);
        }
        let postflight_target = ExternalPoolAdapterBrokerTlsTarget::from_receipt(target)?;
        if target != &preflight_target
            || channel.target() != &postflight_target
            || !channel.is_current()
        {
            bail!("projected-active broker target changed across connect");
        }
        transaction.commit()?;
        Ok(Some(PreparedExternalPoolAdapterBrokerTlsChannel {
            channel,
            checked_at,
        }))
    }

    /// Opens one authenticated channel outside SQLite locks, then re-proves the exact target and
    /// installation under an IMMEDIATE transaction before lending a metadata-only authority.
    pub(in crate::store) async fn with_current_external_pool_adapter_broker_tls(
        &self,
        target_id: &str,
        expected_target_digest: &str,
        reopen_prepared: &mut ExternalPoolAdapterInstallationReopener<'_>,
        consume: impl FnOnce(&CurrentExternalPoolAdapterBrokerTlsAuthority) -> Result<()> + Send,
    ) -> Result<bool> {
        let preflight_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let (preflight_target, preflight_binding, broker_target) = {
            let mut connection = self.conn()?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
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

        let postflight_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
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
        reopen_prepared: &mut ExternalPoolAdapterInstallationReopener<'_>,
        preflight_consume: impl FnOnce(
                &rusqlite::Transaction<'_>,
                &CurrentExternalPoolAdapterUpstreamTransportTargetAuthority,
                &str,
            ) -> Result<()>
            + Send,
    ) -> Result<Option<PreparedExternalPoolAdapterBrokerTlsChannel>> {
        let preflight_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
        let (preflight_target, preflight_binding, broker_target) = {
            let mut connection = self.conn()?;
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
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
            preflight_consume(&transaction, &authority, &checked_at)?;
            let broker_target =
                ExternalPoolAdapterBrokerTlsTarget::from_receipt(authority.target())?;
            let target = authority.target().clone();
            let binding = current_installation_binding(&authority).clone();
            transaction.commit()?;
            (target, binding, broker_target)
        };

        // No database connection, transaction, or Prepared handle crosses this network await.
        let channel = connect_external_pool_adapter_broker_tls(broker_target).await?;

        let postflight_prepared = reopen_prepared().map_err(anyhow::Error::new)?;
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
