use anyhow::anyhow;

use super::{BoundaryProjection, SealedJointCloseBoundary};
mod custody;
mod project;
use crate::{
    node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
        abi::connection_fixture::managed_vfs::{
            a2b2_cases::{
                JointCloseCause as Cause, JointCloseFailureClass as Class,
                JointCloseLogicalRoutePhase as Logical, JointCloseMainLockOffsetClass as Offset,
                JointCloseMainLockPrestate as Prestate, JointClosePhase as Phase,
                JointCloseRegistryRoutePhase as Registry, JointCloseSelector as S,
                JointCloseTiming as Timing,
            },
            joint_close_harness::shm::{ShmBoundary, ShmObserved},
            ManagedTestCallbackFaultObservation, ManagedTestCallbackFaultOperation,
            ManagedTestCallbackFaultTiming, ManagedTestJointCloseControl,
            ManagedTestJointCloseControlSnapshot, ManagedTestLifecycleFaultObservation,
            ManagedTestLifecycleFaultPhase as LifePhase,
            ManagedTestLifecycleFaultTiming as LifeTiming, ManagedTestRouteOrdinal,
        },
        registry::{
            ManagedSqliteRegistryLifecycleStage as Stage,
            ManagedSqliteRegistryTerminalCustodyTestSnapshot,
        },
        ManagedSqliteLogicalFileRole,
    },
    node_agent_managed_fs::{
        ManagedSqliteMainCloseTestNativeEvidence as NativeEvidence,
        ManagedSqliteMainCloseTestNativeObservation as NativeObservation,
        ManagedSqliteMainCloseTestNativeRequest as NativeRequest,
        ManagedSqliteMainLockHeldRangePrestate as Held,
        ManagedSqliteMainLockOffsetClass as NativeOffset, ManagedSqliteShmFailureClass as ShmClass,
        ManagedSqliteShmFailurePhase as ShmPhase,
        ManagedSqliteShmTestUnmapNativeOperation as ShmNative,
    },
};
use project::project;

pub(in super::super) struct BoundaryEvidence<'a> {
    pub(in super::super) selector: S,
    pub(in super::super) code: i32,
    pub(in super::super) route: ManagedTestRouteOrdinal,
    pub(in super::super) callbacks: &'a [ManagedTestCallbackFaultObservation],
    pub(in super::super) lifecycle: &'a [ManagedTestLifecycleFaultObservation],
    pub(in super::super) stages: &'a [Stage],
    pub(in super::super) control: Option<ManagedTestJointCloseControlSnapshot>,
    pub(in super::super) shm: Option<ShmObserved>,
    pub(in super::super) custody: ManagedSqliteRegistryTerminalCustodyTestSnapshot,
    pub(in super::super) callback_claims: Option<usize>,
    pub(in super::super) registry_claims: Option<usize>,
    pub(in super::super) begin_claims: Option<usize>,
    pub(in super::super) callback_pending: usize,
    pub(in super::super) lifecycle_pending: usize,
    pub(in super::super) generic_pending: usize,
}

pub(in super::super) fn seal(
    evidence: BoundaryEvidence<'_>,
) -> anyhow::Result<SealedJointCloseBoundary> {
    if evidence.callback_pending != 0
        || evidence.lifecycle_pending != 0
        || evidence.generic_pending != 0
    {
        return Err(anyhow!(
            "JointClose selected one-shot evidence remains pending: callback={} lifecycle={} generic={} stages={:?} lifecycle_observations={:?} terminal_custody={:?}",
            evidence.callback_pending,
            evidence.lifecycle_pending,
            evidence.generic_pending,
            evidence.stages,
            evidence.lifecycle,
            evidence.custody,
        ));
    }
    validate_trace(evidence.selector, evidence.stages)?;
    validate_callback(&evidence)?;
    validate_lifecycle(&evidence)?;
    validate_control(&evidence)?;
    custody::validate(&evidence)?;
    let projection = project(&evidence)?;
    SealedJointCloseBoundary::new(evidence.selector, evidence.code, projection)
}

