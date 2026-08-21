//! Sealed owner observations and independently incremented RegistrationShutdown action counters.

use std::{cell::RefCell, sync::Arc};

use anyhow::{anyhow, Context};
use rusqlite::ffi;

use super::super::{
    a2b2_cases::{RegistrationShutdownActualCounts, RegistrationShutdownSelector},
    lifecycle_faults::ManagedTestRegistrationShutdownQuarantineWitness,
    shared_namespace::ManagedTestRegistrationShutdownRouteSnapshot,
    ManagedSqliteTestVfsRouteCustodySnapshot, ManagedSqliteTestVfsRoutePhase,
    ManagedTestLifecycleFaultController, ManagedTestLifecycleFaultObservation,
    ManagedTestLifecycleFaultPhase, ManagedTestLifecycleFaultTiming, ManagedTestVfsRouteCollection,
    TestCallback, TestRoute,
};
use super::ObservedRegistrationShutdownOutcome;

const COUNTER_COUNT: usize = 30;

pub(in super::super) struct RegistrationShutdownActions {
    selector: RegistrationShutdownSelector,
    counters: RegistrationShutdownCounters,
    preexisting_callback: RefCell<Option<ManagedTestPreexistingCallbackLeaseWitness>>,
    unregister_receipt: RefCell<Option<ManagedTestVfsUnregisterActionReceipt>>,
    outcome: RefCell<Option<ObservedRegistrationShutdownOutcome>>,
}

impl RegistrationShutdownActions {
    pub(in super::super) fn new(selector: RegistrationShutdownSelector) -> Self {
        Self {
            selector,
            counters: RegistrationShutdownCounters::new(),
            preexisting_callback: RefCell::new(None),
            unregister_receipt: RefCell::new(None),
            outcome: RefCell::new(None),
        }
    }

    pub(in super::super) fn retain_preexisting_callback(
        &self,
        route: &Arc<TestRoute>,
    ) -> anyhow::Result<()> {
        if self.preexisting_callback.borrow().is_some() {
            return Err(anyhow!(
                "preexisting callback witness was already installed"
            ));
        }
        let callback = route
            .begin_access_callback()
            .map_err(|()| anyhow!("begin real preexisting registration callback"))?;
        let callback: &'static TestCallback = Box::leak(Box::new(callback));
        self.preexisting_callback
            .replace(Some(ManagedTestPreexistingCallbackLeaseWitness {
                _callback: callback,
                route: Arc::clone(route),
            }));
        Ok(())
    }

    pub(in super::super) fn observe_route_index(
        &self,
        routes: &ManagedTestVfsRouteCollection,
    ) -> anyhow::Result<usize> {
        let snapshot = routes.registration_shutdown_snapshot()?;
        if self.selector == RegistrationShutdownSelector::RouteIndexObservation {
            require_one_active_route(&snapshot)?;
            self.record_outcome(ObservedRegistrationShutdownOutcome::route_index())?;
            self.counters.increment(Counter::FaultObserve)?;
            return Err(anyhow!(
                "deterministic injected route-index observation uncertainty"
            ));
        }
        if snapshot.live_routes() == 0 {
            return Ok(0);
        }
        let custody = require_one_active_route(&snapshot)?;
        if custody.callbacks_in_flight() != 0 {
            let callback = self.preexisting_callback.borrow();
            let witness = callback
                .as_ref()
                .ok_or_else(|| anyhow!("registry callback has no preexisting lease witness"))?;
            witness.validate(&snapshot)?;
            self.record_outcome(ObservedRegistrationShutdownOutcome::outstanding_callback())?;
            return Err(anyhow!(
                "registration shutdown observed a preexisting callback lease"
            ));
        }
        self.record_outcome(ObservedRegistrationShutdownOutcome::live_route())?;
        Ok(snapshot.live_routes())
    }

