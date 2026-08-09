use super::super::types::ManagedSqliteRegistryCloseProof;
use super::*;

const SESSION_ID: u64 = 11;
const ROUTE_EPOCH: u64 = 17;

fn pending_session() -> ManagedSqliteRegistrySessionState {
    ManagedSqliteRegistrySessionState {
        session_id: ManagedSqliteRegistrySessionId::test_value(SESSION_ID),
        route_epoch: NonZeroU64::new(ROUTE_EPOCH).expect("route epoch"),
        phase: ManagedSqliteRegistrySessionPhase::PendingMain,
        next_lease_ordinal: 0,
        connection_owner: false,
        main_was_claimed: false,
        main_lease: None,
        sidecar_leases: [None; 4],
        shm_lease: None,
        callbacks_in_flight: 0,
        terminal_reason: None,
    }
}

fn route_proof(
    session_id: ManagedSqliteRegistrySessionId,
    route_epoch: u64,
) -> ManagedSqliteRegistryRouteRemovalProof {
    ManagedSqliteRegistryRouteRemovalProof::test_value(session_id, route_epoch)
}

fn close_proof(lease: &ManagedSqliteRegistryFileLease) -> ManagedSqliteRegistryCloseOutcome {
    ManagedSqliteRegistryCloseOutcome::Proven(ManagedSqliteRegistryCloseProof::test_value(
        lease.session_id,
        lease.ordinal,
    ))
}

fn close_shm_proof(lease: &ManagedSqliteRegistryShmLease) -> ManagedSqliteRegistryCloseOutcome {
    ManagedSqliteRegistryCloseOutcome::Proven(ManagedSqliteRegistryCloseProof::test_value(
        lease.session_id,
        lease.ordinal,
    ))
}

#[test]
fn pending_session_retires_only_after_exact_route_removal() {
    let mut session = pending_session();
    let proof = route_proof(session.session_id, ROUTE_EPOCH);

    let receipt = session
        .cancel_pending_after_route_removed(proof)
        .expect("exact pending route removal must retire");

    assert_eq!(session.phase(), ManagedSqliteRegistrySessionPhase::Retired);
    assert_eq!(receipt.session_id(), session.session_id);
    assert_eq!(receipt.route_epoch(), session.route_epoch);
    assert!(!receipt.main_was_claimed());
}

#[test]
fn full_lifecycle_closes_every_exact_lease_before_retirement() {
    let mut session = pending_session();
    session.begin_open_attempt().expect("begin open");
    let main = session.claim_main().expect("claim main");
    let journal = session
        .claim_sidecar(ManagedSqliteLogicalFileRole::Journal)
        .expect("claim journal");
    let wal = session
        .claim_sidecar(ManagedSqliteLogicalFileRole::Wal)
        .expect("claim wal");
    session.activate_connection().expect("activate");
    let shm = session.claim_shm().expect("claim shm");
    let callback = session
        .begin_callback(ManagedSqliteRegistryCallbackKind::Io)
        .expect("begin callback");
    session.finish_callback(&callback).expect("finish callback");

    session.begin_connection_close().expect("begin close");
    let journal_proof = close_proof(&journal);
    session
        .close_file(&journal, &journal_proof)
        .expect("close journal");
    let wal_proof = close_proof(&wal);
    session.close_file(&wal, &wal_proof).expect("close wal");
    let shm_proof = close_shm_proof(&shm);
    session.close_shm(&shm, &shm_proof).expect("close shm");
    let main_proof = close_proof(&main);
    session.close_file(&main, &main_proof).expect("close main");
    session
        .observe_connection_closed()
        .expect("observe connection close");

    let receipt = session
        .retire_after_route_removed(route_proof(session.session_id, ROUTE_EPOCH))
        .expect("retire exact route");
    assert_eq!(session.phase(), ManagedSqliteRegistrySessionPhase::Retired);
    assert!(receipt.main_was_claimed());
}

#[test]
fn route_identity_mismatch_enters_permanent_terminal_quarantine() {
    let mut session = pending_session();
    let wrong_session = ManagedSqliteRegistrySessionId::test_value(SESSION_ID + 1);

    assert_eq!(
        session.cancel_pending_after_route_removed(route_proof(wrong_session, ROUTE_EPOCH)),
        Err(ManagedSqliteRegistryTransitionRejection::RouteRemovalUnproven)
    );
    assert_eq!(
        session.phase(),
        ManagedSqliteRegistrySessionPhase::TerminalQuarantine
    );
    assert_eq!(
        session.terminal_reason(),
        Some(ManagedSqliteRegistryTerminalReason::RouteIdentityMismatch)
    );
    session.quarantine(ManagedSqliteRegistryTerminalReason::CallbackPanicked);
    assert_eq!(
        session.terminal_reason(),
        Some(ManagedSqliteRegistryTerminalReason::RouteIdentityMismatch)
    );
    assert_eq!(
        session.begin_open_attempt(),
        Err(ManagedSqliteRegistryTransitionRejection::Terminal)
    );
}

#[test]
fn mismatched_close_proof_quarantines_live_file_custody() {
    let mut session = pending_session();
    session.begin_open_attempt().expect("begin open");
    let main = session.claim_main().expect("claim main");
    let wrong_session = ManagedSqliteRegistrySessionId::test_value(SESSION_ID + 1);
    let wrong = ManagedSqliteRegistryCloseOutcome::Proven(
        ManagedSqliteRegistryCloseProof::test_value(wrong_session, main.ordinal),
    );

    assert_eq!(
        session.close_file(&main, &wrong),
        Err(ManagedSqliteRegistryTransitionRejection::LeaseIdentityMismatch)
    );
    assert_eq!(
        session.terminal_reason(),
        Some(ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch)
    );
}