fn validate_trace(selector: S, actual: &[Stage]) -> anyhow::Result<()> {
    use Stage as E;
    let expected: &[Stage] = match selector {
        S::RawStateTakeRejected | S::CallbackWrapperBefore => &[],
        S::BeginConnectionCloseRejected | S::CallbackAdmissionRejected => &[E::RawCloseEntered],
        selector if is_shm(selector) || is_main(selector) => &[
            E::RawCloseEntered,
            E::CallbackBegin,
            E::CallbackCompletionAttempt,
        ],
        S::PhysicalSuccess => &[E::RawCloseEntered, E::CallbackBegin],
        S::RegistryWalMainCloseBefore => &[
            E::RawCloseEntered,
            E::CallbackBegin,
            E::PhysicalCloseSucceeded,
            E::CallbackCompletionAttempt,
            E::CallbackCompletionSucceeded,
        ],
        S::RegistryWalMainCloseNativeUncertain => &[
            E::RawCloseEntered,
            E::CallbackBegin,
            E::PhysicalCloseSucceeded,
            E::RegistryWalMainCloseAttempt,
            E::CallbackCompletionAttempt,
        ],
        S::RegistryWalMainCloseAfterKnown => &[
            E::RawCloseEntered,
            E::CallbackBegin,
            E::PhysicalCloseSucceeded,
            E::RegistryWalMainCloseAttempt,
            E::RegistryWalMainCloseSucceeded,
            E::CallbackCompletionAttempt,
            E::CallbackCompletionSucceeded,
        ],
        _ => return Err(anyhow!("JointClose trace selector is not frozen")),
    };
    if actual != expected {
        return Err(anyhow!("JointClose registry stage trace is not exact"));
    }
    Ok(())
}

