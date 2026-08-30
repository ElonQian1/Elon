use anyhow::anyhow;

use super::{
    BoundaryEvidence, BoundaryProjection, Cause, Class, Held, LifePhase, LifeTiming, Logical,
    NativeEvidence, NativeObservation, Offset, Phase, Prestate, Registry, ShmBoundary, ShmClass,
    ShmNative, ShmPhase, Timing, S,
};

pub(super) fn project(evidence: &BoundaryEvidence<'_>) -> anyhow::Result<BoundaryProjection> {
    match evidence.selector {
        S::RawStateTakeRejected => Ok(active(
            Phase::RawStateTake,
            Timing::Validation,
            Class::ProtocolViolation,
        )),
        S::BeginConnectionCloseRejected => Ok(route_terminal(
            Phase::BeginConnectionClose,
            Timing::BeforeCall,
            Class::RegistryRejected,
            false,
        )),
        S::CallbackAdmissionRejected => Ok(route_terminal(
            Phase::CallbackAdmission,
            Timing::BeforeCall,
            Class::RegistryRejected,
            false,
        )),
        S::CallbackWrapperBefore => {
            let mut value = route_terminal(
                Phase::MainFileClose,
                Timing::BeforeCall,
                Class::IoBeforeMutation,
                false,
            );
            value.variant = 1;
            Ok(value)
        }
        selector if super::is_shm(selector) => project_shm(evidence),
        selector if super::is_main(selector) => project_main(evidence),
        S::PhysicalSuccess => Ok(BoundaryProjection {
            phase: Phase::Success,
            cause: Cause::None,
            timing: Timing::Success,
            class: Class::None,
            variant: 0,
            main_lock_prestate: Prestate::NotApplicable,
            main_lock_offset_class: Offset::NotApplicable,
            mutation_may_have_occurred: false,
            lock_outcome_uncertain: false,
            domain_terminal: false,
            registry_route_phase: Registry::Closing,
            logical_route_phase: Logical::Indexed,
            later_callback_allowed: true,
        }),
        selector if super::is_registry(selector) => project_registry(evidence),
        _ => Err(anyhow!(
            "JointClose boundary projection selector is not frozen"
        )),
    }
}

fn project_shm(evidence: &BoundaryEvidence<'_>) -> anyhow::Result<BoundaryProjection> {
    let observed = evidence
        .shm
        .ok_or_else(|| anyhow!("JointClose SHM sealed receipt is absent"))?;
    if !shm_selector_matches(evidence.selector, observed.phase, observed.boundary) {
        return Err(anyhow!(
            "JointClose SHM receipt differs from selected boundary"
        ));
    }
    let cause = match observed.phase {
        ShmPhase::ViewUnmap => Cause::ViewUnmap,
        ShmPhase::MappingClose => Cause::MappingClose,
        ShmPhase::DmsSharedRelease => Cause::DmsSharedRelease,
        ShmPhase::FileClose => Cause::ShmFileClose,
        ShmPhase::ConnectionDetach => Cause::ConnectionDetach,
        _ => return Err(anyhow!("JointClose SHM receipt phase is outside close")),
    };
    let (timing, class, uncertain) = match observed.boundary {
        ShmBoundary::Before => (
            Timing::BeforeCall,
            if observed.phase == ShmPhase::ViewUnmap {
                Class::IoBeforeMutation
            } else {
                Class::MutatedButKnown
            },
            false,
        ),
        ShmBoundary::Native(operation) => (
            if operation == ShmNative::FileCloseRetryable {
                Timing::NativeRetryable
            } else {
                Timing::NativeUncertain
            },
            Class::OutcomeUncertainPoisoned,
            true,
        ),
        ShmBoundary::After(class) => (
            if class == ShmClass::MutatedButKnown {
                Timing::AfterSuccessKnown
            } else {
                Timing::AfterSuccessUncertain
            },
            if class == ShmClass::MutatedButKnown {
                Class::MutatedButKnown
            } else {
                Class::OutcomeUncertainPoisoned
            },
            class == ShmClass::OutcomeUncertainPoisoned,
        ),
    };
    let mutation = observed.phase != ShmPhase::ViewUnmap || timing != Timing::BeforeCall;
    Ok(terminal(
        Phase::ShmUnmapLift,
        cause,
        timing,
        class,
        mutation,
        observed.phase == ShmPhase::DmsSharedRelease && uncertain,
        mutation || uncertain,
    ))
}

