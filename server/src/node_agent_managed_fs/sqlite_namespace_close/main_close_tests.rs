use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use uuid::Uuid;

use super::*;
use crate::node_agent_managed_fs::{
    ManagedSqliteAccess, ManagedSqliteDeleteOutcome, ManagedSqliteFileKind,
    ManagedSqliteLockAttempt, ManagedSqliteOpenMode, ManagedSqliteRequestedLock, PinnedManagedRoot,
    PinnedManagedSqliteNamespace,
};

struct MainCloseFixture {
    path: PathBuf,
    root: PinnedManagedRoot,
    namespace: PinnedManagedSqliteNamespace,
}

impl MainCloseFixture {
    fn new() -> (Self, PinnedManagedSqliteMainFile) {
        let path = std::env::temp_dir().join(format!(
            "elon-managed-main-close-{}",
            Uuid::new_v4().simple()
        ));
        std::fs::create_dir(&path).expect("create isolated main-close root");
        let root = PinnedManagedRoot::pin(&path, &"a".repeat(64)).expect("pin test root");
        drop(
            root.prepare_directory(Path::new("sqlite"))
                .expect("create SQLite directory"),
        );
        let directory = root
            .prepare_directory(Path::new("sqlite"))
            .expect("pin unchanged SQLite directory");
        let namespace = directory
            .into_sqlite_namespace()
            .expect("bind SQLite namespace");
        let main = namespace
            .open(
                ManagedSqliteFileKind::Main,
                ManagedSqliteAccess::ReadWrite,
                ManagedSqliteOpenMode::OpenOrCreate,
            )
            .expect("open test main file")
            .into_main_file()
            .expect("bind test main file");
        (
            Self {
                path,
                root,
                namespace,
            },
            main,
        )
    }

    fn cleanup(self) {
        assert_eq!(
            self.namespace
                .delete(ManagedSqliteFileKind::Main, false)
                .expect("delete closed test main file"),
            ManagedSqliteDeleteOutcome::Deleted
        );
        drop(self.namespace);
        drop(self.root);
        std::fs::remove_dir(self.path.join("sqlite")).expect("remove SQLite test directory");
        std::fs::remove_dir(self.path).expect("remove main-close test root");
    }
}

struct RejectingNativeFaults {
    phase: ManagedSqliteMainCloseTestFaultPhase,
    request: Mutex<Option<ManagedSqliteMainCloseTestNativeRequest>>,
    reject_claim_once: AtomicBool,
    reject_observation: bool,
    observed: Mutex<Vec<ManagedSqliteMainCloseTestNativeEvidence>>,
}

impl RejectingNativeFaults {
    fn claim_rejected(
        phase: ManagedSqliteMainCloseTestFaultPhase,
        request: ManagedSqliteMainCloseTestNativeRequest,
    ) -> Arc<Self> {
        Arc::new(Self {
            phase,
            request: Mutex::new(Some(request)),
            reject_claim_once: AtomicBool::new(true),
            reject_observation: false,
            observed: Mutex::new(Vec::new()),
        })
    }

    fn observation_rejected(
        phase: ManagedSqliteMainCloseTestFaultPhase,
        request: ManagedSqliteMainCloseTestNativeRequest,
    ) -> Arc<Self> {
        Arc::new(Self {
            phase,
            request: Mutex::new(Some(request)),
            reject_claim_once: AtomicBool::new(false),
            reject_observation: true,
            observed: Mutex::new(Vec::new()),
        })
    }
}

impl ManagedSqliteMainCloseTestFaults for RejectingNativeFaults {
    fn before(&self, _phase: ManagedSqliteMainCloseTestFaultPhase) -> Result<bool, ()> {
        Ok(false)
    }

    fn after_success(&self, _phase: ManagedSqliteMainCloseTestFaultPhase) -> Result<bool, ()> {
        Ok(false)
    }

    fn native_failure(&self, _phase: ManagedSqliteMainCloseTestFaultPhase) {}

    fn claim_test_native(
        &self,
        phase: ManagedSqliteMainCloseTestFaultPhase,
    ) -> Result<Option<ManagedSqliteMainCloseTestNativeRequest>, ()> {
        if phase != self.phase {
            return Ok(None);
        }
        let mut request = self.request.lock().map_err(|_| ())?;
        if self.reject_claim_once.swap(false, Ordering::SeqCst) {
            let _ = request.take();
            return Err(());
        }
        Ok(request.take())
    }