    pub(in super::super) fn injected_pre_native_retryable_or_call_sqlite_unregister(
        &self,
        table: *mut ffi::sqlite3_vfs,
    ) -> std::os::raw::c_int {
        // This is the unregister action-seam entry count. It is not a claim that
        // `sqlite3_vfs_unregister` was called.
        if self
            .counters
            .increment(Counter::VfsUnregisterAttempt)
            .is_err()
            || self.unregister_receipt.borrow().is_some()
        {
            return ffi::SQLITE_MISUSE;
        }
        let receipt = if self.selector == RegistrationShutdownSelector::VfsUnregisterNativeRetryable
        {
            let _ = self.counters.increment(Counter::FaultObserve);
            ManagedTestVfsUnregisterActionReceipt {
                code: ffi::SQLITE_BUSY,
                sqlite_call_performed: false,
                injected_pre_native_retryable: true,
            }
        } else {
            // SAFETY: the caller proved the route index empty and passes its live table.
            let code = unsafe { ffi::sqlite3_vfs_unregister(table) };
            ManagedTestVfsUnregisterActionReceipt {
                code,
                sqlite_call_performed: true,
                injected_pre_native_retryable: false,
            }
        };
        if receipt.code == ffi::SQLITE_OK {
            let _ = self.counters.increment(Counter::VfsUnregisterSuccess);
        }
        self.unregister_receipt.replace(Some(receipt));
        receipt.code
    }

    pub(in super::super) fn observe_lifecycle(
        &self,
        lifecycle: &Arc<ManagedTestLifecycleFaultController>,
    ) -> anyhow::Result<()> {
        let observations = lifecycle.observations().map_err(anyhow::Error::msg)?;
        if observations.iter().any(|observation| {
            observation.route.is_some()
                || observation.phase != ManagedTestLifecycleFaultPhase::VfsUnregister
                || observation.occurrence != 1
        }) {
            return Err(anyhow!(
                "registration shutdown observed an unrelated lifecycle transition"
            ));
        }
        for observation in &observations {
            if observation.triggered {
                self.counters.increment(Counter::FaultObserve)?;
                self.counters.increment(Counter::FaultTrigger)?;
            }
        }
        self.counters.set(
            Counter::FaultPending,
            u8::try_from(lifecycle.pending_count().map_err(anyhow::Error::msg)?)
                .context("pending lifecycle fault count exceeds u8")?,
        );
        let receipt = *self.unregister_receipt.borrow();
        match (observations.as_slice(), receipt) {
            ([before], None)
                if lifecycle_observation(
                    before,
                    ManagedTestLifecycleFaultTiming::BeforeCall,
                    true,
                ) =>
            {
                self.record_outcome(ObservedRegistrationShutdownOutcome::before_call())?;
            }
            ([before, native], Some(receipt))
                if lifecycle_observation(
                    before,
                    ManagedTestLifecycleFaultTiming::BeforeCall,
                    false,
                ) && lifecycle_observation(
                    native,
                    ManagedTestLifecycleFaultTiming::NativeFailure,
                    false,
                ) && receipt.injected_pre_native_retryable
                    && !receipt.sqlite_call_performed
                    && receipt.code == ffi::SQLITE_BUSY =>
            {
                self.record_outcome(
                    ObservedRegistrationShutdownOutcome::injected_pre_native_retryable(),
                )?;
            }
            ([before, after], Some(receipt))
                if lifecycle_observation(
                    before,
                    ManagedTestLifecycleFaultTiming::BeforeCall,
                    false,
                ) && lifecycle_observation(
                    after,
                    ManagedTestLifecycleFaultTiming::AfterSuccess,
                    true,
                ) && receipt.sqlite_call_performed
                    && !receipt.injected_pre_native_retryable
                    && receipt.code == ffi::SQLITE_OK =>
            {
                self.record_outcome(ObservedRegistrationShutdownOutcome::after_success())?;
            }
            ([before, after], Some(receipt))
                if lifecycle_observation(
                    before,
                    ManagedTestLifecycleFaultTiming::BeforeCall,
                    false,
                ) && lifecycle_observation(
                    after,
                    ManagedTestLifecycleFaultTiming::AfterSuccess,
                    false,
                ) && receipt.sqlite_call_performed
                    && !receipt.injected_pre_native_retryable
                    && receipt.code == ffi::SQLITE_OK =>
            {
                self.record_outcome(ObservedRegistrationShutdownOutcome::success())?;
            }
            ([], None) => {}
            _ => {
                return Err(anyhow!(
                    "VFS unregister receipt and lifecycle observations disagree"
                ));
            }
        }
        Ok(())
    }

