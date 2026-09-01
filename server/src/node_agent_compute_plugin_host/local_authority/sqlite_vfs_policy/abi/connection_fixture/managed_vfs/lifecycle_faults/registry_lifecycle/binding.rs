//! Route-bound RegistryLifecycle binding and close-fault trait projection.

use super::super::super::shared_namespace::ManagedTestLogicalRouteRemovalReceipt;
use super::*;

impl ManagedTestLifecycleFaultBinding {
    pub(in super::super::super) fn install_connection_observation_sidecar(
        &self,
        file: PinnedManagedSqliteFile,
    ) -> Result<(), &'static str> {
        self.controller
            .install_connection_observation_sidecar(self.route, file)
    }

    pub(in super::super::super) fn observe_registry_lifecycle_stage(
        &self,
        stage: ManagedSqliteRegistryLifecycleStage,
    ) -> Result<(), ()> {
        self.controller.observe_registry_stage(self.route, stage)
    }

    pub(in super::super::super) fn install_registry_lifecycle_control(
        &self,
        control: ManagedTestRegistryLifecycleControl,
    ) -> Result<(), &'static str> {
        self.controller
            .install_registry_control(self.route, control)
    }

    pub(in super::super::super) fn registry_lifecycle_trace(
        &self,
    ) -> Result<ManagedTestRegistryLifecycleTraceSnapshot, &'static str> {
        self.controller.registry_trace(self.route)
    }

    pub(in super::super::super) fn retain_registry_retirement(
        &self,
        receipt: ManagedSqliteRegistryRetirementReceipt,
    ) {
        self.controller
            .retain_registry_retirement(self.route, receipt);
    }

    pub(in super::super::super) fn retain_logical_removal(
        &self,
        receipt: ManagedTestLogicalRouteRemovalReceipt,
    ) {
        self.controller.retain_logical_removal(self.route, receipt);
    }
}

impl ManagedSqliteRegistryCloseLifecycleFaults for ManagedTestLifecycleFaultBinding {
    fn before(&self, phase: ManagedSqliteRegistryCloseLifecyclePhase) -> Result<bool, ()> {
        self.before(registry_phase(phase))
    }

    fn after_success(&self, phase: ManagedSqliteRegistryCloseLifecyclePhase) -> Result<bool, ()> {
        self.after_success(registry_phase(phase))
    }

    fn native_failure(&self, phase: ManagedSqliteRegistryCloseLifecyclePhase) {
        self.controller
            .native_failure(Some(self.route), registry_phase(phase));
    }

    fn observe_unmap_runtime_event(
        &self,
        event: crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryUnmapRuntimeEvent,
    ) -> Result<(), ()> {
        self.controller
            .observe_unmap_runtime_event(self.route, event)
    }

    fn unmap_runtime_observation_enabled(&self) -> Result<bool, ()> {
        self.controller
            .unmap_runtime_observation_enabled(self.route)
    }

    fn claim_native_failure_gate(
        &self,
        phase: ManagedSqliteRegistryCloseLifecyclePhase,
    ) -> Result<bool, ()> {
        self.claim_native_failure_gate(registry_phase(phase))
    }

    fn publish_retirement(
        &self,
        receipt: ManagedSqliteRegistryRetirementReceipt,
    ) -> Result<(), ()> {
        self.controller
            .publish_registry_retirement(self.route, receipt)
    }

    fn retain_retirement_failure(&self, receipt: ManagedSqliteRegistryRetirementReceipt) {
        self.retain_registry_retirement(receipt);
    }

    #[cfg(windows)]
    fn claim_unsafe_shm_route_preemption(&self) -> Result<bool, ()> {
        ManagedTestLifecycleFaultBinding::claim_unsafe_shm_route_preemption(self)
    }

    #[cfg(windows)]
    fn record_unsafe_shm_route_preemption_receipt(
        &self,
        receipt: ManagedSqliteRegistryUnsafeShmRoutePreemptionReceipt,
    ) -> Result<(), ()> {
        ManagedTestLifecycleFaultBinding::record_unsafe_shm_route_preemption_receipt(self, receipt)
    }

    #[cfg(windows)]
    fn claim_ordinary_shm_lock_route_preemption(
        &self,
        request: crate::node_agent_managed_fs::ManagedSqliteShmLockRequest,
        outcome: crate::node_agent_managed_fs::ManagedSqliteShmLockAttempt,
    ) -> Result<bool, ()> {
        ManagedTestLifecycleFaultBinding::claim_ordinary_shm_lock_route_preemption(
            self, request, outcome,
        )
    }

    #[cfg(windows)]
    fn record_ordinary_shm_lock_route_preemption_receipt(
        &self,
        receipt: ManagedSqliteRegistryOrdinaryShmLockRoutePreemptionReceipt,
    ) -> Result<(), ()> {
        ManagedTestLifecycleFaultBinding::record_ordinary_shm_lock_route_preemption_receipt(
            self, receipt,
        )
    }

    fn take_connection_observation_sidecar(&self) -> Result<Option<PinnedManagedSqliteFile>, ()> {
        self.controller
            .take_connection_observation_sidecar(self.route)
    }

    fn observe_registry_lifecycle_stage(
        &self,
        stage: ManagedSqliteRegistryLifecycleStage,
    ) -> Result<(), ()> {
        self.controller.observe_registry_stage(self.route, stage)
    }

    #[cfg(windows)]
    fn claim_physical_success_handoff(&self) -> Result<bool, ()> {
        self.controller
            .claim_joint_close_physical_success(self.route)
    }

    #[cfg(windows)]
    fn claim_registry_wal_main_native_uncertain(&self) -> Result<bool, ()> {
        self.controller
            .claim_joint_close_registry_native(self.route)
    }

    #[cfg(windows)]
    fn claim_close_callback_admission_rejection(&self) -> Result<bool, ()> {
        self.controller
            .claim_joint_close_callback_admission(self.route)
    }

    #[cfg(windows)]
    fn claim_begin_connection_close_rejection(&self) -> Result<bool, ()> {
        self.controller
            .claim_joint_close_begin_connection_close(self.route)
    }
}

fn registry_phase(
    phase: ManagedSqliteRegistryCloseLifecyclePhase,
) -> ManagedTestLifecycleFaultPhase {
    match phase {
        ManagedSqliteRegistryCloseLifecyclePhase::BarrierCallbackCompletion => {
            ManagedTestLifecycleFaultPhase::BarrierCallbackCompletion
        }
        ManagedSqliteRegistryCloseLifecyclePhase::UnmapCallbackCompletion => {
            ManagedTestLifecycleFaultPhase::UnmapCallbackCompletion
        }
        ManagedSqliteRegistryCloseLifecyclePhase::RegistryWalMainClose => {
            ManagedTestLifecycleFaultPhase::RegistryWalMainClose
        }
        ManagedSqliteRegistryCloseLifecyclePhase::CallbackCompletion => {
            ManagedTestLifecycleFaultPhase::CallbackCompletion
        }
        ManagedSqliteRegistryCloseLifecyclePhase::ConnectionObservation => {
            ManagedTestLifecycleFaultPhase::ConnectionObservation
        }
        ManagedSqliteRegistryCloseLifecyclePhase::RouteRetirement => {
            ManagedTestLifecycleFaultPhase::RouteRetirement
        }
    }
}
