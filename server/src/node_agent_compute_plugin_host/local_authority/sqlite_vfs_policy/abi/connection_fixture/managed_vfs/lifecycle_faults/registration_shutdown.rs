//! Sealed registration-level quarantine ownership for the shutdown acceptance harness.

use std::sync::{atomic::Ordering, Arc};

use super::{super::ManagedTestVfsRegistrationCustody, ManagedTestLifecycleFaultController};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedTestRegistrationShutdownQuarantineState {
    Vacant,
    Armed,
    Claimed,
    Retained,
}

pub(in super::super) struct ManagedTestRegistrationShutdownQuarantineWitness {
    controller: Arc<ManagedTestLifecycleFaultController>,
}

pub(in super::super) struct ManagedTestRegistrationShutdownQuarantineClaim {
    controller: Arc<ManagedTestLifecycleFaultController>,
}

impl ManagedTestLifecycleFaultController {
    pub(in super::super) fn arm_registration_shutdown_quarantine(
        self: &Arc<Self>,
    ) -> Result<ManagedTestRegistrationShutdownQuarantineWitness, &'static str> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "registration shutdown quarantine owner poisoned")?;
        if state.registration_shutdown_quarantine
            != ManagedTestRegistrationShutdownQuarantineState::Vacant
            || self.is_terminal()
        {
            return Err("registration shutdown quarantine owner is not vacant");
        }
        state.registration_shutdown_quarantine =
            ManagedTestRegistrationShutdownQuarantineState::Armed;
        Ok(ManagedTestRegistrationShutdownQuarantineWitness {
            controller: Arc::clone(self),
        })
    }

    pub(in super::super) fn claim_registration_shutdown_quarantine(
        self: &Arc<Self>,
    ) -> Result<Option<ManagedTestRegistrationShutdownQuarantineClaim>, &'static str> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "registration shutdown quarantine owner poisoned")?;
        match state.registration_shutdown_quarantine {
            ManagedTestRegistrationShutdownQuarantineState::Vacant => Ok(None),
            ManagedTestRegistrationShutdownQuarantineState::Armed => {
                state.registration_shutdown_quarantine =
                    ManagedTestRegistrationShutdownQuarantineState::Claimed;
                Ok(Some(ManagedTestRegistrationShutdownQuarantineClaim {
                    controller: Arc::clone(self),
                }))
            }
            ManagedTestRegistrationShutdownQuarantineState::Claimed
            | ManagedTestRegistrationShutdownQuarantineState::Retained => {
                Err("registration shutdown quarantine owner was already consumed")
            }
        }
    }

    pub(in super::super) fn retain_registration_shutdown_quarantine(
        &self,
        claim: ManagedTestRegistrationShutdownQuarantineClaim,
        retained: ManagedTestVfsRegistrationCustody,
    ) -> Result<(), &'static str> {
        if !std::ptr::eq(self, Arc::as_ptr(&claim.controller)) {
            let _retained = Box::leak(Box::new(retained));
            self.terminal.store(true, Ordering::SeqCst);
            return Err("registration shutdown quarantine claim owner mismatch");
        }

        // Custody becomes process-lifetime retained before either the witness or terminal bit can
        // claim success. A poisoned observer can therefore never release table/name/context.
        let _retained = Box::leak(Box::new(retained));
        let mut state = self.state.lock().map_err(|_| {
            self.terminal.store(true, Ordering::SeqCst);
            "registration shutdown quarantine owner poisoned"
        })?;
        if state.registration_shutdown_quarantine
            != ManagedTestRegistrationShutdownQuarantineState::Claimed
        {
            self.terminal.store(true, Ordering::SeqCst);
            return Err("registration shutdown quarantine claim is not live");
        }
        state.registration_shutdown_quarantine =
            ManagedTestRegistrationShutdownQuarantineState::Retained;
        self.terminal.store(true, Ordering::SeqCst);
        Ok(())
    }
}

impl ManagedTestRegistrationShutdownQuarantineWitness {
    pub(in super::super) fn retained(&self) -> Result<bool, &'static str> {
        let state = self
            .controller
            .state
            .lock()
            .map_err(|_| "registration shutdown quarantine owner poisoned")?;
        Ok(state.registration_shutdown_quarantine
            == ManagedTestRegistrationShutdownQuarantineState::Retained
            && self.controller.is_terminal())
    }
}

#[test]
fn registration_shutdown_quarantine_has_one_exact_custody_call_site() {
    let registration_source = include_str!("../../managed_vfs.rs");
    assert_eq!(
        registration_source
            .matches(".retain_registration_shutdown_quarantine(")
            .count(),
        1,
        "only unregister_in_place_with may transfer exact VFS custody to quarantine"
    );
}
