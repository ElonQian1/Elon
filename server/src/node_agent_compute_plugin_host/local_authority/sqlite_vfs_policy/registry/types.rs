use std::{fmt, num::NonZeroU64};

use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::types::ManagedSqliteLogicalFileRole;

/// Process-local identity for one future one-shot routing session.
///
/// The value has no creation API in the current build. It can only become constructible together
/// with the future routing-table owner that proves exact-token uniqueness.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct ManagedSqliteRegistrySessionId {
    value: NonZeroU64,
}

impl fmt::Debug for ManagedSqliteRegistrySessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ManagedSqliteRegistrySessionId(<opaque>)")
    }
}

/// A route-removal proof is deliberately impossible to mint in this batch. The future routing
/// owner must create it only after an exact token, session identity and entry identity all match.
#[must_use = "route-removal proof must be consumed by the matching session"]
pub(super) struct ManagedSqliteRegistryRouteRemovalProof {
    session_id: ManagedSqliteRegistrySessionId,
    route_epoch: NonZeroU64,
}

impl ManagedSqliteRegistryRouteRemovalProof {
    pub(super) fn session_id(&self) -> ManagedSqliteRegistrySessionId {
        self.session_id
    }

    pub(super) fn route_epoch(&self) -> NonZeroU64 {
        self.route_epoch
    }
}

impl fmt::Debug for ManagedSqliteRegistryRouteRemovalProof {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteRegistryRouteRemovalProof")
            .field("session_id", &self.session_id)
            .field("route_epoch", &"<opaque>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedSqliteRegistrySessionPhase {
    PendingMain,
    Opening,
    Active,
    Closing,
    AwaitingRouteRetirement,
    Retired,
    TerminalQuarantine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedSqliteRegistryCallbackKind {
    FullPathname,
    Open,
    Access,
    Delete,
    Io,
    Close,
    Shm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedSqliteRegistryTerminalReason {
    CallbackPanicked,
    CallbackCounterOverflow,
    LeaseCounterOverflow,
    LeaseIdentityMismatch,
    HandleCloseUnproven,
    ShmTeardownUnproven,
    ConnectionCloseUnproven,
    FailureCustodyRetained,
    RouteIdentityMismatch,
    StateInvariantViolated,
}

/// Exact close proof reserved for the future adapter that consumes a managed-fs close receipt.
///
/// There is intentionally no constructor in this batch, so a scalar success claim cannot retire
/// a registry lease.
#[must_use = "a close proof must retire its exact registry lease"]
pub(super) struct ManagedSqliteRegistryCloseProof {
    session_id: ManagedSqliteRegistrySessionId,
    lease_ordinal: NonZeroU64,
}

impl ManagedSqliteRegistryCloseProof {
    pub(super) fn session_id(&self) -> ManagedSqliteRegistrySessionId {
        self.session_id
    }

    pub(super) fn lease_ordinal(&self) -> NonZeroU64 {
        self.lease_ordinal
    }
}

impl fmt::Debug for ManagedSqliteRegistryCloseProof {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ManagedSqliteRegistryCloseProof(<opaque>)")
    }
}

#[derive(Debug)]
pub(super) enum ManagedSqliteRegistryCloseOutcome {
    Proven(ManagedSqliteRegistryCloseProof),
    Unproven(ManagedSqliteRegistryTerminalReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedSqliteRegistryTransitionRejection {
    WrongPhase,
    Terminal,
    SessionIdentityMismatch,
    MainAlreadyClaimed,
    MainNotClaimed,
    InvalidSidecarRole,
    LeaseCapacityExhausted,
    LeaseIdentityMismatch,
    OutstandingCallbacks,
    OutstandingHandles,
    CounterOverflow,
    RouteRemovalUnproven,
    StateInvariantViolated,
}

#[derive(Clone, Copy)]
pub(super) struct ManagedSqliteRegistryLeaseRecord {
    pub(super) ordinal: NonZeroU64,
    pub(super) role: ManagedSqliteLogicalFileRole,
}

/// Linear custody for one future `sqlite3_file`.
#[must_use = "a file lease must be consumed by its exact close transition"]
pub(super) struct ManagedSqliteRegistryFileLease {
    pub(super) session_id: ManagedSqliteRegistrySessionId,
    pub(super) ordinal: NonZeroU64,
    pub(super) role: ManagedSqliteLogicalFileRole,
}

impl ManagedSqliteRegistryFileLease {
    pub(super) fn role(&self) -> ManagedSqliteLogicalFileRole {
        self.role
    }
}

/// Linear guard for one callback already admitted by the session phase.
#[must_use = "a callback lease must be consumed when the callback exits"]
pub(super) struct ManagedSqliteRegistryCallbackLease {
    pub(super) session_id: ManagedSqliteRegistrySessionId,
    pub(super) kind: ManagedSqliteRegistryCallbackKind,
}

/// Linear record for one live SHM attachment. SHM never has a logical registry filename.
#[must_use = "an SHM lease must be consumed after exact teardown"]
pub(super) struct ManagedSqliteRegistryShmLease {
    pub(super) session_id: ManagedSqliteRegistrySessionId,
    pub(super) ordinal: NonZeroU64,
}

#[derive(Debug, PartialEq, Eq)]
#[must_use = "retirement must be followed by permanent exact-token tombstoning"]
pub(super) struct ManagedSqliteRegistryRetirementReceipt {
    pub(super) session_id: ManagedSqliteRegistrySessionId,
    pub(super) route_epoch: NonZeroU64,
    pub(super) main_was_claimed: bool,
}

impl ManagedSqliteRegistryRetirementReceipt {
    pub(super) fn session_id(&self) -> ManagedSqliteRegistrySessionId {
        self.session_id
    }

    pub(super) fn main_was_claimed(&self) -> bool {
        self.main_was_claimed
    }

    pub(super) fn route_epoch(&self) -> NonZeroU64 {
        self.route_epoch
    }
}