#[test]
fn unproven_close_preserves_the_exact_terminal_reason() {
    let mut session = pending_session();
    session.begin_open_attempt().expect("begin open");
    let main = session.claim_main().expect("claim main");

    assert_eq!(
        session.close_file(
            &main,
            &ManagedSqliteRegistryCloseOutcome::Unproven(
                ManagedSqliteRegistryTerminalReason::HandleCloseUnproven,
            ),
        ),
        Err(ManagedSqliteRegistryTransitionRejection::Terminal)
    );
    assert_eq!(
        session.terminal_reason(),
        Some(ManagedSqliteRegistryTerminalReason::HandleCloseUnproven)
    );
}

#[test]
fn callbacks_must_drain_before_activation_or_connection_close() {
    let mut session = pending_session();
    session.begin_open_attempt().expect("begin open");
    let _main = session.claim_main().expect("claim main");
    let opening_callback = session
        .begin_callback(ManagedSqliteRegistryCallbackKind::Open)
        .expect("begin opening callback");

    assert_eq!(
        session.activate_connection(),
        Err(ManagedSqliteRegistryTransitionRejection::OutstandingCallbacks)
    );
    session
        .finish_callback(&opening_callback)
        .expect("finish opening callback");
    session.activate_connection().expect("activate");
    let active_callback = session
        .begin_callback(ManagedSqliteRegistryCallbackKind::Io)
        .expect("begin active callback");
    assert_eq!(
        session.begin_connection_close(),
        Err(ManagedSqliteRegistryTransitionRejection::OutstandingCallbacks)
    );
    session
        .finish_callback(&active_callback)
        .expect("finish active callback");
    session.begin_connection_close().expect("begin close");
}

#[test]
fn main_cannot_close_before_sidecar_and_shm_custody() {
    let mut session = pending_session();
    session.begin_open_attempt().expect("begin open");
    let main = session.claim_main().expect("claim main");
    let _wal = session
        .claim_sidecar(ManagedSqliteLogicalFileRole::Wal)
        .expect("claim wal");
    session.activate_connection().expect("activate");
    let _shm = session.claim_shm().expect("claim shm");
    session.begin_connection_close().expect("begin close");
    let proof = close_proof(&main);

    assert_eq!(
        session.close_file(&main, &proof),
        Err(ManagedSqliteRegistryTransitionRejection::LeaseIdentityMismatch)
    );
    assert_eq!(
        session.terminal_reason(),
        Some(ManagedSqliteRegistryTerminalReason::LeaseIdentityMismatch)
    );
}

#[test]
fn observing_connection_close_with_live_handles_fails_closed() {
    let mut session = pending_session();
    session.begin_open_attempt().expect("begin open");
    let _main = session.claim_main().expect("claim main");
    session.activate_connection().expect("activate");
    session.begin_connection_close().expect("begin close");

    assert_eq!(
        session.observe_connection_closed(),
        Err(ManagedSqliteRegistryTransitionRejection::OutstandingHandles)
    );
    assert_eq!(
        session.terminal_reason(),
        Some(ManagedSqliteRegistryTerminalReason::ConnectionCloseUnproven)
    );
}

#[test]
fn sidecar_admission_is_bounded_and_requires_main_custody() {
    let mut session = pending_session();
    session.begin_open_attempt().expect("begin open");
    assert!(matches!(
        session.claim_sidecar(ManagedSqliteLogicalFileRole::Wal),
        Err(ManagedSqliteRegistryTransitionRejection::MainNotClaimed)
    ));
    let _main = session.claim_main().expect("claim main");
    assert!(matches!(
        session.claim_sidecar(ManagedSqliteLogicalFileRole::Main),
        Err(ManagedSqliteRegistryTransitionRejection::InvalidSidecarRole)
    ));
    for index in 0..4 {
        let role = if index % 2 == 0 {
            ManagedSqliteLogicalFileRole::Journal
        } else {
            ManagedSqliteLogicalFileRole::Wal
        };
        let _lease = session.claim_sidecar(role).expect("bounded sidecar slot");
    }
    assert!(matches!(
        session.claim_sidecar(ManagedSqliteLogicalFileRole::Wal),
        Err(ManagedSqliteRegistryTransitionRejection::LeaseCapacityExhausted)
    ));
    assert_eq!(session.phase(), ManagedSqliteRegistrySessionPhase::Opening);
    assert_eq!(session.terminal_reason(), None);
}

#[test]
fn closing_phase_rejects_new_open_and_access_callbacks_but_allows_teardown() {
    let mut session = pending_session();
    session.begin_open_attempt().expect("begin open");
    let _main = session.claim_main().expect("claim main");
    session.activate_connection().expect("activate");
    session.begin_connection_close().expect("begin close");

    for kind in [
        ManagedSqliteRegistryCallbackKind::FullPathname,
        ManagedSqliteRegistryCallbackKind::Open,
        ManagedSqliteRegistryCallbackKind::Access,
    ] {
        assert!(matches!(
            session.begin_callback(kind),
            Err(ManagedSqliteRegistryTransitionRejection::WrongPhase)
        ));
    }
    for kind in [
        ManagedSqliteRegistryCallbackKind::Delete,
        ManagedSqliteRegistryCallbackKind::Io,
        ManagedSqliteRegistryCallbackKind::Close,
        ManagedSqliteRegistryCallbackKind::Shm,
    ] {
        let lease = session
            .begin_callback(kind)
            .expect("teardown callback must remain admitted");
        session.finish_callback(&lease).expect("finish callback");
    }
}