    pub(in super::super) fn observe_quarantined_custody(
        &self,
        witness: &ManagedTestRegistrationShutdownQuarantineWitness,
    ) -> anyhow::Result<()> {
        if !witness.retained().map_err(anyhow::Error::msg)? {
            return Err(anyhow!(
                "registration quarantine did not retain exact VFS custody"
            ));
        }
        self.record_outcome(ObservedRegistrationShutdownOutcome::quarantined_custody())
    }

    pub(in super::super) fn observe_custody_retained(&self) -> anyhow::Result<()> {
        self.counters.increment(Counter::CustodyRetain)
    }

    pub(in super::super) fn native_success(&self) -> u8 {
        self.counters.get(Counter::VfsUnregisterSuccess)
    }

    pub(in super::super) fn take_outcome(
        &self,
    ) -> anyhow::Result<ObservedRegistrationShutdownOutcome> {
        self.outcome
            .borrow_mut()
            .take()
            .context("registration shutdown produced no sealed observed outcome")
    }

    pub(in super::super) fn snapshot(&self) -> RegistrationShutdownActualCounts {
        self.counters.snapshot()
    }

    fn record_outcome(&self, outcome: ObservedRegistrationShutdownOutcome) -> anyhow::Result<()> {
        if self.outcome.borrow_mut().replace(outcome).is_some() {
            return Err(anyhow!(
                "registration shutdown produced more than one observed outcome"
            ));
        }
        Ok(())
    }
}

struct ManagedTestPreexistingCallbackLeaseWitness {
    _callback: &'static TestCallback,
    route: Arc<TestRoute>,
}

