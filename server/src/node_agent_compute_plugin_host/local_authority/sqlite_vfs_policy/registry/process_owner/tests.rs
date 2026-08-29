use std::{
    collections::VecDeque,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::{
    registry::types::ManagedSqliteRegistryTransitionRejection, ManagedSqliteAuthorizerAction,
    ManagedSqliteAuthorizerDecision, ManagedSqliteAuthorizerRequest,
    ManagedSqliteAuthorizerTransitionError,
};
use crate::node_agent_managed_fs::ManagedSqliteFileKind;

const FIRST_NONCE: [u8; 16] = [0x31; 16];
const SECOND_NONCE: [u8; 16] = [0x52; 16];

struct SequenceNonceSource(Mutex<VecDeque<Result<[u8; 16], ()>>>);

impl SequenceNonceSource {
    fn new(values: impl IntoIterator<Item = Result<[u8; 16], ()>>) -> Self {
        Self(Mutex::new(values.into_iter().collect()))
    }
}

impl ManagedSqliteRegistryNonceSource for SequenceNonceSource {
    fn fill_nonce(&self, output: &mut [u8; 16]) -> Result<(), ()> {
        let value = self
            .0
            .lock()
            .expect("test nonce queue")
            .pop_front()
            .ok_or(())??;
        *output = value;
        Ok(())
    }
}

struct DropProbe(Arc<AtomicUsize>);

impl ManagedSqliteRegistryCustody for DropProbe {
    fn ensure_registry_current(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn probe() -> (DropProbe, Arc<AtomicUsize>) {
    let drops = Arc::new(AtomicUsize::new(0));
    (DropProbe(Arc::clone(&drops)), drops)
}

#[test]
fn generated_nonce_retries_zero_and_live_token_collisions() {
    let process = ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([
        Ok([0; 16]),
        Ok(FIRST_NONCE),
        Ok(FIRST_NONCE),
        Ok(SECOND_NONCE),
    ]));
    let (first, first_drops) = probe();
    let (second, second_drops) = probe();

    let first_route = process.register(first).expect("first generated route");
    let second_route = process.register(second).expect("collision retry route");

    assert_ne!(first_route, second_route);
    assert_eq!(first_drops.load(Ordering::SeqCst), 0);
    assert_eq!(second_drops.load(Ordering::SeqCst), 0);
}

#[test]
fn entropy_failure_returns_unconsumed_custody() {
    let process = ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Err(())]));
    let (custody, drops) = probe();

    let failure = process.register(custody).expect_err("entropy failure");
    let (reason, returned) = failure.into_parts();
    assert_eq!(
        reason,
        ManagedSqliteRegistryProcessRegistrationRejection::EntropyUnavailable
    );
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(returned);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn collision_budget_exhaustion_returns_unconsumed_custody() {
    let values = std::iter::once(Ok(FIRST_NONCE))
        .chain(std::iter::repeat_n(Ok(FIRST_NONCE), ROUTE_NONCE_ATTEMPTS));
    let process = ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new(values));
    let (first, first_drops) = probe();
    let (duplicate, duplicate_drops) = probe();
    process.register(first).expect("first route");

    let failure = process.register(duplicate).expect_err("collision budget");
    let (reason, returned) = failure.into_parts();
    assert_eq!(
        reason,
        ManagedSqliteRegistryProcessRegistrationRejection::CollisionBudgetExhausted
    );
    assert_eq!(first_drops.load(Ordering::SeqCst), 0);
    assert_eq!(duplicate_drops.load(Ordering::SeqCst), 0);
    drop(returned);
    assert_eq!(duplicate_drops.load(Ordering::SeqCst), 1);
}

#[test]
fn routed_authorizer_transition_failure_retains_policy_and_custody() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (custody, drops) = probe();
    let route = process.register(custody).expect("authorizer route");
    let select =
        || ManagedSqliteAuthorizerRequest::new(ManagedSqliteAuthorizerAction::Select, None, None);

    assert_eq!(
        process.authorize_sql(route, select()),
        Ok(ManagedSqliteAuthorizerDecision::Deny)
    );
    assert_eq!(
        process.enter_runtime(route),
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::Authorizer(
                ManagedSqliteAuthorizerTransitionError::InvalidPhaseTransition
            )
        ))
    );
    assert_eq!(drops.load(Ordering::SeqCst), 0);

    process
        .enter_schema_migration(route)
        .expect("bootstrap to schema");
    assert_eq!(
        process.authorize_sql(route, select()),
        Ok(ManagedSqliteAuthorizerDecision::Allow)
    );
    process.enter_runtime(route).expect("schema to runtime");
    let _receipt = process.retire_pending(route).expect("retire route");
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn routed_callback_lease_blocks_close_until_explicit_completion() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (custody, drops) = probe();
    let route = process.register(custody).expect("route");
    process.begin_open_attempt(route).expect("opening");
    let callback = process
        .begin_callback(route, ManagedSqliteRegistryCallbackKind::Open)
        .expect("callback lease");

    assert_eq!(
        process.begin_connection_close(route),
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::State(
                ManagedSqliteRegistryTransitionRejection::OutstandingCallbacks,
            ),
        ))
    );
    callback.complete().expect("complete callback");
    process
        .begin_connection_close(route)
        .expect("close after callback drains");
    assert_eq!(drops.load(Ordering::SeqCst), 0);
}

