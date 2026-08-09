use std::{
    collections::{HashMap, HashSet},
    ffi::CStr,
    fmt,
    num::NonZeroU64,
};

use super::{
    state::ManagedSqliteRegistrySessionState,
    types::{
        ManagedSqliteRegistryCallbackKind, ManagedSqliteRegistryCallbackLease,
        ManagedSqliteRegistryCloseOutcome, ManagedSqliteRegistryFileLease,
        ManagedSqliteRegistryRetirementReceipt, ManagedSqliteRegistryRouteRemovalProof,
        ManagedSqliteRegistrySessionId, ManagedSqliteRegistrySessionPhase,
        ManagedSqliteRegistryShmLease, ManagedSqliteRegistryTerminalReason,
        ManagedSqliteRegistryTransitionRejection, ManagedSqliteRegistryWalMainCloseProofs,
    },
};
use crate::node_agent_compute_plugin_host::local_authority::{
    sqlite_vfs_policy::{
        ManagedSqliteAuthorizerDecision, ManagedSqliteAuthorizerRequest,
        ManagedSqliteAuthorizerTransitionError, ManagedSqliteLogicalFileRole,
        ManagedSqliteLogicalNameRejection, SealedHandleBoundSqlitePolicy,
    },
    ComputePluginHandleBoundAuthorityOpenIntent,
};

mod lifecycle;
mod vfs;

/// Future production specialization. No instance or nonce provider exists in the current build.
pub(super) type ComputePluginHandleBoundSqliteRegistryOwner =
    ManagedSqliteRegistryOwner<ComputePluginHandleBoundAuthorityOpenIntent>;

/// Registration gate implemented by the sealed authority-open intent in its defining module.
pub(in crate::node_agent_compute_plugin_host::local_authority) trait ManagedSqliteRegistryCustody {
    fn ensure_registry_current(&self) -> anyhow::Result<()>;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ManagedSqliteRegistryRouteToken([u8; 16]);

impl fmt::Debug for ManagedSqliteRegistryRouteToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ManagedSqliteRegistryRouteToken(<opaque>)")
    }
}

/// Exact route identity. All three fields must match one entry before custody can be observed.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteRegistryRouteHandle
{
    token: ManagedSqliteRegistryRouteToken,
    session_id: ManagedSqliteRegistrySessionId,
    route_epoch: NonZeroU64,
}

