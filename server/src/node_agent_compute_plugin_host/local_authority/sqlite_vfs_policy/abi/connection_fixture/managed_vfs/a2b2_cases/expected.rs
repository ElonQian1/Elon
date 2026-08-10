use std::collections::BTreeSet;

use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::ManagedSqliteLogicalFileRole;

use super::model::{
    CallbackKind, Case, FailureClass, NodePrecondition, Path, Phase, TargetScope, Timing,
    TopologyKind, UnmapMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CaseKey {
    path: Path,
    topology: TopologyKind,
    unmap_mode: UnmapMode,
    node: NodePrecondition,
    variant: u8,
    shared_mask: u8,
    exclusive_mask: u8,
    phase: Phase,
    cause: Option<Phase>,
    timing: Timing,
    class: FailureClass,
    scope: TargetScope,
    registration_id: u64,
    route_ordinal: u64,
    runtime_generation: u64,
    shm_connection_id: u64,
    role: u8,
    callback: Option<CallbackKind>,
    occurrence: u32,
}

impl CaseKey {
    fn expected(
        path: Path,
        topology: TopologyKind,
        unmap_mode: UnmapMode,
        phase: Phase,
        timing: Timing,
        class: FailureClass,
        callback: Option<CallbackKind>,
    ) -> Self {
        let route_scoped = path != Path::RegistrationShutdown;
        Self {
            path,
            topology,
            unmap_mode,
            node: NodePrecondition::Live,
            variant: 0,
            shared_mask: 0,
            exclusive_mask: 0,
            phase,
            cause: None,
            timing,
            class,
            scope: if route_scoped {
                TargetScope::RouteMain
            } else {
                TargetScope::Registration
            },
            registration_id: 1,
            route_ordinal: u64::from(route_scoped),
            runtime_generation: u64::from(route_scoped),
            shm_connection_id: u64::from(route_scoped),
            role: u8::from(route_scoped),
            callback,
            occurrence: 1,
        }
    }

    fn cause(mut self, phase: Phase) -> Self {
        self.cause = Some(phase);
        self
    }

    fn variant(mut self, variant: u8) -> Self {
        self.variant = variant;
        self
    }

    fn masks(mut self, shared: u8, exclusive: u8) -> Self {
        self.shared_mask = shared;
        self.exclusive_mask = exclusive;
        self
    }

    fn node(mut self, node: NodePrecondition) -> Self {
        self.node = node;
        self
    }
}

impl From<&Case> for CaseKey {
    fn from(case: &Case) -> Self {
        Self {
            path: case.path,
            topology: case.topology_kind,
            unmap_mode: case.unmap_mode,
            node: case.node_precondition,
            variant: case.variant,
            shared_mask: case.pre_shared_mask,
            exclusive_mask: case.pre_exclusive_mask,
            phase: case.phase,
            cause: case.cause_phase,
            timing: case.timing,
            class: case.class,
            scope: case.target.scope,
            registration_id: case.target.registration_id,
            route_ordinal: case.target.route_ordinal,
            runtime_generation: case.target.runtime_generation,
            shm_connection_id: case.target.shm_connection_id,
            role: match case.target.role {
                None => 0,
                Some(ManagedSqliteLogicalFileRole::Main) => 1,
                Some(ManagedSqliteLogicalFileRole::Journal) => 2,
                Some(ManagedSqliteLogicalFileRole::Wal) => 3,
            },
            callback: case.target.callback,
            occurrence: case.target.occurrence,
        }
    }
}

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
