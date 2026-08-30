//! Exact-route, one-shot controls for real JointClose native boundaries.

use std::collections::HashMap;
use std::sync::atomic::Ordering;

use super::*;
use crate::node_agent_managed_fs::{
    ManagedSqliteMainCloseTestFaultPhase, ManagedSqliteMainCloseTestNativeEvidence,
    ManagedSqliteMainCloseTestNativeObservation, ManagedSqliteMainCloseTestNativeRequest,
    ManagedSqliteMainLockHeldRangePrestate, ManagedSqliteMainLockOffsetClass,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum ManagedTestJointCloseControl {
    MainNative(ManagedSqliteMainCloseTestNativeRequest),
    BeginConnectionCloseRejected,
    CallbackAdmissionRejected,
    PhysicalSuccessHandoff,
    RegistryWalMainNativeUncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ControlSlot {
    control: ManagedTestJointCloseControl,
    claimed: bool,
    evidence: Option<ManagedSqliteMainCloseTestNativeEvidence>,
}

#[derive(Default)]
pub(super) struct ManagedTestJointCloseState {
    slots: HashMap<ManagedTestRouteOrdinal, ControlSlot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestJointCloseControlSnapshot {
    control: ManagedTestJointCloseControl,
    claimed: bool,
    evidence: Option<ManagedSqliteMainCloseTestNativeEvidence>,
}

impl ManagedTestJointCloseControlSnapshot {
    pub(in super::super) fn control(self) -> ManagedTestJointCloseControl {
        self.control
    }

    pub(in super::super) fn claimed(self) -> bool {
        self.claimed
    }

    pub(in super::super) fn evidence(self) -> Option<ManagedSqliteMainCloseTestNativeEvidence> {
        self.evidence
    }

    pub(in super::super) fn pending_count(self) -> usize {
        usize::from(!self.claimed)
            + usize::from(
                matches!(self.control, ManagedTestJointCloseControl::MainNative(_))
                    && self.evidence.is_none(),
            )
    }
}

impl ManagedTestJointCloseState {
    fn install(
        &mut self,
        route: ManagedTestRouteOrdinal,
        control: ManagedTestJointCloseControl,
    ) -> Result<(), &'static str> {
        if self.slots.contains_key(&route) {
            return Err("JointClose route control is already installed");
        }
        self.slots.insert(
            route,
            ControlSlot {
                control,
                claimed: false,
                evidence: None,
            },
        );
        Ok(())
    }

    fn claim_main_native(
        &mut self,
        route: ManagedTestRouteOrdinal,
        phase: ManagedSqliteMainCloseTestFaultPhase,
    ) -> Result<Option<ManagedSqliteMainCloseTestNativeRequest>, ()> {
        let Some(slot) = self.slots.get_mut(&route) else {
            return Ok(None);
        };
        let ManagedTestJointCloseControl::MainNative(request) = slot.control else {
            return Ok(None);
        };
        if request_phase(request) != phase {
            return Ok(None);
        }
        if slot.claimed {
            return Err(());
        }
        slot.claimed = true;
        Ok(Some(request))
    }

    fn observe_main_native(
        &mut self,
        route: ManagedTestRouteOrdinal,
        evidence: ManagedSqliteMainCloseTestNativeEvidence,
    ) -> Result<(), ()> {
        let slot = self.slots.get_mut(&route).ok_or(())?;
        let ManagedTestJointCloseControl::MainNative(request) = slot.control else {
            return Err(());
        };
        if !slot.claimed || slot.evidence.is_some() || !evidence_matches(request, evidence) {
            return Err(());
        }
        slot.evidence = Some(evidence);
        Ok(())
    }

    fn claim_physical_success(&mut self, route: ManagedTestRouteOrdinal) -> Result<bool, ()> {
        let Some(slot) = self.slots.get_mut(&route) else {
            return Ok(false);
        };
        if slot.control != ManagedTestJointCloseControl::PhysicalSuccessHandoff {
            return Ok(false);
        }
        if slot.claimed {
            return Err(());
        }
        slot.claimed = true;
        Ok(true)
    }

    fn claim_registry_native(&mut self, route: ManagedTestRouteOrdinal) -> Result<bool, ()> {
        let Some(slot) = self.slots.get_mut(&route) else {
            return Ok(false);
        };
        if slot.control != ManagedTestJointCloseControl::RegistryWalMainNativeUncertain {
            return Ok(false);
        }
        if slot.claimed {
            return Err(());
        }
        slot.claimed = true;
        Ok(true)
    }

    fn claim_callback_admission(&mut self, route: ManagedTestRouteOrdinal) -> Result<bool, ()> {
        let Some(slot) = self.slots.get_mut(&route) else {
            return Ok(false);
        };
        if slot.control != ManagedTestJointCloseControl::CallbackAdmissionRejected {
            return Ok(false);
        }
        if slot.claimed {
            return Err(());
        }
        slot.claimed = true;
        Ok(true)
    }

    fn claim_begin_connection_close(&mut self, route: ManagedTestRouteOrdinal) -> Result<bool, ()> {
        let Some(slot) = self.slots.get_mut(&route) else {
            return Ok(false);
        };
        if slot.control != ManagedTestJointCloseControl::BeginConnectionCloseRejected {
            return Ok(false);
        }
        if slot.claimed {
            return Err(());
        }
        slot.claimed = true;
        Ok(true)
    }

    fn snapshot(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Option<ManagedTestJointCloseControlSnapshot> {
        self.slots
            .get(&route)
            .copied()
            .map(|slot| ManagedTestJointCloseControlSnapshot {
                control: slot.control,
                claimed: slot.claimed,
                evidence: slot.evidence,
            })
    }
}

impl ManagedTestLifecycleFaultBinding {
    pub(in super::super) fn install_joint_close_control(
        &self,
        control: ManagedTestJointCloseControl,
    ) -> Result<(), &'static str> {
        self.controller
            .install_joint_close_control(self.route, control)
    }

    pub(in super::super) fn joint_close_control_snapshot(
        &self,
    ) -> Result<ManagedTestJointCloseControlSnapshot, &'static str> {
        self.controller.joint_close_control_snapshot(self.route)
    }
}

impl ManagedTestLifecycleFaultController {
    fn install_joint_close_control(
        &self,
        route: ManagedTestRouteOrdinal,
        control: ManagedTestJointCloseControl,
    ) -> Result<(), &'static str> {
        self.state
            .lock()
            .map_err(|_| "lifecycle fault controller poisoned")?
            .joint_close
            .install(route, control)
    }

    pub(super) fn claim_joint_close_main_native(
        &self,
        route: ManagedTestRouteOrdinal,
        phase: ManagedSqliteMainCloseTestFaultPhase,
    ) -> Result<Option<ManagedSqliteMainCloseTestNativeRequest>, ()> {
        self.state
            .lock()
            .map_err(|_| self.terminal.store(true, Ordering::SeqCst))?
            .joint_close
            .claim_main_native(route, phase)
    }

    pub(super) fn observe_joint_close_main_native(
        &self,
        route: ManagedTestRouteOrdinal,
        evidence: ManagedSqliteMainCloseTestNativeEvidence,
    ) -> Result<(), ()> {
        let observed = self
            .state
            .lock()
            .map_err(|_| ())?
            .joint_close
            .observe_main_native(route, evidence);
        if observed.is_err() {
            self.terminal.store(true, Ordering::SeqCst);
        }
        observed
    }

    pub(super) fn claim_joint_close_physical_success(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<bool, ()> {
        self.state
            .lock()
            .map_err(|_| self.terminal.store(true, Ordering::SeqCst))?
            .joint_close
            .claim_physical_success(route)
    }

    pub(super) fn claim_joint_close_registry_native(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<bool, ()> {
        self.state
            .lock()
            .map_err(|_| self.terminal.store(true, Ordering::SeqCst))?
            .joint_close
            .claim_registry_native(route)
    }

    pub(super) fn claim_joint_close_callback_admission(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<bool, ()> {
        self.state
            .lock()
            .map_err(|_| self.terminal.store(true, Ordering::SeqCst))?
            .joint_close
            .claim_callback_admission(route)
    }

    pub(super) fn claim_joint_close_begin_connection_close(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<bool, ()> {
        self.state
            .lock()
            .map_err(|_| self.terminal.store(true, Ordering::SeqCst))?
            .joint_close
            .claim_begin_connection_close(route)
    }

    fn joint_close_control_snapshot(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<ManagedTestJointCloseControlSnapshot, &'static str> {
        self.state
            .lock()
            .map_err(|_| "lifecycle fault controller poisoned")?
            .joint_close
            .snapshot(route)
            .ok_or("JointClose route control is not installed")
    }
}

fn request_phase(
    request: ManagedSqliteMainCloseTestNativeRequest,
) -> ManagedSqliteMainCloseTestFaultPhase {
    match request {
        ManagedSqliteMainCloseTestNativeRequest::MainLockReleaseNativeUncertainShared
        | ManagedSqliteMainCloseTestNativeRequest::MainLockReleaseNativeUncertainReserved => {
            ManagedSqliteMainCloseTestFaultPhase::Unlock
        }
        ManagedSqliteMainCloseTestNativeRequest::MainFileCloseNativeRetryable
        | ManagedSqliteMainCloseTestNativeRequest::MainFileCloseNativeUncertain => {
            ManagedSqliteMainCloseTestFaultPhase::FileClose
        }
    }
}

fn evidence_matches(
    request: ManagedSqliteMainCloseTestNativeRequest,
    evidence: ManagedSqliteMainCloseTestNativeEvidence,
) -> bool {
    use ManagedSqliteMainCloseTestNativeObservation as Observation;
    use ManagedSqliteMainCloseTestNativeRequest as Request;
    match (request, evidence) {
        (
            Request::MainLockReleaseNativeUncertainShared,
            ManagedSqliteMainCloseTestNativeEvidence::MainLockRelease {
                held_range_prestate: ManagedSqliteMainLockHeldRangePrestate::Shared,
                selected_offset_class: ManagedSqliteMainLockOffsetClass::SharedRange,
                exact_call_occurrence,
                observation: Observation::ReturnReceiptUnavailable,
            },
        )
        | (
            Request::MainLockReleaseNativeUncertainReserved,
            ManagedSqliteMainCloseTestNativeEvidence::MainLockRelease {
                held_range_prestate: ManagedSqliteMainLockHeldRangePrestate::ReservedShared,
                selected_offset_class: ManagedSqliteMainLockOffsetClass::ReservedByte,
                exact_call_occurrence,
                observation: Observation::ReturnReceiptUnavailable,
            },
        ) => exact_call_occurrence.get() == 1,
        (
            Request::MainFileCloseNativeRetryable,
            ManagedSqliteMainCloseTestNativeEvidence::MainFileClose {
                exact_call_occurrence,
                observation: Observation::NativeFailureObserved,
            },
        )
        | (
            Request::MainFileCloseNativeUncertain,
            ManagedSqliteMainCloseTestNativeEvidence::MainFileClose {
                exact_call_occurrence,
                observation: Observation::ReturnReceiptUnavailable,
            },
        ) => exact_call_occurrence.get() == 1,
        _ => false,
    }
}