impl ManagedTestPreexistingCallbackLeaseWitness {
    fn validate(
        &self,
        snapshot: &ManagedTestRegistrationShutdownRouteSnapshot,
    ) -> anyhow::Result<()> {
        let route = snapshot
            .only_route()
            .context("preexisting callback route disappeared")?;
        let custody = snapshot
            .only_route_custody()
            .context("preexisting callback custody disappeared")?;
        if !Arc::ptr_eq(route, &self.route) || custody.callbacks_in_flight() != 1 {
            return Err(anyhow!(
                "preexisting callback token and registry in-flight count disagree"
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct ManagedTestVfsUnregisterActionReceipt {
    code: std::os::raw::c_int,
    sqlite_call_performed: bool,
    injected_pre_native_retryable: bool,
}

fn lifecycle_observation(
    observation: &ManagedTestLifecycleFaultObservation,
    timing: ManagedTestLifecycleFaultTiming,
    triggered: bool,
) -> bool {
    observation.timing == timing && observation.triggered == triggered
}

fn require_one_active_route(
    snapshot: &ManagedTestRegistrationShutdownRouteSnapshot,
) -> anyhow::Result<ManagedSqliteTestVfsRouteCustodySnapshot> {
    let custody = snapshot
        .only_route_custody()
        .context("registration shutdown route custody is not singular")?;
    if snapshot.live_routes() != 1
        || snapshot.logical_names() != 3
        || custody.phase() != ManagedSqliteTestVfsRoutePhase::Active
        || !custody.connection_owner()
    {
        return Err(anyhow!(
            "registration shutdown route is not one live active owner"
        ));
    }
    Ok(custody)
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum Counter {
    RawStateTakeAttempt,
    RawStateTakeSuccess,
    RawStateAbandon,
    MethodsClear,
    CallbackBegin,
    CallbackCompleteAttempt,
    CallbackCompleteSuccess,
    SelectedActionAttempt,
    SelectedActionSuccess,
    ShmDetach,
    MainUnlockAttempt,
    MainUnlockSuccess,
    MainFileCloseAttempt,
    MainFileCloseSuccess,
    RegistryCloseAttempt,
    RegistryCloseSuccess,
    ConnectionObserveAttempt,
    ConnectionObserveSuccess,
    RegistryRouteRemoveAttempt,
    RegistryRouteRemoveSuccess,
    LogicalNamesRemoveAttempt,
    LogicalNamesRemoveSuccess,
    LogicalNamesRemove,
    VfsUnregisterAttempt,
    VfsUnregisterSuccess,
    FaultObserve,
    FaultTrigger,
    FaultPending,
    CustodyRetain,
    PhysicalRetry,
}

struct RegistrationShutdownCounters {
    values: [std::cell::Cell<u8>; COUNTER_COUNT],
}

impl RegistrationShutdownCounters {
    fn new() -> Self {
        Self {
            values: std::array::from_fn(|_| std::cell::Cell::new(0)),
        }
    }

    fn get(&self, counter: Counter) -> u8 {
        self.values[counter as usize].get()
    }

    fn set(&self, counter: Counter, value: u8) {
        self.values[counter as usize].set(value);
    }

    fn increment(&self, counter: Counter) -> anyhow::Result<()> {
        let value = self
            .get(counter)
            .checked_add(1)
            .context("registration shutdown counter overflow")?;
        self.set(counter, value);
        Ok(())
    }

    fn snapshot(&self) -> RegistrationShutdownActualCounts {
        RegistrationShutdownActualCounts {
            raw_state_take_attempt: self.get(Counter::RawStateTakeAttempt),
            raw_state_take_success: self.get(Counter::RawStateTakeSuccess),
            raw_state_abandon: self.get(Counter::RawStateAbandon),
            methods_clear: self.get(Counter::MethodsClear),
            callback_begin: self.get(Counter::CallbackBegin),
            callback_complete_attempt: self.get(Counter::CallbackCompleteAttempt),
            callback_complete_success: self.get(Counter::CallbackCompleteSuccess),
            selected_action_attempt: self.get(Counter::SelectedActionAttempt),
            selected_action_success: self.get(Counter::SelectedActionSuccess),
            shm_detach: self.get(Counter::ShmDetach),
            main_unlock_attempt: self.get(Counter::MainUnlockAttempt),
            main_unlock_success: self.get(Counter::MainUnlockSuccess),
            main_file_close_attempt: self.get(Counter::MainFileCloseAttempt),
            main_file_close_success: self.get(Counter::MainFileCloseSuccess),
            registry_close_attempt: self.get(Counter::RegistryCloseAttempt),
            registry_close_success: self.get(Counter::RegistryCloseSuccess),
            connection_observe_attempt: self.get(Counter::ConnectionObserveAttempt),
            connection_observe_success: self.get(Counter::ConnectionObserveSuccess),
            registry_route_remove_attempt: self.get(Counter::RegistryRouteRemoveAttempt),
            registry_route_remove_success: self.get(Counter::RegistryRouteRemoveSuccess),
            logical_names_remove_attempt: self.get(Counter::LogicalNamesRemoveAttempt),
            logical_names_remove_success: self.get(Counter::LogicalNamesRemoveSuccess),
            logical_names_remove: self.get(Counter::LogicalNamesRemove),
            vfs_unregister_attempt: self.get(Counter::VfsUnregisterAttempt),
            vfs_unregister_success: self.get(Counter::VfsUnregisterSuccess),
            fault_observe: self.get(Counter::FaultObserve),
            fault_trigger: self.get(Counter::FaultTrigger),
            fault_pending: self.get(Counter::FaultPending),
            custody_retain: self.get(Counter::CustodyRetain),
            physical_retry: self.get(Counter::PhysicalRetry),
        }
    }
}