#[test]
fn dropped_callback_lease_quarantines_the_exact_route() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (custody, drops) = probe();
    let route = process.register(custody).expect("route");
    process.begin_open_attempt(route).expect("opening");
    {
        let _callback = process
            .begin_callback(route, ManagedSqliteRegistryCallbackKind::FullPathname)
            .expect("callback lease");
    }
    let witness = process
        .terminal_custody_test_snapshot(route)
        .expect("redacted terminal custody witness");
    assert_eq!(witness.retention_count(), 1);
    assert_eq!(witness.callback_lease_retention_count(), 1);
    assert_eq!(witness.completion_evidence_retention_count(), 0);
    assert_eq!(witness.other_terminal_custody_retention_count(), 0);
    assert_eq!(witness.explicit_failure_custody_retained_count(), 1);
    assert_eq!(witness.route_removal_count(), 1);
    assert!(!witness.active_route_present());
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert!(matches!(
        process.begin_callback(route, ManagedSqliteRegistryCallbackKind::FullPathname),
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::UnknownOrRetired,
        ))
    ));
}

#[test]
fn callback_completion_evidence_witness_is_exact_and_route_cannot_be_reused() {
    let process = ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([
        Ok(FIRST_NONCE),
        Ok(SECOND_NONCE),
    ]));
    let (terminal_custody, terminal_drops) = probe();
    let (untouched_custody, untouched_drops) = probe();
    let route = process.register(terminal_custody).expect("terminal route");
    let untouched_route = process
        .register(untouched_custody)
        .expect("untouched route");
    process.begin_open_attempt(route).expect("opening");
    let completed = process
        .begin_callback(route, ManagedSqliteRegistryCallbackKind::FullPathname)
        .expect("callback lease")
        .complete_with_receipt()
        .expect("completion evidence");

    process
        .retain_terminal_custody(
            route,
            ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
            completed,
        )
        .expect("retain completion evidence and remove route");

    let witness = process
        .terminal_custody_test_snapshot(route)
        .expect("redacted terminal custody witness");
    assert_eq!(witness.retention_count(), 1);
    assert_eq!(witness.callback_lease_retention_count(), 0);
    assert_eq!(witness.completion_evidence_retention_count(), 1);
    assert_eq!(witness.other_terminal_custody_retention_count(), 0);
    assert_eq!(witness.explicit_failure_custody_retained_count(), 1);
    assert_eq!(witness.route_removal_count(), 1);
    assert!(!witness.active_route_present());
    assert!(matches!(
        process.begin_callback(route, ManagedSqliteRegistryCallbackKind::FullPathname),
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::UnknownOrRetired,
        ))
    ));

    let untouched = process
        .terminal_custody_test_snapshot(untouched_route)
        .expect("unrelated exact route witness");
    assert_eq!(untouched.retention_count(), 0);
    assert_eq!(untouched.route_removal_count(), 0);
    assert!(untouched.active_route_present());
    assert_eq!(terminal_drops.load(Ordering::SeqCst), 0);
    assert_eq!(untouched_drops.load(Ordering::SeqCst), 0);
}

#[test]
fn barrier_native_completion_rejection_retains_exact_callback_and_keeps_sibling_live() {
    let process = ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([
        Ok(FIRST_NONCE),
        Ok(SECOND_NONCE),
    ]));
    let (terminal_custody, terminal_drops) = probe();
    let (sibling_custody, sibling_drops) = probe();
    let route = process.register(terminal_custody).expect("terminal route");
    let sibling = process.register(sibling_custody).expect("sibling route");
    process
        .begin_open_attempt(route)
        .expect("open terminal route");
    let _main = process.claim_main(route).expect("claim terminal main");
    process
        .activate_connection(route)
        .expect("activate terminal route");
    let mut callback = process
        .begin_callback(route, ManagedSqliteRegistryCallbackKind::Shm)
        .expect("begin exact SHM callback");

    callback
        .arm_shm_callback_completion_native_rejection()
        .expect("arm exact registry rejection");
    assert!(matches!(
        callback.complete_with_receipt(),
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::State(
                ManagedSqliteRegistryTransitionRejection::Terminal,
            ),
        ))
    ));

    let witness = process
        .terminal_custody_test_snapshot(route)
        .expect("terminal callback witness");
    assert_eq!(witness.callback_lease_retention_count(), 1);
    assert_eq!(witness.explicit_failure_custody_retained_count(), 1);
    assert_eq!(witness.route_removal_count(), 1);
    assert!(!witness.active_route_present());
    assert_eq!(terminal_drops.load(Ordering::SeqCst), 0);

    process
        .begin_open_attempt(sibling)
        .expect("sibling owner remains live");
    process
        .begin_callback(sibling, ManagedSqliteRegistryCallbackKind::Open)
        .expect("sibling callback")
        .complete()
        .expect("sibling callback completion");
    assert_eq!(sibling_drops.load(Ordering::SeqCst), 0);
}

