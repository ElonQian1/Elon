//! Sealed normalized identity produced only from an observed registration shutdown branch.

use super::super::{
    a2b2_cases::{
        RegistrationShutdownActualIdentity, RegistrationShutdownActualTarget,
        RegistrationShutdownFailureClass, RegistrationShutdownPhase, RegistrationShutdownSelector,
        RegistrationShutdownTiming,
    },
    ManagedTestRegistrationShutdownTargetWitness, ManagedTestVfsRetainedPartsSnapshot,
};

pub(in super::super) struct ObservedRegistrationShutdownOutcome {
    kind: ObservedRegistrationShutdownOutcomeKind,
}

#[derive(Clone, Copy)]
enum ObservedRegistrationShutdownOutcomeKind {
    OutstandingCallback,
    LiveRoute,
    QuarantinedCustody,
    RouteIndex,
    BeforeCall,
    InjectedPreNativeRetryable,
    AfterSuccess,
    Success,
}

impl ObservedRegistrationShutdownOutcome {
    fn new(kind: ObservedRegistrationShutdownOutcomeKind) -> Self {
        Self { kind }
    }

    pub(super) fn outstanding_callback() -> Self {
        Self::new(ObservedRegistrationShutdownOutcomeKind::OutstandingCallback)
    }

    pub(super) fn live_route() -> Self {
        Self::new(ObservedRegistrationShutdownOutcomeKind::LiveRoute)
    }

    pub(super) fn quarantined_custody() -> Self {
        Self::new(ObservedRegistrationShutdownOutcomeKind::QuarantinedCustody)
    }

    pub(super) fn route_index() -> Self {
        Self::new(ObservedRegistrationShutdownOutcomeKind::RouteIndex)
    }

    pub(super) fn before_call() -> Self {
        Self::new(ObservedRegistrationShutdownOutcomeKind::BeforeCall)
    }

    pub(super) fn injected_pre_native_retryable() -> Self {
        Self::new(ObservedRegistrationShutdownOutcomeKind::InjectedPreNativeRetryable)
    }

    pub(super) fn after_success() -> Self {
        Self::new(ObservedRegistrationShutdownOutcomeKind::AfterSuccess)
    }

    pub(super) fn success() -> Self {
        Self::new(ObservedRegistrationShutdownOutcomeKind::Success)
    }

    pub(in super::super) fn selector(&self) -> RegistrationShutdownSelector {
        use ObservedRegistrationShutdownOutcomeKind as Kind;
        match self.kind {
            Kind::OutstandingCallback => RegistrationShutdownSelector::OutstandingCallbackGate,
            Kind::LiveRoute => RegistrationShutdownSelector::LiveRouteGate,
            Kind::QuarantinedCustody => RegistrationShutdownSelector::QuarantinedCustodyGate,
            Kind::RouteIndex => RegistrationShutdownSelector::RouteIndexObservation,
            Kind::BeforeCall => RegistrationShutdownSelector::VfsUnregisterBeforeCall,
            Kind::InjectedPreNativeRetryable => {
                RegistrationShutdownSelector::VfsUnregisterNativeRetryable
            }
            Kind::AfterSuccess => RegistrationShutdownSelector::VfsUnregisterAfterSuccessKnown,
            Kind::Success => RegistrationShutdownSelector::Success,
        }
    }

    pub(in super::super) fn is_success(&self) -> bool {
        matches!(self.kind, ObservedRegistrationShutdownOutcomeKind::Success)
    }

    pub(in super::super) fn into_identity(
        self,
        target: ManagedTestRegistrationShutdownTargetWitness,
        retained: Option<ManagedTestVfsRetainedPartsSnapshot>,
    ) -> RegistrationShutdownActualIdentity {
        use ObservedRegistrationShutdownOutcomeKind as Kind;
        let (variant, phase, timing) = match self.kind {
            Kind::OutstandingCallback => (
                1,
                RegistrationShutdownPhase::OutstandingCallbackGate,
                RegistrationShutdownTiming::Validation,
            ),
            Kind::LiveRoute => (
                2,
                RegistrationShutdownPhase::LiveRouteGate,
                RegistrationShutdownTiming::Validation,
            ),
            Kind::QuarantinedCustody => (
                3,
                RegistrationShutdownPhase::QuarantinedCustodyGate,
                RegistrationShutdownTiming::Validation,
            ),
            Kind::RouteIndex => (
                4,
                RegistrationShutdownPhase::RouteIndexObservation,
                RegistrationShutdownTiming::NativeUncertain,
            ),
            Kind::BeforeCall => (
                0,
                RegistrationShutdownPhase::VfsUnregister,
                RegistrationShutdownTiming::BeforeCall,
            ),
            Kind::InjectedPreNativeRetryable => (
                0,
                RegistrationShutdownPhase::VfsUnregister,
                RegistrationShutdownTiming::NativeRetryable,
            ),
            Kind::AfterSuccess => (
                0,
                RegistrationShutdownPhase::VfsUnregister,
                RegistrationShutdownTiming::AfterSuccessKnown,
            ),
            Kind::Success => (
                0,
                RegistrationShutdownPhase::Success,
                RegistrationShutdownTiming::Success,
            ),
        };
        RegistrationShutdownActualIdentity {
            path_is_registration_shutdown: true,
            topology_is_registration_only: true,
            unmap_is_not_applicable: true,
            node_is_not_applicable: true,
            variant,
            pre_shared_mask: 0,
            pre_exclusive_mask: 0,
            phase,
            cause_phase_is_none: true,
            timing,
            class: if retained.is_some() {
                RegistrationShutdownFailureClass::RegistrationRetained
            } else {
                RegistrationShutdownFailureClass::None
            },
            target: RegistrationShutdownActualTarget {
                scope_is_registration: true,
                registration_id: target.registration_id(),
                route_ordinal_is_not_applicable: true,
                runtime_generation_is_not_applicable: true,
                shm_connection_id_is_not_applicable: true,
                role_is_none: true,
                callback_is_none: true,
                occurrence: target.occurrence(),
            },
            sqlite_outcome_is_not_applicable: true,
        }
    }
}
