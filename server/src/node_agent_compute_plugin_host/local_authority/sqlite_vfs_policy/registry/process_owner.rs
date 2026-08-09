use std::{
    fmt,
    sync::{Mutex, MutexGuard},
};

use ring::rand::{SecureRandom, SystemRandom};

use super::{
    owner::{
        ManagedSqliteRegistryCustody, ManagedSqliteRegistryOwner,
        ManagedSqliteRegistryRegistrationRejection, ManagedSqliteRegistryRouteHandle,
        ManagedSqliteRegistryRouteRejection,
    },
    types::{
        ManagedSqliteRegistryCallbackKind, ManagedSqliteRegistryCallbackLease,
        ManagedSqliteRegistryCloseOutcome, ManagedSqliteRegistryFileLease,
        ManagedSqliteRegistryRetirementReceipt, ManagedSqliteRegistrySessionPhase,
        ManagedSqliteRegistryShmLease, ManagedSqliteRegistryTerminalReason,
        ManagedSqliteRegistryWalMainCloseProofs,
    },
};
use crate::{
    node_agent_compute_plugin_host::local_authority::{
        sqlite_vfs_policy::ManagedSqliteLogicalFileRole,
        ComputePluginHandleBoundAuthorityOpenIntent,
    },
    node_agent_managed_fs::{
        ManagedSqliteFileCloseReceipt, ManagedSqliteMainFileCloseReceipt,
        ManagedSqliteWalMainCloseReceipt,
    },
};

mod lifecycle;

const ROUTE_NONCE_ATTEMPTS: usize = 8;

pub(super) type ComputePluginHandleBoundSqliteProcessOwner = ManagedSqliteRegistryProcessOwner<
    ComputePluginHandleBoundAuthorityOpenIntent,
    ManagedSqliteRegistrySystemNonceSource,
>;

pub(in crate::node_agent_compute_plugin_host::local_authority) trait ManagedSqliteRegistryNonceSource
{
    fn fill_nonce(&self, output: &mut [u8; 16]) -> Result<(), ()>;
}

pub(in crate::node_agent_compute_plugin_host::local_authority) struct ManagedSqliteRegistrySystemNonceSource;

impl ManagedSqliteRegistryNonceSource for ManagedSqliteRegistrySystemNonceSource {
    fn fill_nonce(&self, output: &mut [u8; 16]) -> Result<(), ()> {
        SystemRandom::new().fill(output).map_err(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedSqliteRegistryProcessRegistrationRejection {
    EntropyUnavailable,
    CollisionBudgetExhausted,
    OwnerPoisoned,
    Registry(ManagedSqliteRegistryRegistrationRejection),
}

#[must_use = "registration failure retains the authority-open custody"]
pub(super) struct ManagedSqliteRegistryProcessRegistrationFailure<Custody> {
    reason: ManagedSqliteRegistryProcessRegistrationRejection,
    custody: Custody,
}

impl<Custody> ManagedSqliteRegistryProcessRegistrationFailure<Custody> {
    pub(super) fn into_parts(self) -> (ManagedSqliteRegistryProcessRegistrationRejection, Custody) {
        (self.reason, self.custody)
    }
}

impl<Custody> fmt::Debug for ManagedSqliteRegistryProcessRegistrationFailure<Custody> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedSqliteRegistryProcessRegistrationFailure")
            .field("reason", &self.reason)
            .field("custody", &"<retained>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedSqliteRegistryProcessRouteRejection {
    OwnerPoisoned,
    Route(ManagedSqliteRegistryRouteRejection),
}

/// A process-lifetime route table. Construction deliberately leaks the wrapper so a poisoned
/// mutex or abandoned callback can never release live authority custody through `Drop`.
pub(super) struct ManagedSqliteRegistryProcessOwner<Custody, NonceSource> {
    routes: Mutex<ManagedSqliteRegistryOwner<Custody>>,
    nonce_source: NonceSource,
}

impl<Custody, NonceSource> ManagedSqliteRegistryProcessOwner<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(super) fn leak(nonce_source: NonceSource) -> &'static Self {
        Box::leak(Box::new(Self {
            routes: Mutex::new(ManagedSqliteRegistryOwner::new()),
            nonce_source,
        }))
    }

    pub(super) fn register(
        &'static self,
        mut custody: Custody,
    ) -> Result<
        ManagedSqliteRegistryRouteHandle,
        ManagedSqliteRegistryProcessRegistrationFailure<Custody>,
    > {
        for _ in 0..ROUTE_NONCE_ATTEMPTS {
            let mut nonce = [0_u8; 16];
            if self.nonce_source.fill_nonce(&mut nonce).is_err() {
                return Err(Self::registration_failure(
                    ManagedSqliteRegistryProcessRegistrationRejection::EntropyUnavailable,
                    custody,
                ));
            }
            let mut routes = match self.lock_routes() {
                Ok(routes) => routes,
                Err(_) => {
                    return Err(Self::registration_failure(
                        ManagedSqliteRegistryProcessRegistrationRejection::OwnerPoisoned,
                        custody,
                    ));
                }
            };
            match routes.register(nonce, custody) {
                Ok(route) => return Ok(route),
                Err(failure) => {
                    let (reason, returned_custody) = failure.into_parts();
                    custody = returned_custody;
                    if !matches!(
                        reason,
                        ManagedSqliteRegistryRegistrationRejection::InvalidNonce(_)
                            | ManagedSqliteRegistryRegistrationRejection::TokenAlreadyUsed
                    ) {
                        return Err(Self::registration_failure(
                            ManagedSqliteRegistryProcessRegistrationRejection::Registry(reason),
                            custody,
                        ));
                    }
                }
            }
        }
        Err(Self::registration_failure(
            ManagedSqliteRegistryProcessRegistrationRejection::CollisionBudgetExhausted,
            custody,
        ))
    }

    pub(super) fn phase(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistrySessionPhase, ManagedSqliteRegistryProcessRouteRejection> {
        self.lock_routes()?
            .phase(route)
            .map_err(ManagedSqliteRegistryProcessRouteRejection::Route)
    }

    pub(super) fn begin_open_attempt(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.begin_open_attempt(route))
    }

    pub(super) fn begin_connection_close(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.begin_connection_close(route))
    }

