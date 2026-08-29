//! Typed, exact-route native rejection seams used only by lifecycle evidence tests.

use super::*;

impl<Custody, NonceSource> ManagedSqliteRegistryProcessOwner<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(in super::super) fn claim_connection_observation_sidecar(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        role: ManagedSqliteLogicalFileRole,
    ) -> Result<ManagedSqliteRegistryFileLease, ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| {
            routes.claim_connection_observation_sidecar(route, role)
        })
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn observe_connection_closed_after_callback_with_sidecar(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        callback: ManagedSqliteRegistryCallbackCompletionReceipt,
        sidecar: super::super::file_custody::ManagedSqliteRegistryPinnedFile<Custody, NonceSource>,
    ) -> Result<
        (
            ManagedSqliteRegistryConnectionClosedReceipt,
            super::super::file_custody::ManagedSqliteRegistryPinnedFile<Custody, NonceSource>,
        ),
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        let evidence = (callback, sidecar);
        let mut routes = match self.lock_routes() {
            Ok(routes) => routes,
            Err(rejection) => {
                let _ = self.record_terminal_custody_test_event(
                    route,
                    super::lifecycle::ManagedSqliteRegistryTerminalCustodyTestEventKind::Retention {
                        kind: super::lifecycle::ManagedSqliteRegistryTerminalCustodyTestRetentionKind::OtherTerminalCustody,
                        explicit_failure_custody_retained: false,
                        terminal_route: None,
                    },
                );
                let _permanent_evidence = Box::leak(Box::new(evidence));
                return Err(rejection);
            }
        };
        let exact_route_was_active = routes.phase(route).is_ok();
        match routes.observe_connection_closed_after_callback(route, &evidence.0) {
            Ok(observed) => Ok((observed, evidence.1)),
            Err(rejection) => {
                let explicit_failure_custody_retained = matches!(
                    routes.terminal_reason(route),
                    Ok(Some(
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained
                    ))
                );
                let terminal_route = routes.terminal_route_test_snapshot(route).ok();
                let _ = self.record_terminal_custody_test_event(
                    route,
                    super::lifecycle::ManagedSqliteRegistryTerminalCustodyTestEventKind::Retention {
                        kind: super::lifecycle::ManagedSqliteRegistryTerminalCustodyTestRetentionKind::OtherTerminalCustody,
                        explicit_failure_custody_retained,
                        terminal_route,
                    },
                );
                routes.retain_terminal_if_present(route);
                if exact_route_was_active
                    && matches!(
                        routes.phase(route),
                        Err(ManagedSqliteRegistryRouteRejection::UnknownOrRetired)
                    )
                {
                    let _ = self.record_terminal_custody_test_event(
                        route,
                        super::lifecycle::ManagedSqliteRegistryTerminalCustodyTestEventKind::RouteRemoved,
                    );
                }
                let _permanent_evidence = Box::leak(Box::new(evidence));
                Err(ManagedSqliteRegistryProcessRouteRejection::Route(rejection))
            }
        }
    }

    pub(super) fn arm_close_callback_completion_native_rejection(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        lease: &ManagedSqliteRegistryCallbackLease,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| {
            routes.arm_close_callback_completion_native_rejection(route, lease)
        })
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn arm_route_retirement_native_rejection(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        receipt: &ManagedSqliteRegistryConnectionClosedReceipt,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| {
            routes.arm_route_retirement_native_rejection(route, receipt)
        })
    }
}

impl<Custody, NonceSource> ManagedSqliteRegistryRoutedCallbackLease<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn arm_close_callback_completion_native_rejection(
        &mut self,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let lease = self
            .lease
            .as_ref()
            .expect("live routed callback lease must contain state custody");
        self.owner
            .arm_close_callback_completion_native_rejection(self.route, lease)
    }
}