    fn observe_test_native(
        &self,
        evidence: ManagedSqliteMainCloseTestNativeEvidence,
    ) -> Result<(), ()> {
        self.observed.lock().map_err(|_| ())?.push(evidence);
        if self.reject_observation {
            Err(())
        } else {
            Ok(())
        }
    }
}

#[test]
fn native_claim_rejection_is_before_call_with_zero_native_evidence() {
    let (fixture, mut main) = MainCloseFixture::new();
    assert_eq!(
        main.lock_to(ManagedSqliteRequestedLock::Shared)
            .expect("acquire shared main lock"),
        ManagedSqliteLockAttempt::Acquired
    );
    let faults = RejectingNativeFaults::claim_rejected(
        ManagedSqliteMainCloseTestFaultPhase::Unlock,
        ManagedSqliteMainCloseTestNativeRequest::MainLockReleaseNativeUncertainShared,
    );
    main.install_close_test_faults(faults.clone())
        .expect("install route-bound native faults");

    let failure = match main.close() {
        Ok(_) => panic!("claim rejection must stop before native unlock"),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.test_fault,
        Some(ManagedSqliteMainCloseTestFault {
            phase: ManagedSqliteMainCloseTestFaultPhase::Unlock,
            timing: ManagedSqliteMainCloseTestFaultTiming::BeforeCall,
        })
    );
    assert_eq!(failure.test_protocol_failure(), None);
    assert!(faults
        .observed
        .lock()
        .expect("observed evidence")
        .is_empty());

    let main = failure
        .into_main()
        .expect("claim rejection retains live main");
    match main.close() {
        Ok(_) => {}
        Err(_) => panic!("retry after one-shot claim rejection must close"),
    }
    fixture.cleanup();
}

#[test]
fn native_observation_rejection_keeps_retryable_custody_and_exposes_exact_call_marker() {
    let (fixture, mut main) = MainCloseFixture::new();
    let faults = RejectingNativeFaults::observation_rejected(
        ManagedSqliteMainCloseTestFaultPhase::FileClose,
        ManagedSqliteMainCloseTestNativeRequest::MainFileCloseNativeRetryable,
    );
    main.install_close_test_faults(faults.clone())
        .expect("install route-bound native faults");

    let failure = match main.close() {
        Ok(_) => panic!("observer rejection must fail closed"),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.phase(),
        ManagedSqliteMainFileCloseFailurePhase::FileClose
    );
    assert!(!failure.close_outcome_uncertain());
    let evidence = match failure.test_protocol_failure() {
        Some(ManagedSqliteMainCloseTestProtocolFailure::NativeEvidenceObservationRejected(
            evidence,
        )) => evidence,
        Some(ManagedSqliteMainCloseTestProtocolFailure::NativeEvidenceIncomplete { .. }) => {
            panic!("complete retryable evidence must not be classified as incomplete")
        }
        None => panic!("observer rejection marker must be retained"),
    };
    assert_eq!(
        evidence,
        ManagedSqliteMainCloseTestNativeEvidence::MainFileClose {
            exact_call_occurrence: std::num::NonZeroU32::new(1).expect("one is non-zero"),
            observation: ManagedSqliteMainCloseTestNativeObservation::NativeFailureObserved,
        }
    );
    assert_eq!(
        faults
            .observed
            .lock()
            .expect("observed evidence")
            .as_slice(),
        &[evidence]
    );

    let main = failure
        .into_main()
        .expect("retryable CloseHandle failure retains live main custody");
    match main.close() {
        Ok(_) => {}
        Err(_) => panic!("retry after rejected evidence publication must close"),
    }
    fixture.cleanup();
}

#[test]
fn requested_native_close_with_missing_observation_is_a_typed_protocol_failure() {
    let request = ManagedSqliteMainCloseTestNativeRequest::MainFileCloseNativeRetryable;
    let protocol_failure =
        main_file_native_protocol_failure(&None, request, std::num::NonZeroU32::new(1), None);
    assert!(matches!(
        protocol_failure,
        Some(ManagedSqliteMainCloseTestProtocolFailure::NativeEvidenceIncomplete {
            request: actual_request,
            exact_call_occurrence: Some(exact_call_occurrence),
            observation: None,
        }) if actual_request == request
            && exact_call_occurrence.get() == 1
    ));
}