fn validate_callback(evidence: &BoundaryEvidence<'_>) -> anyhow::Result<()> {
    if evidence.selector != S::CallbackWrapperBefore {
        if evidence.callbacks.is_empty() {
            return Ok(());
        }
        return Err(anyhow!(
            "JointClose observed an unselected callback-wrapper fault"
        ));
    }
    let [observation] = evidence.callbacks else {
        return Err(anyhow!(
            "JointClose callback-wrapper receipt is not exact-once"
        ));
    };
    let step = observation.step();
    if step.route_ordinal() != evidence.route
        || step.role() != ManagedSqliteLogicalFileRole::Main
        || step.operation() != ManagedTestCallbackFaultOperation::FileClose
        || step.occurrence() != 1
        || step.timing() != ManagedTestCallbackFaultTiming::BeforeCall
    {
        return Err(anyhow!(
            "JointClose callback-wrapper key escaped its exact route"
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct LifeSpec(LifePhase, LifeTiming, bool);

fn validate_lifecycle(evidence: &BoundaryEvidence<'_>) -> anyhow::Result<()> {
    let expected = expected_lifecycle(evidence.selector);
    if evidence.lifecycle.len() != expected.len()
        || evidence
            .lifecycle
            .iter()
            .zip(expected.iter().copied())
            .any(|(actual, expected)| {
                actual.route != Some(evidence.route)
                    || actual.phase != expected.0
                    || actual.occurrence != 1
                    || actual.timing != expected.1
                    || actual.triggered != expected.2
            })
    {
        return Err(anyhow!(
            "JointClose lifecycle receipt sequence is not exact: actual={:?} expected={expected:?}",
            evidence.lifecycle,
        ));
    }
    Ok(())
}

fn expected_lifecycle(selector: S) -> Vec<LifeSpec> {
    use LifePhase::{MainFileClose as File, MainUnlock as Unlock, RegistryWalMainClose as Wal};
    use LifeTiming::{AfterSuccess as After, BeforeCall as Before, NativeFailure as Native};
    match selector {
        selector if is_shm(selector) => with_callback_completion(Vec::new(), Native),
        S::MainLockReleaseBefore => {
            with_callback_completion(vec![LifeSpec(Unlock, Before, true)], Native)
        }
        S::MainLockReleaseNativeUncertainShared | S::MainLockReleaseNativeUncertainReserved => {
            with_callback_completion(vec![LifeSpec(Unlock, Before, false)], Native)
        }
        S::MainLockReleaseAfterKnown => with_callback_completion(
            vec![
                LifeSpec(Unlock, Before, false),
                LifeSpec(Unlock, After, true),
            ],
            Native,
        ),
        S::MainFileCloseBefore => with_callback_completion(
            vec![
                LifeSpec(Unlock, Before, false),
                LifeSpec(Unlock, After, false),
                LifeSpec(File, Before, true),
            ],
            Native,
        ),
        S::MainFileCloseNativeRetryable | S::MainFileCloseNativeUncertain => {
            with_callback_completion(
                vec![
                    LifeSpec(Unlock, Before, false),
                    LifeSpec(Unlock, After, false),
                    LifeSpec(File, Before, false),
                ],
                Native,
            )
        }
        S::MainFileCloseAfterKnown => with_callback_completion(
            vec![
                LifeSpec(Unlock, Before, false),
                LifeSpec(Unlock, After, false),
                LifeSpec(File, Before, false),
                LifeSpec(File, After, true),
            ],
            Native,
        ),
        S::PhysicalSuccess => main_success_specs(),
        S::RegistryWalMainCloseBefore => {
            with_callback_completion(with_wal(LifeSpec(Wal, Before, true)), After)
        }
        S::RegistryWalMainCloseNativeUncertain => {
            let mut specs = with_wal(LifeSpec(Wal, Before, false));
            specs.push(LifeSpec(Wal, Native, false));
            with_callback_completion(specs, Native)
        }
        S::RegistryWalMainCloseAfterKnown => {
            let mut specs = with_wal(LifeSpec(Wal, Before, false));
            specs.push(LifeSpec(Wal, After, true));
            with_callback_completion(specs, After)
        }
        _ => Vec::new(),
    }
}

fn with_callback_completion(mut specs: Vec<LifeSpec>, completion: LifeTiming) -> Vec<LifeSpec> {
    specs.push(LifeSpec(
        LifePhase::CallbackCompletion,
        LifeTiming::BeforeCall,
        false,
    ));
    specs.push(LifeSpec(LifePhase::CallbackCompletion, completion, false));
    specs
}

fn main_success_specs() -> Vec<LifeSpec> {
    use LifePhase::{MainFileClose as File, MainUnlock as Unlock};
    use LifeTiming::{AfterSuccess as After, BeforeCall as Before};
    vec![
        LifeSpec(Unlock, Before, false),
        LifeSpec(Unlock, After, false),
        LifeSpec(File, Before, false),
        LifeSpec(File, After, false),
    ]
}

fn with_wal(wal: LifeSpec) -> Vec<LifeSpec> {
    let mut specs = main_success_specs();
    specs.push(wal);
    specs
}

fn validate_control(evidence: &BoundaryEvidence<'_>) -> anyhow::Result<()> {
    let selected = evidence.control;
    let valid = match evidence.selector {
        S::BeginConnectionCloseRejected => matches!(selected, Some(snapshot)
            if snapshot.control() == ManagedTestJointCloseControl::BeginConnectionCloseRejected
                && snapshot.claimed() && snapshot.evidence().is_none()
                && snapshot.pending_count() == 0
                && evidence.begin_claims == Some(1)
                && evidence.callback_claims.is_none()
                && evidence.registry_claims.is_none()),
        S::CallbackAdmissionRejected => matches!(selected, Some(snapshot)
            if snapshot.control() == ManagedTestJointCloseControl::CallbackAdmissionRejected
                && snapshot.claimed() && snapshot.evidence().is_none()
                && snapshot.pending_count() == 0
                && evidence.begin_claims.is_none()
                && evidence.callback_claims == Some(1)
                && evidence.registry_claims.is_none()),
        S::MainLockReleaseNativeUncertainShared => main_native(
            selected,
            NativeRequest::MainLockReleaseNativeUncertainShared,
            Held::Shared,
            NativeOffset::SharedRange,
            NativeObservation::ReturnReceiptUnavailable,
        ),
        S::MainLockReleaseNativeUncertainReserved => main_native(
            selected,
            NativeRequest::MainLockReleaseNativeUncertainReserved,
            Held::ReservedShared,
            NativeOffset::ReservedByte,
            NativeObservation::ReturnReceiptUnavailable,
        ),
        S::MainFileCloseNativeRetryable => file_native(
            selected,
            NativeRequest::MainFileCloseNativeRetryable,
            NativeObservation::NativeFailureObserved,
        ),
        S::MainFileCloseNativeUncertain => file_native(
            selected,
            NativeRequest::MainFileCloseNativeUncertain,
            NativeObservation::ReturnReceiptUnavailable,
        ),
        S::PhysicalSuccess => matches!(selected, Some(snapshot)
            if snapshot.control() == ManagedTestJointCloseControl::PhysicalSuccessHandoff
                && snapshot.claimed() && snapshot.evidence().is_none()
                && snapshot.pending_count() == 0),
        S::RegistryWalMainCloseNativeUncertain => matches!(selected, Some(snapshot)
            if snapshot.control() == ManagedTestJointCloseControl::RegistryWalMainNativeUncertain
                && snapshot.claimed() && snapshot.evidence().is_none()
                && snapshot.pending_count() == 0
                && evidence.registry_claims == Some(1)
                && evidence.callback_claims.is_none()),
        _ => {
            selected.is_none()
                && evidence.begin_claims.is_none()
                && evidence.callback_claims.is_none()
                && evidence.registry_claims.is_none()
        }
    };
    if !valid {
        return Err(anyhow!(
            "JointClose route-bound claim or typed native evidence is not exact"
        ));
    }
    Ok(())
}

fn main_native(
    selected: Option<ManagedTestJointCloseControlSnapshot>,
    request: NativeRequest,
    held: Held,
    offset: NativeOffset,
    observation: NativeObservation,
) -> bool {
    matches!(selected, Some(snapshot)
        if snapshot.control() == ManagedTestJointCloseControl::MainNative(request)
            && snapshot.claimed() && snapshot.pending_count() == 0
            && matches!(snapshot.evidence(), Some(NativeEvidence::MainLockRelease {
                held_range_prestate,
                selected_offset_class,
                exact_call_occurrence,
                observation: actual,
            }) if held_range_prestate == held && selected_offset_class == offset
                && exact_call_occurrence.get() == 1 && actual == observation))
}

fn file_native(
    selected: Option<ManagedTestJointCloseControlSnapshot>,
    request: NativeRequest,
    observation: NativeObservation,
) -> bool {
    matches!(selected, Some(snapshot)
        if snapshot.control() == ManagedTestJointCloseControl::MainNative(request)
            && snapshot.claimed() && snapshot.pending_count() == 0
            && matches!(snapshot.evidence(), Some(NativeEvidence::MainFileClose {
                exact_call_occurrence,
                observation: actual,
            }) if exact_call_occurrence.get() == 1 && actual == observation))
}

fn is_shm(selector: S) -> bool {
    matches!(
        selector,
        S::ShmViewUnmapBefore
            | S::ShmViewUnmapNativeUncertain
            | S::ShmViewUnmapAfterKnown
            | S::ShmViewUnmapAfterUncertain
            | S::ShmMappingCloseBefore
            | S::ShmMappingCloseNativeUncertain
            | S::ShmMappingCloseAfterKnown
            | S::ShmMappingCloseAfterUncertain
            | S::ShmDmsReleaseBefore
            | S::ShmDmsReleaseNativeUncertain
            | S::ShmDmsReleaseAfterKnown
            | S::ShmDmsReleaseAfterUncertain
            | S::ShmFileCloseBefore
            | S::ShmFileCloseNativeRetryable
            | S::ShmFileCloseNativeUncertain
            | S::ShmFileCloseAfterKnown
            | S::ShmFileCloseAfterUncertain
            | S::ShmDetachBefore
            | S::ShmDetachAfterKnown
            | S::ShmDetachAfterUncertain
    )
}

fn is_main(selector: S) -> bool {
    matches!(
        selector,
        S::MainLockReleaseBefore
            | S::MainLockReleaseNativeUncertainShared
            | S::MainLockReleaseNativeUncertainReserved
            | S::MainLockReleaseAfterKnown
            | S::MainFileCloseBefore
            | S::MainFileCloseNativeRetryable
            | S::MainFileCloseNativeUncertain
            | S::MainFileCloseAfterKnown
    )
}

fn is_registry(selector: S) -> bool {
    matches!(
        selector,
        S::RegistryWalMainCloseBefore
            | S::RegistryWalMainCloseNativeUncertain
            | S::RegistryWalMainCloseAfterKnown
    )
}
