//! Transactional authority kernel for one interactive-desktop Session head.

use std::marker::PhantomData;

use anyhow::Result;

use crate::compute_federation::interactive_desktop::{
    authority_record::InteractiveDesktopAuthorityRecord,
    session::{InteractiveDesktopAction, InteractiveDesktopSessionState},
};

use super::{node_credentials::NodeEndpointSessionPermit, Store};

mod read;
mod sources;
mod write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InteractiveDesktopAuthorityHeadExpectation {
    session_id: String,
    session_revision: i64,
    session_digest: String,
    authority_record_digest: String,
}

impl InteractiveDesktopAuthorityHeadExpectation {
    pub(crate) fn new(
        session_id: String,
        session_revision: i64,
        session_digest: String,
        authority_record_digest: String,
    ) -> Self {
        Self {
            session_id,
            session_revision,
            session_digest,
            authority_record_digest,
        }
    }

    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(crate) fn session_revision(&self) -> i64 {
        self.session_revision
    }

    pub(crate) fn session_digest(&self) -> &str {
        &self.session_digest
    }

    pub(crate) fn authority_record_digest(&self) -> &str {
        &self.authority_record_digest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InteractiveDesktopAuthorityCommitDisposition {
    Inserted,
    ExactCurrentReplay,
}

pub(crate) struct CommittedInteractiveDesktopAuthority {
    current: InteractiveDesktopAuthorityHeadExpectation,
    disposition: InteractiveDesktopAuthorityCommitDisposition,
}

impl CommittedInteractiveDesktopAuthority {
    pub(crate) fn current(&self) -> &InteractiveDesktopAuthorityHeadExpectation {
        &self.current
    }

    pub(crate) fn disposition(&self) -> InteractiveDesktopAuthorityCommitDisposition {
        self.disposition
    }
}

/// A current authority proved against one open transaction. It is deliberately neither Clone nor
/// serializable and cannot be constructed outside this Store module.
pub(in crate::store) struct CurrentInteractiveDesktopAuthority<'tx, 'conn> {
    record: InteractiveDesktopAuthorityRecord,
    _transaction: PhantomData<&'tx rusqlite::Transaction<'conn>>,
}

impl CurrentInteractiveDesktopAuthority<'_, '_> {
    pub(in crate::store) fn record(&self) -> &InteractiveDesktopAuthorityRecord {
        &self.record
    }
}

impl Store {
    /// Source-only Store seam. Keep this unavailable to services and routes until active and
    /// non-authorizing transitions receive separate typed producer authorities.
    pub(in crate::store) fn commit_interactive_desktop_authority(
        &self,
        record: &InteractiveDesktopAuthorityRecord,
        expected_head: Option<&InteractiveDesktopAuthorityHeadExpectation>,
        host_endpoint_session: &NodeEndpointSessionPermit,
        consumer_bearer_token: &str,
        observed_viewer_device_key_digest: &str,
        observed_viewer_transport_identity_digest: &str,
    ) -> Result<CommittedInteractiveDesktopAuthority> {
        write::commit(
            self,
            record,
            expected_head,
            host_endpoint_session,
            consumer_bearer_token,
            observed_viewer_device_key_digest,
            observed_viewer_transport_identity_digest,
        )
    }
}

pub(super) fn committed(
    current: InteractiveDesktopAuthorityHeadExpectation,
    disposition: InteractiveDesktopAuthorityCommitDisposition,
) -> CommittedInteractiveDesktopAuthority {
    CommittedInteractiveDesktopAuthority {
        current,
        disposition,
    }
}

pub(super) fn current_authority<'tx, 'conn>(
    record: InteractiveDesktopAuthorityRecord,
) -> CurrentInteractiveDesktopAuthority<'tx, 'conn> {
    CurrentInteractiveDesktopAuthority {
        record,
        _transaction: PhantomData,
    }
}

pub(in crate::store) fn require_current_interactive_desktop_authority_on<'tx, 'conn>(
    transaction: &'tx rusqlite::Transaction<'conn>,
    expected: &InteractiveDesktopAuthorityHeadExpectation,
    host_endpoint_session: &NodeEndpointSessionPermit,
    consumer_bearer_token: &str,
    observed_viewer_device_key_digest: &str,
    observed_viewer_transport_identity_digest: &str,
    action: InteractiveDesktopAction,
) -> Result<CurrentInteractiveDesktopAuthority<'tx, 'conn>> {
    read::require_current_on(
        transaction,
        expected,
        host_endpoint_session,
        consumer_bearer_token,
        observed_viewer_device_key_digest,
        observed_viewer_transport_identity_digest,
        action,
    )
}

pub(super) fn session_state_name(state: InteractiveDesktopSessionState) -> &'static str {
    match state {
        InteractiveDesktopSessionState::Requested => "requested",
        InteractiveDesktopSessionState::Reserved => "reserved",
        InteractiveDesktopSessionState::HostLeased => "host_leased",
        InteractiveDesktopSessionState::ViewerGranted => "viewer_granted",
        InteractiveDesktopSessionState::Connecting => "connecting",
        InteractiveDesktopSessionState::Active => "active",
        InteractiveDesktopSessionState::Reconnecting => "reconnecting",
        InteractiveDesktopSessionState::Ending => "ending",
        InteractiveDesktopSessionState::Ended => "ended",
        InteractiveDesktopSessionState::Canceled => "canceled",
        InteractiveDesktopSessionState::Failed => "failed",
    }
}