#[test]
fn retired_route_cannot_admit_a_callback() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (custody, drops) = probe();
    let route = process.register(custody).expect("route");
    process.retire_pending(route).expect("retire pending route");

    assert!(matches!(
        process.begin_callback(route, ManagedSqliteRegistryCallbackKind::Open),
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::UnknownOrRetired
        ))
    ));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn poisoned_process_owner_fails_closed_without_dropping_existing_custody() {
    let process = ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([
        Ok(FIRST_NONCE),
        Ok(SECOND_NONCE),
    ]));
    let (first, first_drops) = probe();
    let (second, second_drops) = probe();
    process.register(first).expect("first route");
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _guard = process.routes.lock().expect("lock before poison");
        panic!("poison process owner for test");
    }));

    let failure = process.register(second).expect_err("poisoned owner");
    let (reason, returned) = failure.into_parts();
    assert_eq!(
        reason,
        ManagedSqliteRegistryProcessRegistrationRejection::OwnerPoisoned
    );
    assert_eq!(first_drops.load(Ordering::SeqCst), 0);
    assert_eq!(second_drops.load(Ordering::SeqCst), 0);
    drop(returned);
    assert_eq!(second_drops.load(Ordering::SeqCst), 1);
}

#[test]
fn close_evidence_is_permanently_retained_when_route_lock_is_poisoned() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (route_custody, route_drops) = probe();
    let (close_evidence, evidence_drops) = probe();
    let route = process.register(route_custody).expect("route");
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _guard = process.routes.lock().expect("lock before poison");
        panic!("poison process owner before close transition");
    }));

    let result: Result<(), _> = process.apply_route_retaining_failure(
        route,
        close_evidence,
        lifecycle::ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CompletionEvidence,
        |_routes, _evidence| Ok(()),
    );

    assert_eq!(
        result,
        Err(ManagedSqliteRegistryProcessRouteRejection::OwnerPoisoned)
    );
    assert_eq!(route_drops.load(Ordering::SeqCst), 0);
    assert_eq!(evidence_drops.load(Ordering::SeqCst), 0);
}

#[test]
fn close_evidence_is_permanently_retained_when_route_transition_rejects() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (route_custody, route_drops) = probe();
    let (close_evidence, evidence_drops) = probe();
    let route = process.register(route_custody).expect("route");

    let result: Result<(), _> = process.apply_route_retaining_failure(
        route,
        close_evidence,
        lifecycle::ManagedSqliteRegistryTerminalCustodyTestRetentionKind::CompletionEvidence,
        |_routes, _evidence| Err(ManagedSqliteRegistryRouteRejection::UnknownOrRetired),
    );

    assert_eq!(
        result,
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::UnknownOrRetired,
        ))
    );
    assert_eq!(route_drops.load(Ordering::SeqCst), 0);
    assert_eq!(evidence_drops.load(Ordering::SeqCst), 0);
}

#[test]
fn terminal_retention_precedes_exact_route_quarantine() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (route_custody, route_drops) = probe();
    let (physical_custody, physical_drops) = probe();
    let route = process.register(route_custody).expect("route");
    process.begin_open_attempt(route).expect("opening");

    process
        .retain_terminal_custody(
            route,
            ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
            physical_custody,
        )
        .expect("quarantine exact route");

    assert_eq!(route_drops.load(Ordering::SeqCst), 0);
    assert_eq!(physical_drops.load(Ordering::SeqCst), 0);
    assert!(matches!(
        process.phase(route),
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::UnknownOrRetired,
        ))
    ));
}

