use std::collections::BTreeSet;

use super::{case_key::CaseKey, model::Case};

pub(super) fn validate(cases: &[Case]) -> Result<(), &'static str> {
    let mut actual = BTreeSet::new();
    for case in cases {
        if !actual.insert(CaseKey::from(case)) {
            return Err("duplicate A2b2 CaseKey");
        }
    }
    let expected = frozen_inventory();
    if expected.len() != 117 || actual != expected {
        return Err("A2b2 actual CaseKey set differs from the frozen 117-key inventory");
    }
    Ok(())
}

fn frozen_inventory() -> BTreeSet<CaseKey> {
    use CallbackKind::{Close, Shm};
    use FailureClass::{
        IoBeforeMutation as Io, MutatedButKnown as Mutated, None as NoFailure,
        OutcomeUncertainPoisoned as Uncertain, ProtocolViolation as Protocol, RegistrationRetained,
        RegistryRejected as Registry,
    };
    use NodePrecondition::{Absent, NotApplicable};
    use Path::{Barrier, JointClose, RegistrationShutdown, RegistryLifecycle, Unmap};
    use Phase::*;
    use Timing::{
        AfterSuccessKnown as After, AfterSuccessUncertain as AfterUncertain, BeforeCall as Before,
        NativeRetryable, NativeUncertain, Success as Succeeded, Validation,
    };
    use TopologyKind::{FinalConnection as Final, RegistrationOnly, SharedNonFinal as Shared};
    use UnmapMode::{Delete, Keep, NotApplicable as NoUnmap};

    let mut expected = BTreeSet::new();
    let mut add = |key| {
        expected.insert(key);
    };
    let k = CaseKey::expected;
    let shm = Some(Shm);
    let close = Some(Close);

    for (phase, timing, class) in [
        (CallbackAdmission, Before, Registry),
        (BarrierFence, Before, Io),
        (BarrierFence, AfterUncertain, Uncertain),
        (Success, Succeeded, NoFailure),
    ] {
        add(k(Barrier, Shared, NoUnmap, phase, timing, class, shm));
    }
    add(k(Barrier, Shared, NoUnmap, BarrierFence, Before, Io, shm).variant(1));
    for timing in [Before, NativeUncertain, After] {
        add(k(
            Barrier,
            Shared,
            NoUnmap,
            CallbackCompletion,
            timing,
            Registry,
            shm,
        ));
    }

    add(k(
        Unmap,
        Shared,
        Delete,
        RequestValidation,
        Validation,
        Protocol,
        shm,
    ));
    add(k(
        Unmap,
        Shared,
        Keep,
        CallbackAdmission,
        Before,
        Registry,
        shm,
    ));
    add(k(Unmap, Shared, Keep, ConnectionDetach, Before, Io, shm).variant(1));
    add(k(Unmap, Shared, Keep, HeldLockGate, Validation, Protocol, shm).masks(1, 0));
    add(k(Unmap, Shared, Keep, HeldLockGate, Validation, Protocol, shm).masks(0, 1));
    for (phase, timing, class) in [
        (ConnectionDetach, Before, Io),
        (ConnectionDetach, After, Mutated),
        (ConnectionDetach, AfterUncertain, Uncertain),
        (CallbackCompletion, NativeUncertain, Registry),
        (Success, Succeeded, NoFailure),
    ] {
        add(k(Unmap, Shared, Keep, phase, timing, class, shm));
    }
    add(k(Unmap, Shared, Delete, Success, Succeeded, NoFailure, shm));

    for (phase, before_class) in [
        (ViewUnmap, Io),
        (MappingClose, Mutated),
        (DmsSharedRelease, Mutated),
    ] {
        for (timing, class) in [
            (Before, before_class),
            (NativeUncertain, Uncertain),
            (After, Mutated),
            (AfterUncertain, Uncertain),
        ] {
            add(k(Unmap, Final, Keep, phase, timing, class, shm));
        }
    }
    for (timing, class) in [
        (Before, Mutated),
        (NativeRetryable, Uncertain),
        (NativeUncertain, Uncertain),
        (After, Mutated),
        (AfterUncertain, Uncertain),
    ] {
        add(k(Unmap, Final, Keep, ShmFileClose, timing, class, shm));
    }
    for (phase, timing, class) in [
        (ConnectionDetach, Before, Mutated),
        (ConnectionDetach, After, Mutated),
        (ConnectionDetach, AfterUncertain, Uncertain),
        (CallbackCompletion, NativeUncertain, Registry),
    ] {
        add(k(Unmap, Final, Keep, phase, timing, class, shm));
    }
    add(k(Unmap, Final, Keep, Success, Succeeded, NoFailure, shm));
    add(k(Unmap, Final, Keep, Success, Succeeded, NoFailure, shm).node(Absent));

    for variant in 1..=3 {
        add(k(
            Unmap,
            Final,
            Delete,
            DeleteAuthorization,
            Validation,
            Protocol,
            shm,
        )
        .variant(variant));
    }
    add(k(
        Unmap,
        Final,
        Delete,
        DeleteAuthorization,
        Validation,
        Uncertain,
        shm,
    )
    .variant(4));
    for (timing, class) in [
        (Before, Mutated),
        (NativeRetryable, Uncertain),
        (NativeUncertain, Uncertain),
        (After, Mutated),
        (AfterUncertain, Uncertain),
    ] {
        add(k(
            Unmap,
            Final,
            Delete,
            ExactSiblingDelete,
            timing,
            class,
            shm,
        ));
    }
    for (phase, timing) in [
        (ConnectionDetach, Before),
        (ConnectionDetach, After),
        (ConnectionDetach, AfterUncertain),
        (CallbackCompletion, NativeUncertain),
    ] {
        let class = if phase == CallbackCompletion {
            Registry
        } else if timing == AfterUncertain {
            Uncertain
        } else {
            Mutated
        };
        add(k(Unmap, Final, Delete, phase, timing, class, shm).variant(1));
    }
    add(k(Unmap, Final, Delete, Success, Succeeded, NoFailure, shm));
    add(k(Unmap, Final, Delete, Success, Succeeded, NoFailure, shm).variant(1));

    add(k(
        JointClose,
        Final,
        Keep,
        RawStateTake,
        Validation,
        Protocol,
        close,
    ));
    add(k(
        JointClose,
        Final,
        Keep,
        BeginConnectionClose,
        Before,
        Registry,
        close,
    ));
    add(k(
        JointClose,
        Final,
        Keep,
        CallbackAdmission,
        Before,
        Registry,
        close,
    ));
    add(k(JointClose, Final, Keep, MainFileClose, Before, Io, close).variant(1));
    for (cause, before_class) in [
        (ViewUnmap, Io),
        (MappingClose, Mutated),
        (DmsSharedRelease, Mutated),
    ] {
        for (timing, class) in [
            (Before, before_class),
            (NativeUncertain, Uncertain),
            (After, Mutated),
            (AfterUncertain, Uncertain),
        ] {
            add(k(JointClose, Final, Keep, ShmUnmapLift, timing, class, close).cause(cause));
        }
    }
    for (timing, class) in [
        (Before, Mutated),
        (NativeRetryable, Uncertain),
        (NativeUncertain, Uncertain),
        (After, Mutated),
        (AfterUncertain, Uncertain),
    ] {
        add(k(JointClose, Final, Keep, ShmUnmapLift, timing, class, close).cause(ShmFileClose));
    }
    for (cause, timing, class) in [
        (ConnectionDetach, Before, Mutated),
        (ConnectionDetach, After, Mutated),
        (ConnectionDetach, AfterUncertain, Uncertain),
    ] {
        add(k(JointClose, Final, Keep, ShmUnmapLift, timing, class, close).cause(cause));
    }
    for phase in [MainLockRelease, MainFileClose] {
        for (timing, class) in [
            (Before, Mutated),
            (NativeRetryable, Mutated),
            (NativeUncertain, Uncertain),
            (After, Mutated),
        ] {
            add(k(JointClose, Final, Keep, phase, timing, class, close));
        }
    }
    add(k(
        JointClose, Final, Keep, Success, Succeeded, NoFailure, close,
    ));
    for timing in [Before, NativeUncertain, After] {
        add(k(
            JointClose,
            Final,
            Keep,
            RegistryWalMainClose,
            timing,
            Registry,
            close,
        ));
    }

    for timing in [Before, NativeUncertain, After] {
        add(k(
            RegistryLifecycle,
            Final,
            Keep,
            CallbackCompletion,
            timing,
            Registry,
            close,
        ));
    }
    for (phase, timing, variant) in [
        (ConnectionObservation, Before, 0),
        (ConnectionObservation, Validation, 1),
        (ConnectionObservation, After, 0),
        (RegistryRouteRemoval, Before, 0),
        (RegistryRouteRemoval, NativeUncertain, 1),
        (RegistryRouteRemoval, NativeUncertain, 2),
        (RegistryRouteRemoval, After, 0),
        (LogicalRouteRemoval, Before, 0),
        (LogicalRouteRemoval, NativeUncertain, 1),
        (LogicalRouteRemoval, NativeUncertain, 2),
        (LogicalRouteRemoval, After, 0),
    ] {
        add(k(
            RegistryLifecycle,
            Final,
            Keep,
            phase,
            timing,
            Registry,
            close,
        )
        .variant(variant));
    }
    add(k(
        RegistryLifecycle,
        Shared,
        Keep,
        Success,
        Succeeded,
        NoFailure,
        close,
    ));
    add(k(
        RegistryLifecycle,
        Final,
        Keep,
        Success,
        Succeeded,
        NoFailure,
        close,
    ));

    for (phase, variant) in [
        (OutstandingCallbackGate, 1),
        (LiveRouteGate, 2),
        (QuarantinedCustodyGate, 3),
    ] {
        add(k(
            RegistrationShutdown,
            RegistrationOnly,
            NoUnmap,
            phase,
            Validation,
            RegistrationRetained,
            None,
        )
        .variant(variant)
        .node(NotApplicable));
    }
    add(k(
        RegistrationShutdown,
        RegistrationOnly,
        NoUnmap,
        RouteIndexObservation,
        NativeUncertain,
        RegistrationRetained,
        None,
    )
    .variant(4)
    .node(NotApplicable));
    for timing in [Before, NativeRetryable, After] {
        add(k(
            RegistrationShutdown,
            RegistrationOnly,
            NoUnmap,
            VfsUnregister,
            timing,
            RegistrationRetained,
            None,
        )
        .node(NotApplicable));
    }
    add(k(
        RegistrationShutdown,
        RegistrationOnly,
        NoUnmap,
        Success,
        Succeeded,
        NoFailure,
        None,
    )
    .node(NotApplicable));
    expected
}
