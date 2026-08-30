//! Exact-route, one-shot owner gate for the JointClose registry-native boundary.

use std::sync::Mutex;

use super::*;

#[derive(Clone, Copy)]
struct Slot {
    route: ManagedSqliteRegistryRouteHandle,
    claims: usize,
}

pub(super) struct ManagedSqliteRegistryWalMainNativeUncertainTestGate {
    slots: Mutex<Vec<Slot>>,
}

pub(super) struct ManagedSqliteRegistryCloseCallbackAdmissionTestGate {
    slots: Mutex<Vec<Slot>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteRegistryWalMainNativeUncertainTestSnapshot
{
    claims: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) struct ManagedSqliteRegistryCloseCallbackAdmissionTestSnapshot
{
    claims: usize,
}

impl ManagedSqliteRegistryWalMainNativeUncertainTestSnapshot {
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn claim_count(
        self,
    ) -> usize {
        self.claims
    }
}

impl ManagedSqliteRegistryCloseCallbackAdmissionTestSnapshot {
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn claim_count(
        self,
    ) -> usize {
        self.claims
    }
}

impl ManagedSqliteRegistryWalMainNativeUncertainTestGate {
    pub(super) fn new() -> Self {
        Self {
            slots: Mutex::new(Vec::new()),
        }
    }

    fn arm(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let mut slots = self
            .slots
            .lock()
            .map_err(|_| ManagedSqliteRegistryProcessRouteRejection::OwnerPoisoned)?;
        if slots.iter().any(|slot| slot.route == route) {
            return Err(ManagedSqliteRegistryProcessRouteRejection::RegistryWalMainNativeUncertain);
        }
        slots.push(Slot { route, claims: 0 });
        Ok(())
    }

    fn claim(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<bool, ManagedSqliteRegistryProcessRouteRejection> {
        let mut slots = self
            .slots
            .lock()
            .map_err(|_| ManagedSqliteRegistryProcessRouteRejection::OwnerPoisoned)?;
        let Some(slot) = slots.iter_mut().find(|slot| slot.route == route) else {
            return Ok(false);
        };
        if slot.claims != 0 {
            return Err(ManagedSqliteRegistryProcessRouteRejection::RegistryWalMainNativeUncertain);
        }
        slot.claims = 1;
        Ok(true)
    }

    fn snapshot(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<
        ManagedSqliteRegistryWalMainNativeUncertainTestSnapshot,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        let slots = self
            .slots
            .lock()
            .map_err(|_| ManagedSqliteRegistryProcessRouteRejection::OwnerPoisoned)?;
        let slot = slots
            .iter()
            .find(|slot| slot.route == route)
            .ok_or(ManagedSqliteRegistryProcessRouteRejection::RegistryWalMainNativeUncertain)?;
        Ok(ManagedSqliteRegistryWalMainNativeUncertainTestSnapshot {
            claims: slot.claims,
        })
    }
}

impl ManagedSqliteRegistryCloseCallbackAdmissionTestGate {
    pub(super) fn new() -> Self {
        Self {
            slots: Mutex::new(Vec::new()),
        }
    }

    fn arm(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        let mut slots = self
            .slots
            .lock()
            .map_err(|_| ManagedSqliteRegistryProcessRouteRejection::OwnerPoisoned)?;
        if slots.iter().any(|slot| slot.route == route) {
            return Err(ManagedSqliteRegistryProcessRouteRejection::CloseCallbackAdmissionRejected);
        }
        slots.push(Slot { route, claims: 0 });
        Ok(())
    }

    fn claim(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<bool, ManagedSqliteRegistryProcessRouteRejection> {
        let mut slots = self
            .slots
            .lock()
            .map_err(|_| ManagedSqliteRegistryProcessRouteRejection::OwnerPoisoned)?;
        let Some(slot) = slots.iter_mut().find(|slot| slot.route == route) else {
            return Ok(false);
        };
        if slot.claims != 0 {
            return Err(ManagedSqliteRegistryProcessRouteRejection::CloseCallbackAdmissionRejected);
        }
        slot.claims = 1;
        Ok(true)
    }

    fn snapshot(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<
        ManagedSqliteRegistryCloseCallbackAdmissionTestSnapshot,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        let slots = self
            .slots
            .lock()
            .map_err(|_| ManagedSqliteRegistryProcessRouteRejection::OwnerPoisoned)?;
        let slot = slots
            .iter()
            .find(|slot| slot.route == route)
            .ok_or(ManagedSqliteRegistryProcessRouteRejection::CloseCallbackAdmissionRejected)?;
        Ok(ManagedSqliteRegistryCloseCallbackAdmissionTestSnapshot {
            claims: slot.claims,
        })
    }
}

impl<Custody, NonceSource> ManagedSqliteRegistryProcessOwner<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn arm_registry_wal_main_native_uncertain(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.joint_close_registry_native_fault.arm(route)
    }

    pub(super) fn claim_registry_wal_main_native_uncertain(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<bool, ManagedSqliteRegistryProcessRouteRejection> {
        self.joint_close_registry_native_fault.claim(route)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn registry_wal_main_native_uncertain_test_snapshot(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<
        ManagedSqliteRegistryWalMainNativeUncertainTestSnapshot,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        self.joint_close_registry_native_fault.snapshot(route)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry) fn arm_close_callback_admission_rejection(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<(), ManagedSqliteRegistryProcessRouteRejection> {
        self.joint_close_callback_admission_fault.arm(route)
    }

    pub(super) fn claim_close_callback_admission_rejection(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<bool, ManagedSqliteRegistryProcessRouteRejection> {
        self.joint_close_callback_admission_fault.claim(route)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn close_callback_admission_test_snapshot(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<
        ManagedSqliteRegistryCloseCallbackAdmissionTestSnapshot,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        self.joint_close_callback_admission_fault.snapshot(route)
    }
}