#[test]
fn second_terminal_retention_keeps_custody_without_duplicate_route_projection() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (route_custody, route_drops) = probe();
    let (first_retained, first_drops) = probe();
    let (second_retained, second_drops) = probe();
    let route = process.register(route_custody).expect("route");
    process.begin_open_attempt(route).expect("opening");

    process
        .retain_terminal_custody(
            route,
            ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
            first_retained,
        )
        .expect("first terminal retention");
    assert!(matches!(
        process.retain_terminal_custody(
            route,
            ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
            second_retained,
        ),
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::UnknownOrRetired,
        ))
    ));

    let witness = process
        .terminal_custody_test_snapshot(route)
        .expect("double-retention witness");
    assert_eq!(witness.retention_count(), 2);
    assert_eq!(witness.terminal_route_observation_count(), 1);
    assert_eq!(witness.route_removal_count(), 1);
    assert_eq!(route_drops.load(Ordering::SeqCst), 0);
    assert_eq!(first_drops.load(Ordering::SeqCst), 0);
    assert_eq!(second_drops.load(Ordering::SeqCst), 0);
}

#[test]
fn process_owner_is_explicitly_leaked_and_starts_pending() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (custody, drops) = probe();
    let route = process.register(custody).expect("route");

    assert_eq!(
        process.phase(route),
        Ok(ManagedSqliteRegistrySessionPhase::PendingMain)
    );
    assert_eq!(drops.load(Ordering::SeqCst), 0);
}

#[test]
fn managed_fs_receipts_drive_exact_process_route_retirement() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (custody, drops) = probe();
    let route = process.register(custody).expect("route");

    process.begin_open_attempt(route).expect("opening");
    let main = process.claim_main(route).expect("main lease");
    let wal = process
        .claim_sidecar(route, ManagedSqliteLogicalFileRole::Wal)
        .expect("WAL lease");
    process.activate_connection(route).expect("active");
    let shm = process.claim_shm(route).expect("SHM lease");
    process.begin_connection_close(route).expect("begin close");

    process
        .close_sidecar(
            route,
            wal,
            ManagedSqliteFileCloseReceipt::test_value(ManagedSqliteFileKind::Wal),
        )
        .expect("close WAL sidecar");
    process
        .close_wal_main(
            route,
            main,
            shm,
            ManagedSqliteWalMainCloseReceipt::test_value(),
        )
        .expect("close WAL main and SHM");
    process
        .observe_connection_closed(route)
        .expect("observe connection close");
    let receipt = process.retire_closed(route).expect("retire closed route");

    assert!(receipt.main_was_claimed());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(
        process.phase(route),
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::UnknownOrRetired,
        ))
    );
}

#[test]
fn mismatched_managed_fs_receipt_permanently_quarantines_route_custody() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (custody, drops) = probe();
    let route = process.register(custody).expect("route");

    process.begin_open_attempt(route).expect("opening");
    let _main = process.claim_main(route).expect("main lease");
    let journal = process
        .claim_sidecar(route, ManagedSqliteLogicalFileRole::Journal)
        .expect("journal lease");
    let result = process.close_sidecar(
        route,
        journal,
        ManagedSqliteFileCloseReceipt::test_value(ManagedSqliteFileKind::Wal),
    );

    assert_eq!(
        result,
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::State(
                ManagedSqliteRegistryTransitionRejection::Terminal,
            ),
        ))
    );
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(
        process.phase(route),
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::UnknownOrRetired,
        ))
    );
}

#[test]
fn main_receipt_closes_non_wal_connection_before_retirement() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (custody, drops) = probe();
    let route = process.register(custody).expect("route");

    process.begin_open_attempt(route).expect("opening");
    let main = process.claim_main(route).expect("main lease");
    process.activate_connection(route).expect("active");
    process.begin_connection_close(route).expect("begin close");
    process
        .close_main(route, main, ManagedSqliteMainFileCloseReceipt::test_value())
        .expect("close main");
    process
        .observe_connection_closed(route)
        .expect("observe connection close");
    process.retire_closed(route).expect("retire closed route");

    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn generic_file_receipt_cannot_bypass_main_lock_domain_close() {
    let process =
        ManagedSqliteRegistryProcessOwner::leak(SequenceNonceSource::new([Ok(FIRST_NONCE)]));
    let (custody, drops) = probe();
    let route = process.register(custody).expect("route");

    process.begin_open_attempt(route).expect("opening");
    let main = process.claim_main(route).expect("main lease");
    let result = process.close_sidecar(
        route,
        main,
        ManagedSqliteFileCloseReceipt::test_value(ManagedSqliteFileKind::Main),
    );

    assert_eq!(
        result,
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::State(
                ManagedSqliteRegistryTransitionRejection::Terminal,
            ),
        ))
    );
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(
        process.phase(route),
        Err(ManagedSqliteRegistryProcessRouteRejection::Route(
            ManagedSqliteRegistryRouteRejection::UnknownOrRetired,
        ))
    );
}
