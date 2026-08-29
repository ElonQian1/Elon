//! Narrow delegation from one routed main-file binding to exact-target Unmap test authority.

use crate::node_agent_managed_fs::{
    ManagedSqliteShmTestUnmapDeletePrestate, ManagedSqliteShmTestUnmapNativeOperation,
    ManagedSqliteShmTestUnmapReceipt,
};

use super::{ManagedTestShmFaultPlanBinding, ManagedTestShmFaultPlanState};
use crate::node_agent_managed_fs::ManagedSqliteShmFailurePhase;

impl ManagedTestShmFaultPlanBinding {
    pub(in super::super) fn begin_unmap_action_observation(&self) -> Result<(), &'static str> {
        self.observer()?.begin_unmap_action_observation()
    }

    pub(in super::super) fn install_unmap_native_operation(
        &self,
        operation: ManagedSqliteShmTestUnmapNativeOperation,
    ) -> Result<(), &'static str> {
        self.observer()?.install_unmap_native_operation(operation)
    }

    pub(in super::super) fn set_unmap_delete_prestate(
        &self,
        prestate: ManagedSqliteShmTestUnmapDeletePrestate,
    ) -> Result<(), &'static str> {
        self.observer()?.set_unmap_delete_prestate(prestate)
    }

    pub(in super::super) fn observe_unmap_test_receipt(
        &self,
    ) -> Result<ManagedSqliteShmTestUnmapReceipt, &'static str> {
        self.observer()?.observe_unmap_test_receipt()
    }

    pub(in super::super) fn finish_unmap_test_receipt(
        &self,
    ) -> Result<ManagedSqliteShmTestUnmapReceipt, &'static str> {
        self.observer()?.finish_unmap_test_receipt()
    }

    pub(in super::super) fn unmap_fault_was_observed(
        &self,
        phase: ManagedSqliteShmFailurePhase,
        occurrence: u32,
    ) -> Result<bool, &'static str> {
        let state = self
            .slot
            .state
            .lock()
            .map_err(|_| "managed SHM fault plan slot poisoned")?;
        match &*state {
            ManagedTestShmFaultPlanState::Promoted(_) => Ok(false),
            ManagedTestShmFaultPlanState::Installed(probe) => probe.was_observed(phase, occurrence),
            _ => Err("managed SHM target observer is not installed"),
        }
    }
}
