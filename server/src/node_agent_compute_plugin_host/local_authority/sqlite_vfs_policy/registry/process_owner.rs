use std::{
    fmt,
    sync::{Mutex, MutexGuard},
};

use ring::rand::{SecureRandom, SystemRandom};

#[cfg(test)]
use super::state::{
    ManagedSqliteRegistrySessionTestSnapshot, ManagedSqliteRegistryTerminalRouteTestSnapshot,
};
#[cfg(test)]
use super::types::{
    ManagedSqliteRegistryCallbackCompletionReceipt, ManagedSqliteRegistryConnectionClosedReceipt,
};
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
#[cfg(all(test, windows))]
use crate::node_agent_managed_fs::{
    ManagedSqliteWalMainCloseFailure, ManagedSqliteWalMainCloseFailureTestSnapshot,
};
use crate::{
    node_agent_compute_plugin_host::local_authority::{
        sqlite_vfs_policy::{
            ManagedSqliteAuthorizerDecision, ManagedSqliteAuthorizerRequest,
            ManagedSqliteLogicalFileRole,
        },
        ComputePluginHandleBoundAuthorityOpenIntent,
    },
    node_agent_managed_fs::{
        ManagedSqliteFileCloseReceipt, ManagedSqliteMainFileCloseReceipt,
        ManagedSqliteWalMainCloseReceipt,
    },
};

#[cfg(all(test, windows))]
mod joint_close_direct_xclose;
#[cfg(all(test, windows))]
mod joint_close_fault;
mod lifecycle;
#[cfg(all(test, windows))]
mod test_lock_callback_admission;
#[cfg(all(test, windows))]
mod test_registry_lifecycle;
mod vfs;

#[cfg(test)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) use lifecycle::ManagedSqliteRegistryTerminalCustodyTestSnapshot;

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
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) enum ManagedSqliteRegistryProcessRegistrationRejection
{
    EntropyUnavailable,
    CollisionBudgetExhausted,
    OwnerPoisoned,
    Registry(ManagedSqliteRegistryRegistrationRejection),
}

#[must_use = "registration failure retains the authority-open custody"]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteRegistryProcessRegistrationFailure<
    Custody,
> {
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
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) enum ManagedSqliteRegistryProcessRouteRejection
{
    OwnerPoisoned,
    Route(ManagedSqliteRegistryRouteRejection),
    #[cfg(all(test, windows))]
    RegistryWalMainNativeUncertain,
    #[cfg(all(test, windows))]
    CloseCallbackAdmissionRejected,
    #[cfg(all(test, windows))]
    BeginConnectionCloseRejected,
    #[cfg(all(test, windows))]
    JointClosePhysicalFailureEvidenceUnavailable,
}

/// A process-lifetime route table. Construction deliberately leaks the wrapper so a poisoned
/// mutex or abandoned callback can never release live authority custody through `Drop`.
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteRegistryProcessOwner<
    Custody,
    NonceSource,
> {
    routes: Mutex<ManagedSqliteRegistryOwner<Custody>>,
    nonce_source: NonceSource,
    #[cfg(test)]
    terminal_custody_test_ledger: lifecycle::ManagedSqliteRegistryTerminalCustodyTestLedger,
    #[cfg(all(test, windows))]
    joint_close_registry_native_fault:
        joint_close_fault::ManagedSqliteRegistryWalMainNativeUncertainTestGate,
    #[cfg(all(test, windows))]
    joint_close_callback_admission_fault:
        joint_close_fault::ManagedSqliteRegistryCloseCallbackAdmissionTestGate,
    #[cfg(all(test, windows))]
    joint_close_begin_connection_close_fault:
        joint_close_fault::ManagedSqliteRegistryBeginConnectionCloseTestGate,
}