impl fmt::Debug for ManagedSqliteRegistryRouteHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteRegistryRouteHandle")
            .field("token", &self.token)
            .field("session_id", &self.session_id)
            .field("route_epoch", &"<opaque>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) enum ManagedSqliteRegistryRegistrationRejection
{
    CustodyNotCurrent,
    InvalidNonce(ManagedSqliteLogicalNameRejection),
    TokenAlreadyUsed,
    IdentityExhausted,
}

#[must_use = "registration failure retains the authority-open custody"]
pub(super) struct ManagedSqliteRegistryRegistrationFailure<Custody> {
    reason: ManagedSqliteRegistryRegistrationRejection,
    custody: Custody,
}

impl<Custody> ManagedSqliteRegistryRegistrationFailure<Custody> {
    pub(super) fn into_parts(self) -> (ManagedSqliteRegistryRegistrationRejection, Custody) {
        (self.reason, self.custody)
    }
}

impl<Custody> fmt::Debug for ManagedSqliteRegistryRegistrationFailure<Custody> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteRegistryRegistrationFailure")
            .field("reason", &self.reason)
            .field("custody", &"<retained>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) enum ManagedSqliteRegistryRouteRejection
{
    UnknownOrRetired,
    IdentityMismatch,
    Authorizer(ManagedSqliteAuthorizerTransitionError),
    State(ManagedSqliteRegistryTransitionRejection),
}

struct ManagedSqliteRegistryEntry<Custody> {
    state: ManagedSqliteRegistrySessionState,
    policy: SealedHandleBoundSqlitePolicy,
    // Declared last so normal retirement drops policy/state before authority-open custody.
    custody: Custody,
}

/// Single-owner route table. A future static VFS owner must keep this value for process lifetime.
#[must_use = "dropping the route owner retires every active authority-open custody"]
pub(super) struct ManagedSqliteRegistryOwner<Custody> {
    next_session_id: u64,
    next_route_epoch: u64,
    routes: HashMap<ManagedSqliteRegistryRouteToken, ManagedSqliteRegistryEntry<Custody>>,
    used_tokens: HashSet<ManagedSqliteRegistryRouteToken>,
}

impl<Custody: ManagedSqliteRegistryCustody> ManagedSqliteRegistryOwner<Custody> {
    pub(super) fn new() -> Self {
        Self {
            next_session_id: 0,
            next_route_epoch: 0,
            routes: HashMap::new(),
            used_tokens: HashSet::new(),
        }
    }

    /// Atomically inserts custody, policy and state. A failed insertion returns custody unchanged.
    pub(super) fn register(
        &mut self,
        nonce: [u8; 16],
        custody: Custody,
    ) -> Result<ManagedSqliteRegistryRouteHandle, ManagedSqliteRegistryRegistrationFailure<Custody>>
    {
        if custody.ensure_registry_current().is_err() {
            return Err(ManagedSqliteRegistryRegistrationFailure {
                reason: ManagedSqliteRegistryRegistrationRejection::CustodyNotCurrent,
                custody,
            });
        }
        let policy = match SealedHandleBoundSqlitePolicy::from_registry_nonce(nonce) {
            Ok(policy) => policy,
            Err(reason) => {
                return Err(ManagedSqliteRegistryRegistrationFailure {
                    reason: ManagedSqliteRegistryRegistrationRejection::InvalidNonce(reason),
                    custody,
                });
            }
        };
        let token = ManagedSqliteRegistryRouteToken(nonce);
        if self.used_tokens.contains(&token) {
            return Err(ManagedSqliteRegistryRegistrationFailure {
                reason: ManagedSqliteRegistryRegistrationRejection::TokenAlreadyUsed,
                custody,
            });
        }
        let Some(session_value) = self
            .next_session_id
            .checked_add(1)
            .and_then(NonZeroU64::new)
        else {
            return Err(self.exhausted(custody));
        };
        let Some(route_epoch) = self
            .next_route_epoch
            .checked_add(1)
            .and_then(NonZeroU64::new)
        else {
            return Err(self.exhausted(custody));
        };
        let session_id = ManagedSqliteRegistrySessionId::from_registry(session_value);
        let state = ManagedSqliteRegistrySessionState::new_pending(session_id, route_epoch);
        let handle = ManagedSqliteRegistryRouteHandle {
            token,
            session_id,
            route_epoch,
        };
        let prior = self.routes.insert(
            token,
            ManagedSqliteRegistryEntry {
                state,
                policy,
                custody,
            },
        );
        debug_assert!(
            prior.is_none(),
            "used token must exclude active replacement"
        );
        self.used_tokens.insert(token);
        self.next_session_id = session_value.get();
        self.next_route_epoch = route_epoch.get();
        Ok(handle)
    }

    pub(super) fn main_logical_name(
        &self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<&CStr, ManagedSqliteRegistryRouteRejection> {
        let entry = self.exact_entry(handle)?;
        Ok(entry
            .policy
            .logical_name(ManagedSqliteLogicalFileRole::Main))
    }

    pub(super) fn phase(
        &self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistrySessionPhase, ManagedSqliteRegistryRouteRejection> {
        Ok(self.exact_entry(handle)?.state.phase())
    }

    pub(super) fn authorize_sql(
        &self,
        handle: ManagedSqliteRegistryRouteHandle,
        request: ManagedSqliteAuthorizerRequest<'_>,
    ) -> Result<ManagedSqliteAuthorizerDecision, ManagedSqliteRegistryRouteRejection> {
        Ok(self.exact_entry(handle)?.policy.authorize_sql(request))
    }

    pub(super) fn enter_schema_migration(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .policy
            .enter_schema_migration()
            .map_err(ManagedSqliteRegistryRouteRejection::Authorizer)
    }

    pub(super) fn enter_runtime(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .policy
            .enter_runtime()
            .map_err(ManagedSqliteRegistryRouteRejection::Authorizer)
    }

    pub(super) fn begin_open_attempt(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .state
            .begin_open_attempt()
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }

    pub(super) fn begin_callback(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
        kind: ManagedSqliteRegistryCallbackKind,
    ) -> Result<ManagedSqliteRegistryCallbackLease, ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .state
            .begin_callback(kind)
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }

    pub(super) fn finish_callback(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
        lease: &ManagedSqliteRegistryCallbackLease,
    ) -> Result<(), ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .state
            .finish_callback(lease)
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }

    pub(super) fn begin_connection_close(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryRouteRejection> {
        self.exact_entry_mut(handle)?
            .state
            .begin_connection_close()
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }

    pub(super) fn retire_pending(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryRetirementReceipt, ManagedSqliteRegistryRouteRejection> {
        self.exact_entry(handle)?;
        let mut entry = self
            .routes
            .remove(&handle.token)
            .expect("validated route must remain present under exclusive owner access");
        let proof = ManagedSqliteRegistryRouteRemovalProof::from_removed_route(
            entry.state.session_id(),
            entry.state.route_epoch(),
        );
        match entry.state.cancel_pending_after_route_removed(proof) {
            Ok(receipt) => Ok(receipt),
            Err(rejection) => {
                entry
                    .state
                    .quarantine(ManagedSqliteRegistryTerminalReason::FailureCustodyRetained);
                Self::retain_terminal(entry);
                Err(ManagedSqliteRegistryRouteRejection::State(rejection))
            }
        }
    }

    pub(super) fn quarantine(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
        reason: ManagedSqliteRegistryTerminalReason,
    ) -> Result<(), ManagedSqliteRegistryRouteRejection> {
        self.exact_entry(handle)?;
        let mut entry = self
            .routes
            .remove(&handle.token)
            .expect("validated route must remain present under exclusive owner access");
        entry.state.quarantine(reason);
        Self::retain_terminal(entry);
        Ok(())
    }

    fn exact_entry(
        &self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<&ManagedSqliteRegistryEntry<Custody>, ManagedSqliteRegistryRouteRejection> {
        let Some(entry) = self.routes.get(&handle.token) else {
            return Err(ManagedSqliteRegistryRouteRejection::UnknownOrRetired);
        };
        if entry.state.session_id() != handle.session_id
            || entry.state.route_epoch() != handle.route_epoch
        {
            return Err(ManagedSqliteRegistryRouteRejection::IdentityMismatch);
        }
        Ok(entry)
    }

    fn exact_entry_mut(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<&mut ManagedSqliteRegistryEntry<Custody>, ManagedSqliteRegistryRouteRejection> {
        let Some(entry) = self.routes.get_mut(&handle.token) else {
            return Err(ManagedSqliteRegistryRouteRejection::UnknownOrRetired);
        };
        if entry.state.session_id() != handle.session_id
            || entry.state.route_epoch() != handle.route_epoch
        {
            return Err(ManagedSqliteRegistryRouteRejection::IdentityMismatch);
        }
        Ok(entry)
    }

    pub(super) fn retain_terminal_if_present(&mut self, handle: ManagedSqliteRegistryRouteHandle) {
        if self.phase(handle) == Ok(ManagedSqliteRegistrySessionPhase::TerminalQuarantine) {
            let _ = self.quarantine(
                handle,
                ManagedSqliteRegistryTerminalReason::StateInvariantViolated,
            );
        }
    }

    fn exhausted(&self, custody: Custody) -> ManagedSqliteRegistryRegistrationFailure<Custody> {
        ManagedSqliteRegistryRegistrationFailure {
            reason: ManagedSqliteRegistryRegistrationRejection::IdentityExhausted,
            custody,
        }
    }

    fn retain_terminal(entry: ManagedSqliteRegistryEntry<Custody>) {
        let _permanent_process_custody = Box::leak(Box::new(entry));
    }
}

#[cfg(test)]
mod tests;
