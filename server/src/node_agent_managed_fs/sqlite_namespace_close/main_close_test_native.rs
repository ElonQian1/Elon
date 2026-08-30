use super::*;
use crate::node_agent_managed_fs::ManagedSqliteUnlockTarget;

pub(super) fn unlock_main_for_close_test_native(
    main: &mut PinnedManagedSqliteMainFile,
) -> Result<
    (
        Result<(), ManagedSqliteLockFailure>,
        bool,
        Option<ManagedSqliteMainCloseTestProtocolFailure>,
    ),
    (),
> {
    let request = test_faults::claim_test_native(
        &main.close_test_faults,
        ManagedSqliteMainCloseTestFaultPhase::Unlock,
    )?;
    let request = match request {
        None => {
            return Ok((main.unlock_to(ManagedSqliteUnlockTarget::None), false, None));
        }
        Some(request @ (
            ManagedSqliteMainCloseTestNativeRequest::MainLockReleaseNativeUncertainShared
            | ManagedSqliteMainCloseTestNativeRequest::MainLockReleaseNativeUncertainReserved
        )) => request,
        Some(
            ManagedSqliteMainCloseTestNativeRequest::MainFileCloseNativeRetryable
            | ManagedSqliteMainCloseTestNativeRequest::MainFileCloseNativeUncertain,
        ) => return Err(()),
    };
    let offset_class = match request {
        ManagedSqliteMainCloseTestNativeRequest::MainLockReleaseNativeUncertainShared => {
            ManagedSqliteMainLockOffsetClass::SharedRange
        }
        ManagedSqliteMainCloseTestNativeRequest::MainLockReleaseNativeUncertainReserved => {
            ManagedSqliteMainLockOffsetClass::ReservedByte
        }
        ManagedSqliteMainCloseTestNativeRequest::MainFileCloseNativeRetryable
        | ManagedSqliteMainCloseTestNativeRequest::MainFileCloseNativeUncertain => {
            unreachable!("file-close request was rejected above")
        }
    };
    let native = main.unlock_to_for_main_close_test_native(offset_class);
    let protocol_failure = match native.evidence {
        Some(evidence) => test_faults::observe_test_native(&main.close_test_faults, evidence)
            .err()
            .map(|()| {
                ManagedSqliteMainCloseTestProtocolFailure::NativeEvidenceObservationRejected(
                    evidence,
                )
            }),
        None => Some(
            ManagedSqliteMainCloseTestProtocolFailure::NativeEvidenceIncomplete {
                request,
                exact_call_occurrence: None,
                observation: None,
            },
        ),
    };
    Ok((native.result, true, protocol_failure))
}

pub(super) fn close_main_file_for_test_native(
    file: PinnedManagedSqliteFile,
    faults: &Option<std::sync::Arc<dyn ManagedSqliteMainCloseTestFaults>>,
    request: Option<ManagedSqliteMainCloseTestNativeRequest>,
) -> (
    Result<ManagedSqliteFileCloseReceipt, ManagedSqliteFileCloseFailure>,
    bool,
    Option<ManagedSqliteMainCloseTestProtocolFailure>,
) {
    let Some(request) = request else {
        return (file.close(), false, None);
    };
    let native = match request {
        ManagedSqliteMainCloseTestNativeRequest::MainFileCloseNativeRetryable => {
            platform::PlatformManagedSqliteCloseTestNative::Retryable
        }
        ManagedSqliteMainCloseTestNativeRequest::MainFileCloseNativeUncertain => {
            platform::PlatformManagedSqliteCloseTestNative::OutcomeUncertain
        }
        ManagedSqliteMainCloseTestNativeRequest::MainLockReleaseNativeUncertainShared
        | ManagedSqliteMainCloseTestNativeRequest::MainLockReleaseNativeUncertainReserved => {
            unreachable!("lock-release request was rejected by the phase gate")
        }
    };
    let native = file.close_for_unmap_test_native(native);
    let observation = native.observation.map(|observation| match observation {
        crate::node_agent_managed_fs::ManagedSqliteShmTestUnmapNativeObservation::NativeFailureObserved => {
            ManagedSqliteMainCloseTestNativeObservation::NativeFailureObserved
        }
        crate::node_agent_managed_fs::ManagedSqliteShmTestUnmapNativeObservation::ReturnReceiptUnavailable => {
            ManagedSqliteMainCloseTestNativeObservation::ReturnReceiptUnavailable
        }
    });
    let protocol_failure = main_file_native_protocol_failure(
        faults,
        request,
        native.exact_call_occurrence,
        observation,
    );
    (native.result, true, protocol_failure)
}

pub(super) fn main_file_native_protocol_failure(
    faults: &Option<std::sync::Arc<dyn ManagedSqliteMainCloseTestFaults>>,
    request: ManagedSqliteMainCloseTestNativeRequest,
    exact_call_occurrence: Option<std::num::NonZeroU32>,
    observation: Option<ManagedSqliteMainCloseTestNativeObservation>,
) -> Option<ManagedSqliteMainCloseTestProtocolFailure> {
    if let (Some(exact_call_occurrence), Some(observation)) = (exact_call_occurrence, observation) {
        let evidence = ManagedSqliteMainCloseTestNativeEvidence::MainFileClose {
            exact_call_occurrence,
            observation,
        };
        test_faults::observe_test_native(faults, evidence)
            .err()
            .map(|()| {
                ManagedSqliteMainCloseTestProtocolFailure::NativeEvidenceObservationRejected(
                    evidence,
                )
            })
    } else {
        Some(
            ManagedSqliteMainCloseTestProtocolFailure::NativeEvidenceIncomplete {
                request,
                exact_call_occurrence,
                observation,
            },
        )
    }
}