fn project_main(evidence: &BoundaryEvidence<'_>) -> anyhow::Result<BoundaryProjection> {
    if let Some(native) = evidence.control.and_then(|value| value.evidence()) {
        let (phase, timing, class, prestate, offset, lock_uncertain) = match native {
            NativeEvidence::MainLockRelease {
                held_range_prestate,
                selected_offset_class,
                exact_call_occurrence: _,
                observation: NativeObservation::ReturnReceiptUnavailable,
            } => (
                Phase::MainLockRelease,
                Timing::NativeUncertain,
                Class::OutcomeUncertainPoisoned,
                match held_range_prestate {
                    Held::Shared => Prestate::Shared,
                    Held::ReservedShared => Prestate::ReservedShared,
                },
                match selected_offset_class {
                    super::NativeOffset::SharedRange => Offset::SharedRange,
                    super::NativeOffset::ReservedByte => Offset::ReservedByte,
                },
                true,
            ),
            NativeEvidence::MainFileClose {
                exact_call_occurrence: _,
                observation,
            } => (
                Phase::MainFileClose,
                if observation == NativeObservation::NativeFailureObserved {
                    Timing::NativeRetryable
                } else {
                    Timing::NativeUncertain
                },
                if observation == NativeObservation::NativeFailureObserved {
                    Class::MutatedButKnown
                } else {
                    Class::OutcomeUncertainPoisoned
                },
                Prestate::NotApplicable,
                Offset::NotApplicable,
                false,
            ),
            _ => return Err(anyhow!("JointClose main native evidence is not canonical")),
        };
        let mut value = terminal(
            phase,
            Cause::None,
            timing,
            class,
            true,
            lock_uncertain,
            false,
        );
        value.main_lock_prestate = prestate;
        value.main_lock_offset_class = offset;
        value.variant = u8::from(prestate == Prestate::ReservedShared);
        return Ok(value);
    }
    let selected = evidence
        .lifecycle
        .iter()
        .find(|value| value.triggered)
        .ok_or_else(|| anyhow!("JointClose main generic trigger is absent"))?;
    let phase = match selected.phase {
        LifePhase::MainUnlock => Phase::MainLockRelease,
        LifePhase::MainFileClose => Phase::MainFileClose,
        _ => return Err(anyhow!("JointClose main trigger phase is foreign")),
    };
    let timing = match selected.timing {
        LifeTiming::BeforeCall => Timing::BeforeCall,
        LifeTiming::AfterSuccess => Timing::AfterSuccessKnown,
        LifeTiming::NativeFailure => return Err(anyhow!("generic main trigger is native")),
    };
    Ok(terminal(
        phase,
        Cause::None,
        timing,
        Class::MutatedButKnown,
        true,
        false,
        false,
    ))
}

fn project_registry(evidence: &BoundaryEvidence<'_>) -> anyhow::Result<BoundaryProjection> {
    let selected = evidence
        .lifecycle
        .iter()
        .find(|value| {
            value.phase == LifePhase::RegistryWalMainClose
                && (value.triggered || value.timing == LifeTiming::NativeFailure)
        })
        .ok_or_else(|| anyhow!("JointClose registry selected receipt is absent"))?;
    let timing = match selected.timing {
        LifeTiming::BeforeCall => Timing::BeforeCall,
        LifeTiming::AfterSuccess => Timing::AfterSuccessKnown,
        LifeTiming::NativeFailure => Timing::NativeUncertain,
    };
    Ok(terminal(
        Phase::RegistryWalMainClose,
        Cause::None,
        timing,
        Class::RegistryRejected,
        true,
        false,
        false,
    ))
}

fn active(phase: Phase, timing: Timing, class: Class) -> BoundaryProjection {
    BoundaryProjection {
        phase,
        cause: Cause::None,
        timing,
        class,
        variant: 0,
        main_lock_prestate: Prestate::NotApplicable,
        main_lock_offset_class: Offset::NotApplicable,
        mutation_may_have_occurred: false,
        lock_outcome_uncertain: false,
        domain_terminal: false,
        registry_route_phase: Registry::Active,
        logical_route_phase: Logical::Indexed,
        later_callback_allowed: true,
    }
}

