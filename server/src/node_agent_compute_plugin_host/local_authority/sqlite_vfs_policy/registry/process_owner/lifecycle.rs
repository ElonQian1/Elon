use super::*;

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedSqliteRegistryTerminalCustodyTestRetentionKind {
    CallbackLease,
    CompletionEvidence,
    WalMainPhysicalCustody,
    OtherTerminalCustody,
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(super) enum ManagedSqliteRegistryTerminalCustodyTestEventKind {
    Retention {
        kind: ManagedSqliteRegistryTerminalCustodyTestRetentionKind,
        explicit_failure_custody_retained: bool,
        terminal_route: Option<ManagedSqliteRegistryTerminalRouteTestSnapshot>,
    },
    RouteRemoved,
    PhysicalSuccessHandoff {
        route_closing: bool,
        main_lease: bool,
        shm_lease: bool,
        callbacks_in_flight: u32,
    },
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(super) struct ManagedSqliteRegistryTerminalCustodyTestEvent {
    route: ManagedSqliteRegistryRouteHandle,
    kind: ManagedSqliteRegistryTerminalCustodyTestEventKind,
}

#[cfg(test)]
pub(super) struct ManagedSqliteRegistryTerminalCustodyTestLedger(
    Mutex<Vec<ManagedSqliteRegistryTerminalCustodyTestEvent>>,
);

#[cfg(test)]
impl ManagedSqliteRegistryTerminalCustodyTestLedger {
    pub(super) fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }
}

/// A test-only, redacted projection of terminal retention for one exact private route.
/// It deliberately contains no route identity, pointer, path, name, receipt, or custody.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteRegistryTerminalCustodyTestSnapshot
{
    callback_lease_retentions: usize,
    completion_evidence_retentions: usize,
    wal_main_physical_custody_retentions: usize,
    other_terminal_custody_retentions: usize,
    explicit_failure_custody_retained_retentions: usize,
    terminal_route_observations: usize,
    terminal_route: Option<ManagedSqliteRegistryTerminalRouteTestSnapshot>,
    route_removals: usize,
    active_route_present: bool,
    physical_success_handoff_retentions: usize,
    physical_success_route_closing: bool,
    physical_success_main_lease: bool,
    physical_success_shm_lease: bool,
    physical_success_callbacks_in_flight: u32,
}

#[cfg(test)]
impl ManagedSqliteRegistryTerminalCustodyTestSnapshot {
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn retention_count(
        self,
    ) -> usize {
        self.callback_lease_retentions
            + self.completion_evidence_retentions
            + self.wal_main_physical_custody_retentions
            + self.other_terminal_custody_retentions
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn callback_lease_retention_count(
        self,
    ) -> usize {
        self.callback_lease_retentions
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn completion_evidence_retention_count(
        self,
    ) -> usize {
        self.completion_evidence_retentions
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn other_terminal_custody_retention_count(
        self,
    ) -> usize {
        self.other_terminal_custody_retentions
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn wal_main_physical_custody_retention_count(
        self,
    ) -> usize {
        self.wal_main_physical_custody_retentions
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn explicit_failure_custody_retained_count(
        self,
    ) -> usize {
        self.explicit_failure_custody_retained_retentions
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn route_removal_count(
        self,
    ) -> usize {
        self.route_removals
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn terminal_route_observation_count(
        self,
    ) -> usize {
        self.terminal_route_observations
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn terminal_route(
        self,
    ) -> Option<ManagedSqliteRegistryTerminalRouteTestSnapshot> {
        self.terminal_route
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn active_route_present(
        self,
    ) -> bool {
        self.active_route_present
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn physical_success_handoff_retention_count(
        self,
    ) -> usize {
        self.physical_success_handoff_retentions
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn physical_success_handoff_shape(
        self,
    ) -> (bool, bool, bool, u32) {
        (
            self.physical_success_route_closing,
            self.physical_success_main_lease,
            self.physical_success_shm_lease,
            self.physical_success_callbacks_in_flight,
        )
    }
}

impl<Custody, NonceSource> ManagedSqliteRegistryProcessOwner<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    /// Permanently retains physical and registry custody before removing the exact route into
    /// terminal quarantine. Retention happens first so owner poisoning or a stale route can never
    /// make an uncertain handle fall through ordinary Rust destruction.
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn retain_terminal_custody<
        Retained: 'static,
    >(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        reason: ManagedSqliteRegistryTerminalReason,
        custody: Retained,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        #[cfg(test)]
        let retained_kind = Self::explicit_terminal_custody_test_kind::<Retained>();
        self.retain_terminal_custody_with_test_kind(
            route,
            reason,
            custody,
            #[cfg(test)]
            retained_kind,
        )
    }

    pub(super) fn retain_terminal_custody_with_test_kind<Retained: 'static>(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        reason: ManagedSqliteRegistryTerminalReason,
        custody: Retained,
        #[cfg(test)] retained_kind: ManagedSqliteRegistryTerminalCustodyTestRetentionKind,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        #[cfg(test)]
        {
            let retained_event =
                |terminal_route| ManagedSqliteRegistryTerminalCustodyTestEventKind::Retention {
                    kind: retained_kind,
                    explicit_failure_custody_retained: matches!(
                        reason,
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained
                    ),
                    terminal_route,
                };
            let mut custody = Some(custody);
            let mut routes = match self.lock_routes() {
                Ok(routes) => routes,
                Err(rejection) => {
                    let _ = self.record_terminal_custody_test_event(route, retained_event(None));
                    let _permanent_physical_custody =
                        Box::leak(Box::new(custody.take().expect("terminal custody")));
                    return Err(rejection);
                }
            };
            let terminal_route = match routes.prepare_terminal_route_test_snapshot(route, reason) {
                Ok(terminal_route) => terminal_route,
                Err(rejection) => {
                    let _ = self.record_terminal_custody_test_event(route, retained_event(None));
                    let _permanent_physical_custody =
                        Box::leak(Box::new(custody.take().expect("terminal custody")));
                    return Err(ManagedSqliteRegistryProcessRouteRejection::Route(rejection));
                }
            };
            if let Err(rejection) =
                self.record_terminal_custody_test_event(route, retained_event(Some(terminal_route)))
            {
                let _permanent_physical_custody =
                    Box::leak(Box::new(custody.take().expect("terminal custody")));
                return Err(rejection);
            }
            let _permanent_physical_custody =
                Box::leak(Box::new(custody.take().expect("terminal custody")));
            let quarantine = routes
                .quarantine(route, reason)
                .map_err(ManagedSqliteRegistryProcessRouteRejection::Route);
            drop(routes);
            if quarantine.is_err() {
                return quarantine;
            }
            let removal_observation = self.record_terminal_custody_test_event(
                route,
                ManagedSqliteRegistryTerminalCustodyTestEventKind::RouteRemoved,
            );
            removal_observation?;
            return Ok(());
        }

        #[cfg(not(test))]
        {
            let _permanent_physical_custody = Box::leak(Box::new(custody));
            self.apply_route(route, |routes| routes.quarantine(route, reason))
        }
    }

    #[cfg(test)]
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn retain_terminal_wal_main_physical_custody<
        Retained: 'static,
    >(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        reason: ManagedSqliteRegistryTerminalReason,
        custody: Retained,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.retain_terminal_custody_with_test_kind(
            route,
            reason,
            custody,
            ManagedSqliteRegistryTerminalCustodyTestRetentionKind::WalMainPhysicalCustody,
        )
    }

    /// Retains the exact post-physical-close custody without quarantining or retiring the route.
    /// This boundary is test-only: it models loss of control immediately after the real WAL-main
    /// close receipt while the close callback and registry leases remain live.
    #[cfg(all(test, windows))]
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn retain_physical_success_handoff<
        Retained: 'static,
    >(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        custody: Retained,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let mut custody = Some(custody);
        let snapshot = match self.registration_shutdown_test_snapshot(route) {
            Ok(snapshot) => snapshot,
            Err(rejection) => {
                let _permanent_physical_success_handoff =
                    Box::leak(Box::new(custody.take().expect("physical-success custody")));
                return Err(rejection);
            }
        };
        let event = ManagedSqliteRegistryTerminalCustodyTestEventKind::PhysicalSuccessHandoff {
            route_closing: snapshot.phase() == ManagedSqliteRegistrySessionPhase::Closing,
            main_lease: snapshot.main_file_lock_owner_lease(),
            shm_lease: snapshot.shm_lease(),
            callbacks_in_flight: snapshot.callbacks_in_flight(),
        };
        if let Err(rejection) = self.record_terminal_custody_test_event(route, event) {
            let _permanent_physical_success_handoff =
                Box::leak(Box::new(custody.take().expect("physical-success custody")));
            return Err(rejection);
        }
        let _permanent_physical_success_handoff =
            Box::leak(Box::new(custody.take().expect("physical-success custody")));
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn record_terminal_custody_test_event(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        kind: ManagedSqliteRegistryTerminalCustodyTestEventKind,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.terminal_custody_test_ledger
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(ManagedSqliteRegistryTerminalCustodyTestEvent { route, kind });
        Ok(())
    }

    #[cfg(test)]
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn terminal_custody_test_snapshot(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<
        ManagedSqliteRegistryTerminalCustodyTestSnapshot,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        let active_route_present = {
            let routes = self.lock_routes()?;
            match routes.phase(route) {
                Ok(_) => true,
                Err(ManagedSqliteRegistryRouteRejection::UnknownOrRetired) => false,
                Err(rejection) => {
                    return Err(ManagedSqliteRegistryProcessRouteRejection::Route(rejection));
                }
            }
        };
        let events = self
            .terminal_custody_test_ledger
            .0
            .lock()
            .map_err(|_| ManagedSqliteRegistryProcessRouteRejection::OwnerPoisoned)?;
        let mut snapshot = ManagedSqliteRegistryTerminalCustodyTestSnapshot {
            callback_lease_retentions: 0,
            completion_evidence_retentions: 0,
            wal_main_physical_custody_retentions: 0,
            other_terminal_custody_retentions: 0,
            explicit_failure_custody_retained_retentions: 0,
            terminal_route_observations: 0,
            terminal_route: None,
            route_removals: 0,
            active_route_present,
            physical_success_handoff_retentions: 0,
            physical_success_route_closing: false,
            physical_success_main_lease: false,
            physical_success_shm_lease: false,
            physical_success_callbacks_in_flight: 0,
        };
        for event in events.iter().filter(|event| event.route == route) {
            match event.kind {
                ManagedSqliteRegistryTerminalCustodyTestEventKind::Retention {
                    kind,
                    explicit_failure_custody_retained,
                    terminal_route,
                } => {
                    match kind {
                        ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CallbackLease => {
                            snapshot.callback_lease_retentions += 1;
                        }
                        ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CompletionEvidence => {
                            snapshot.completion_evidence_retentions += 1;
                        }
                        ManagedSqliteRegistryTerminalCustodyTestRetentionKind::WalMainPhysicalCustody => {
                            snapshot.wal_main_physical_custody_retentions += 1;
                        }
                        ManagedSqliteRegistryTerminalCustodyTestRetentionKind::OtherTerminalCustody => {
                            snapshot.other_terminal_custody_retentions += 1;
                        }
                    }
                    if explicit_failure_custody_retained {
                        snapshot.explicit_failure_custody_retained_retentions += 1;
                    }
                    if let Some(terminal_route) = terminal_route {
                        snapshot.terminal_route_observations += 1;
                        snapshot.terminal_route = Some(terminal_route);
                    }
                }
                ManagedSqliteRegistryTerminalCustodyTestEventKind::RouteRemoved => {
                    snapshot.route_removals += 1;
                }
                ManagedSqliteRegistryTerminalCustodyTestEventKind::PhysicalSuccessHandoff {
                    route_closing,
                    main_lease,
                    shm_lease,
                    callbacks_in_flight,
                } => {
                    snapshot.physical_success_handoff_retentions += 1;
                    snapshot.physical_success_route_closing = route_closing;
                    snapshot.physical_success_main_lease = main_lease;
                    snapshot.physical_success_shm_lease = shm_lease;
                    snapshot.physical_success_callbacks_in_flight = callbacks_in_flight;
                }
            }
        }
        Ok(snapshot)
    }

    #[cfg(test)]
    fn explicit_terminal_custody_test_kind<Retained: 'static>(
    ) -> ManagedSqliteRegistryTerminalCustodyTestRetentionKind {
        use std::any::TypeId;

        if TypeId::of::<Retained>() == TypeId::of::<ManagedSqliteRegistryCallbackLease>() {
            ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CallbackLease
        } else if TypeId::of::<Retained>()
            == TypeId::of::<ManagedSqliteRegistryCallbackCompletionReceipt>()
            || TypeId::of::<Retained>()
                == TypeId::of::<ManagedSqliteRegistryConnectionClosedReceipt>()
            || TypeId::of::<Retained>() == TypeId::of::<ManagedSqliteRegistryRetirementReceipt>()
        {
            ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CompletionEvidence
        } else {
            ManagedSqliteRegistryTerminalCustodyTestRetentionKind::OtherTerminalCustody
        }
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn claim_main(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryFileLease, ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.claim_main(route))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn claim_sidecar(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        role: ManagedSqliteLogicalFileRole,
    ) -> Result<ManagedSqliteRegistryFileLease, ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.claim_sidecar(route, role))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn claim_shm(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryShmLease, ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.claim_shm(route))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn activate_connection(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.activate_connection(route))
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn close_sidecar(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        lease: ManagedSqliteRegistryFileLease,
        receipt: ManagedSqliteFileCloseReceipt,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let outcome = lease.close_with_file_receipt(receipt);
        self.apply_route_retaining_failure(
            route,
            (lease, outcome),
            #[cfg(test)]
            ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CompletionEvidence,
            |routes, evidence| routes.close_file(route, &evidence.0, &evidence.1),
        )
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn close_main(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        lease: ManagedSqliteRegistryFileLease,
        receipt: ManagedSqliteMainFileCloseReceipt,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let outcome = lease.close_with_main_receipt(receipt);
        self.apply_route_retaining_failure(
            route,
            (lease, outcome),
            #[cfg(test)]
            ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CompletionEvidence,
            |routes, evidence| routes.close_file(route, &evidence.0, &evidence.1),
        )
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn close_wal_main(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        main: ManagedSqliteRegistryFileLease,
        shm: ManagedSqliteRegistryShmLease,
        receipt: ManagedSqliteWalMainCloseReceipt,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let proofs = ManagedSqliteRegistryWalMainCloseProofs::from_receipt(&main, &shm, receipt);
        let (main_outcome, shm_outcome) = match proofs {
            Ok(proofs) => {
                let (main_proof, shm_proof) = proofs.into_parts();
                (
                    ManagedSqliteRegistryCloseOutcome::Proven(main_proof),
                    ManagedSqliteRegistryCloseOutcome::Proven(shm_proof),
                )
            }
            Err(reason) => (
                ManagedSqliteRegistryCloseOutcome::Unproven(reason),
                ManagedSqliteRegistryCloseOutcome::Unproven(reason),
            ),
        };
        self.apply_route_retaining_failure(
            route,
            (main, shm, main_outcome, shm_outcome),
            #[cfg(test)]
            ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CompletionEvidence,
            |routes, evidence| {
                routes.close_wal_main(route, &evidence.0, &evidence.1, &evidence.2, &evidence.3)
            },
        )
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn connection_close_failed(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        reason: ManagedSqliteRegistryTerminalReason,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| {
            routes.connection_close_failed(route, reason)
        })
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn observe_connection_closed(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.apply_route(route, |routes| routes.observe_connection_closed(route))
    }

    #[cfg(test)]
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn observe_connection_closed_after_callback(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        callback: ManagedSqliteRegistryCallbackCompletionReceipt,
    ) -> Result<
        ManagedSqliteRegistryConnectionClosedReceipt,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        self.apply_route_retaining_failure(
            route,
            callback,
            ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CompletionEvidence,
            |routes, callback| routes.observe_connection_closed_after_callback(route, callback),
        )
    }

    #[cfg(test)]
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn retire_closed_after_observation(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
        observed: ManagedSqliteRegistryConnectionClosedReceipt,
    ) -> Result<ManagedSqliteRegistryRetirementReceipt, ManagedSqliteRegistryProcessRouteRejection>
    {
        self.apply_route_retaining_failure(
            route,
            observed,
            ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CompletionEvidence,
            |routes, observed| routes.retire_closed_after_observation(route, observed),
        )
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn retire_closed(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryRetirementReceipt, ManagedSqliteRegistryProcessRouteRejection>
    {
        self.apply_route(route, |routes| routes.retire_closed(route))
    }
}