impl<Custody, NonceSource> ManagedSqliteRegistryProcessOwner<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn leak(
        nonce_source: NonceSource,
    ) -> &'static Self {
        Box::leak(Box::new(Self {
            routes: Mutex::new(ManagedSqliteRegistryOwner::new()),
            nonce_source,
            #[cfg(test)]
            terminal_custody_test_ledger:
                lifecycle::ManagedSqliteRegistryTerminalCustodyTestLedger::new(),
            #[cfg(all(test, windows))]
            joint_close_registry_native_fault:
                joint_close_fault::ManagedSqliteRegistryWalMainNativeUncertainTestGate::new(),
            #[cfg(all(test, windows))]
            joint_close_callback_admission_fault:
                joint_close_fault::ManagedSqliteRegistryCloseCallbackAdmissionTestGate::new(),
            #[cfg(all(test, windows))]
            joint_close_begin_connection_close_fault:
                joint_close_fault::ManagedSqliteRegistryBeginConnectionCloseTestGate::new(),
        }))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn register(
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

    #[cfg(test)]
    pub(super) fn registration_shutdown_test_snapshot(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistrySessionTestSnapshot, ManagedSqliteRegistryProcessRouteRejection>
    {
        self.lock_routes()?
            .registration_shutdown_test_snapshot(route)
            .map_err(ManagedSqliteRegistryProcessRouteRejection::Route)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn authorize_sql(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        request: ManagedSqliteAuthorizerRequest<'_>,
    ) -> Result<ManagedSqliteAuthorizerDecision, ManagedSqliteRegistryProcessRouteRejection> {
        self.lock_routes()?
            .authorize_sql(route, request)
            .map_err(ManagedSqliteRegistryProcessRouteRejection::Route)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn enter_schema_migration(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.enter_schema_migration(route))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn enter_runtime(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.enter_runtime(route))
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
        #[cfg(all(test, windows))]
        if self.claim_begin_connection_close_rejection(route)? {
            return Err(ManagedSqliteRegistryProcessRouteRejection::BeginConnectionCloseRejected);
        }
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
        #[cfg(all(test, windows))]
        if kind == ManagedSqliteRegistryCallbackKind::Close
            && self.claim_close_callback_admission_rejection(route)?
        {
            return Err(ManagedSqliteRegistryProcessRouteRejection::CloseCallbackAdmissionRejected);
        }
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

    #[cfg(test)]
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn retire_pending_for_test(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let _receipt = self.retire_pending(route)?;
        Ok(())
    }

    fn finish_callback(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        lease: ManagedSqliteRegistryCallbackLease,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route_retaining_failure(
            route,
            lease,
            #[cfg(test)]
            lifecycle::ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CallbackLease,
            |routes, lease| routes.finish_callback(route, lease),
        )
    }

    #[cfg(test)]
    fn arm_shm_callback_completion_native_rejection(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        lease: &ManagedSqliteRegistryCallbackLease,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| {
            routes.arm_shm_callback_completion_native_rejection(route, lease)
        })
    }

    #[cfg(test)]
    fn finish_callback_with_receipt(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        lease: ManagedSqliteRegistryCallbackLease,
    ) -> Result<
        ManagedSqliteRegistryCallbackCompletionReceipt,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        self.apply_route_retaining_failure(
            route,
            lease,
            lifecycle::ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CallbackLease,
            |routes, lease| routes.finish_callback_with_receipt(route, lease),
        )
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
        #[cfg(test)]
        retained_kind: lifecycle::ManagedSqliteRegistryTerminalCustodyTestRetentionKind,
        operation: impl FnOnce(
            &mut ManagedSqliteRegistryOwner<Custody>,
            &Evidence,
        ) -> Result<T, ManagedSqliteRegistryRouteRejection>,
    ) -> Result<T, ManagedSqliteRegistryProcessRouteRejection> {
        let mut routes = match self.lock_routes() {
            Ok(routes) => routes,
            Err(rejection) => {
                #[cfg(test)]
                let _ = self.record_terminal_custody_test_event(
                    route,
                    lifecycle::ManagedSqliteRegistryTerminalCustodyTestEventKind::Retention {
                        kind: retained_kind,
                        explicit_failure_custody_retained: false,
                        terminal_route: None,
                    },
                );
                let _permanent_evidence = Box::leak(Box::new(evidence));
                return Err(rejection);
            }
        };
        #[cfg(test)]
        let exact_route_was_active = routes.phase(route).is_ok();
        match operation(&mut routes, &evidence) {
            Ok(value) => Ok(value),
            Err(rejection) => {
                #[cfg(test)]
                let explicit_failure_custody_retained = matches!(
                    routes.terminal_reason(route),
                    Ok(Some(
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained
                    ))
                );
                #[cfg(test)]
                let terminal_route = routes.terminal_route_test_snapshot(route).ok();
                #[cfg(test)]
                let _ = self.record_terminal_custody_test_event(
                    route,
                    lifecycle::ManagedSqliteRegistryTerminalCustodyTestEventKind::Retention {
                        kind: retained_kind,
                        explicit_failure_custody_retained,
                        terminal_route,
                    },
                );
                routes.retain_terminal_if_present(route);
                #[cfg(test)]
                if exact_route_was_active
                    && matches!(
                        routes.phase(route),
                        Err(ManagedSqliteRegistryRouteRejection::UnknownOrRetired)
                    )
                {
                    let _ = self.record_terminal_custody_test_event(
                        route,
                        lifecycle::ManagedSqliteRegistryTerminalCustodyTestEventKind::RouteRemoved,
                    );
                }
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

#[must_use = "the routed callback lease must be explicitly completed; drop quarantines its route"]
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
    #[cfg(test)]
    pub(super) fn arm_shm_callback_completion_native_rejection(
        &mut self,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let lease = self
            .lease
            .as_ref()
            .expect("live routed callback lease must contain state custody");
        self.owner
            .arm_shm_callback_completion_native_rejection(self.route, lease)
    }

    pub(super) fn complete(mut self) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let lease = self
            .lease
            .take()
            .expect("live routed callback lease must contain state custody");
        self.owner.finish_callback(self.route, lease)
    }

    #[cfg(test)]
    pub(super) fn complete_with_receipt(
        mut self,
    ) -> Result<
        ManagedSqliteRegistryCallbackCompletionReceipt,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        let lease = self
            .lease
            .take()
            .expect("live routed callback lease must contain state custody");
        self.owner.finish_callback_with_receipt(self.route, lease)
    }
}

impl<Custody, NonceSource> Drop for ManagedSqliteRegistryRoutedCallbackLease<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            // Stack unwinding or an abandoned callback is not normal completion evidence. Keep
            // the exact lease forever and quarantine its route before any custody can be reused.
            let _ = self.owner.retain_terminal_custody_with_test_kind(
                self.route,
                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                lease,
                #[cfg(test)]
                lifecycle::ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CallbackLease,
            );
        }
    }
}

#[cfg(test)]
mod tests;