fn route_terminal(
    phase: Phase,
    timing: Timing,
    class: Class,
    mutation: bool,
) -> BoundaryProjection {
    terminal(phase, Cause::None, timing, class, mutation, false, false)
}

fn terminal(
    phase: Phase,
    cause: Cause,
    timing: Timing,
    class: Class,
    mutation: bool,
    lock_uncertain: bool,
    domain_terminal: bool,
) -> BoundaryProjection {
    BoundaryProjection {
        phase,
        cause,
        timing,
        class,
        variant: 0,
        main_lock_prestate: Prestate::NotApplicable,
        main_lock_offset_class: Offset::NotApplicable,
        mutation_may_have_occurred: mutation,
        lock_outcome_uncertain: lock_uncertain,
        domain_terminal,
        registry_route_phase: Registry::TerminalQuarantine,
        logical_route_phase: Logical::Retained,
        later_callback_allowed: false,
    }
}

fn shm_selector_matches(selector: S, phase: ShmPhase, boundary: ShmBoundary) -> bool {
    use ShmBoundary::{After, Before, Native};
    let expected = match selector {
        S::ShmViewUnmapBefore => (ShmPhase::ViewUnmap, Before),
        S::ShmViewUnmapNativeUncertain => (
            ShmPhase::ViewUnmap,
            Native(ShmNative::ViewUnmapOutcomeUncertain),
        ),
        S::ShmViewUnmapAfterKnown => (ShmPhase::ViewUnmap, After(ShmClass::MutatedButKnown)),
        S::ShmViewUnmapAfterUncertain => (
            ShmPhase::ViewUnmap,
            After(ShmClass::OutcomeUncertainPoisoned),
        ),
        S::ShmMappingCloseBefore => (ShmPhase::MappingClose, Before),
        S::ShmMappingCloseNativeUncertain => (
            ShmPhase::MappingClose,
            Native(ShmNative::MappingCloseOutcomeUncertain),
        ),
        S::ShmMappingCloseAfterKnown => (ShmPhase::MappingClose, After(ShmClass::MutatedButKnown)),
        S::ShmMappingCloseAfterUncertain => (
            ShmPhase::MappingClose,
            After(ShmClass::OutcomeUncertainPoisoned),
        ),
        S::ShmDmsReleaseBefore => (ShmPhase::DmsSharedRelease, Before),
        S::ShmDmsReleaseNativeUncertain => (
            ShmPhase::DmsSharedRelease,
            Native(ShmNative::DmsSharedReleaseOutcomeUncertain),
        ),
        S::ShmDmsReleaseAfterKnown => {
            (ShmPhase::DmsSharedRelease, After(ShmClass::MutatedButKnown))
        }
        S::ShmDmsReleaseAfterUncertain => (
            ShmPhase::DmsSharedRelease,
            After(ShmClass::OutcomeUncertainPoisoned),
        ),
        S::ShmFileCloseBefore => (ShmPhase::FileClose, Before),
        S::ShmFileCloseNativeRetryable => {
            (ShmPhase::FileClose, Native(ShmNative::FileCloseRetryable))
        }
        S::ShmFileCloseNativeUncertain => (
            ShmPhase::FileClose,
            Native(ShmNative::FileCloseOutcomeUncertain),
        ),
        S::ShmFileCloseAfterKnown => (ShmPhase::FileClose, After(ShmClass::MutatedButKnown)),
        S::ShmFileCloseAfterUncertain => (
            ShmPhase::FileClose,
            After(ShmClass::OutcomeUncertainPoisoned),
        ),
        S::ShmDetachBefore => (ShmPhase::ConnectionDetach, Before),
        S::ShmDetachAfterKnown => (ShmPhase::ConnectionDetach, After(ShmClass::MutatedButKnown)),
        S::ShmDetachAfterUncertain => (
            ShmPhase::ConnectionDetach,
            After(ShmClass::OutcomeUncertainPoisoned),
        ),
        _ => return false,
    };
    (phase, boundary) == expected
}