    pub(super) fn begin_callback(
        &'static self,
        route: ManagedSqliteRegistryRouteHandle,
        kind: ManagedSqliteRegistryCallbackKind,
    ) -> Result<
        ManagedSqliteRegistryRoutedCallbackLease<Custody, NonceSource>,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        let lease = self.apply_route(route, |routes| routes.begin_callback(route, kind))?;
        Ok(ManagedSqliteRegistryRoutedCallbackLease {
            owner: self,
            route,
            lease: Some(lease),
        })
    }

    pub(super) fn retire_pending(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryRetirementReceipt, ManagedSqliteRegistryProcessRouteRejection>
    {
        self.apply_route(route, |routes| routes.retire_pending(route))
    }

    fn finish_callback(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        lease: ManagedSqliteRegistryCallbackLease,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route_retaining_failure(route, lease, |routes, lease| {
            routes.finish_callback(route, lease)
        })
    }

    fn apply_route<T>(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        operation: impl FnOnce(
            &mut ManagedSqliteRegistryOwner<Custody>,
        ) -> Result<T, ManagedSqliteRegistryRouteRejection>,
    ) -> Result<T, ManagedSqliteRegistryProcessRouteRejection> {
        let mut routes = self.lock_routes()?;
        match operation(&mut routes) {
            Ok(value) => Ok(value),
            Err(rejection) => {
                routes.retain_terminal_if_present(route);
                Err(ManagedSqliteRegistryProcessRouteRejection::Route(rejection))
            }
        }
    }

    /// Linear completion evidence cannot be reconstructed after its one-shot operation. Keep the
    /// exact value alive forever whenever route locking or the state transition fails; neither
    /// xClose nor callback completion has a safe retry channel.
    fn apply_route_retaining_failure<T, Evidence: 'static>(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        evidence: Evidence,
        operation: impl FnOnce(
            &mut ManagedSqliteRegistryOwner<Custody>,
            &Evidence,
        ) -> Result<T, ManagedSqliteRegistryRouteRejection>,
    ) -> Result<T, ManagedSqliteRegistryProcessRouteRejection> {
        let mut routes = match self.lock_routes() {
            Ok(routes) => routes,
            Err(rejection) => {
                let _permanent_evidence = Box::leak(Box::new(evidence));
                return Err(rejection);
            }
        };
        match operation(&mut routes, &evidence) {
            Ok(value) => Ok(value),
            Err(rejection) => {
                routes.retain_terminal_if_present(route);
                let _permanent_evidence = Box::leak(Box::new(evidence));
                Err(ManagedSqliteRegistryProcessRouteRejection::Route(rejection))
            }
        }
    }

    fn lock_routes(
        &self,
    ) -> Result<
        MutexGuard<'_, ManagedSqliteRegistryOwner<Custody>>,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        self.routes
            .lock()
            .map_err(|_| ManagedSqliteRegistryProcessRouteRejection::OwnerPoisoned)
    }

    fn registration_failure(
        reason: ManagedSqliteRegistryProcessRegistrationRejection,
        custody: Custody,
    ) -> ManagedSqliteRegistryProcessRegistrationFailure<Custody> {
        ManagedSqliteRegistryProcessRegistrationFailure { reason, custody }
    }
}

impl ComputePluginHandleBoundSqliteProcessOwner {
    pub(super) fn leak_with_system_nonce_source() -> &'static Self {
        Self::leak(ManagedSqliteRegistrySystemNonceSource)
    }
}

#[must_use = "the routed callback lease must be completed or dropped"]
pub(super) struct ManagedSqliteRegistryRoutedCallbackLease<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    owner: &'static ManagedSqliteRegistryProcessOwner<Custody, NonceSource>,
    route: ManagedSqliteRegistryRouteHandle,
    lease: Option<ManagedSqliteRegistryCallbackLease>,
}

impl<Custody, NonceSource> ManagedSqliteRegistryRoutedCallbackLease<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(super) fn complete(mut self) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let lease = self
            .lease
            .take()
            .expect("live routed callback lease must contain state custody");
        self.owner.finish_callback(self.route, lease)
    }
}

impl<Custody, NonceSource> Drop for ManagedSqliteRegistryRoutedCallbackLease<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            let _ = self.owner.finish_callback(self.route, lease);
        }
    }
}

#[cfg(test)]
mod tests;
