use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use super::*;

const FIRST_NONCE: [u8; 16] = [0x21; 16];
const SECOND_NONCE: [u8; 16] = [0x42; 16];

struct DropProbe(Arc<AtomicUsize>);

impl ManagedSqliteRegistryCustody for DropProbe {
    fn ensure_registry_current(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

struct StaleProbe(DropProbe);

impl ManagedSqliteRegistryCustody for StaleProbe {
    fn ensure_registry_current(&self) -> anyhow::Result<()> {
        anyhow::bail!("test custody is stale")
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
fn registration_atomically_holds_policy_state_and_custody() {
    let (custody, drops) = probe();
    let mut owner = ManagedSqliteRegistryOwner::new();
    let route = owner
        .register(FIRST_NONCE, custody)
        .expect("first route must register");

    assert_eq!(
        owner.phase(route),
        Ok(ManagedSqliteRegistrySessionPhase::PendingMain)
    );
    assert!(owner
        .main_logical_name(route)
        .expect("exact route")
        .to_bytes()
        .starts_with(b"elon-hbsql-v1-"));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn duplicate_nonce_returns_new_custody_without_replacing_the_live_route() {
    let (first, first_drops) = probe();
    let (duplicate, duplicate_drops) = probe();
    let mut owner = ManagedSqliteRegistryOwner::new();
    let first_route = owner
        .register(FIRST_NONCE, first)
        .expect("first route must register");
    let failure = owner
        .register(FIRST_NONCE, duplicate)
        .expect_err("duplicate token must fail");
    let (reason, returned) = failure.into_parts();
    assert_eq!(
        reason,
        ManagedSqliteRegistryRegistrationRejection::TokenAlreadyUsed
    );
    assert_eq!(
        owner.phase(first_route),
        Ok(ManagedSqliteRegistrySessionPhase::PendingMain)
    );
    assert_eq!(first_drops.load(Ordering::SeqCst), 0);
    assert_eq!(duplicate_drops.load(Ordering::SeqCst), 0);
    drop(returned);
    assert_eq!(duplicate_drops.load(Ordering::SeqCst), 1);
}

#[test]
fn normal_pending_retirement_releases_custody_and_tombstones_the_token() {
    let (custody, drops) = probe();
    let mut owner = ManagedSqliteRegistryOwner::new();
    let route = owner
        .register(FIRST_NONCE, custody)
        .expect("register route");
    let receipt = owner
        .retire_pending(route)
        .expect("exact pending retirement");
    assert!(!receipt.main_was_claimed());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(
        owner.phase(route),
        Err(ManagedSqliteRegistryRouteRejection::UnknownOrRetired)
    );

    let (replacement, replacement_drops) = probe();
    let failure = owner
        .register(FIRST_NONCE, replacement)
        .expect_err("retired token must remain consumed");
    assert_eq!(
        failure.into_parts().0,
        ManagedSqliteRegistryRegistrationRejection::TokenAlreadyUsed
    );
    assert_eq!(replacement_drops.load(Ordering::SeqCst), 1);
}

#[test]
fn terminal_quarantine_keeps_complete_custody_after_owner_drop() {
    let (custody, drops) = probe();
    let mut owner = ManagedSqliteRegistryOwner::new();
    let route = owner
        .register(FIRST_NONCE, custody)
        .expect("register route");
    owner
        .quarantine(
            route,
            ManagedSqliteRegistryTerminalReason::ConnectionCloseUnproven,
        )
        .expect("quarantine exact route");
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(
        owner.phase(route),
        Err(ManagedSqliteRegistryRouteRejection::UnknownOrRetired)
    );
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
}

#[test]
fn invalid_nonce_and_identity_exhaustion_return_unconsumed_custody() {
    let (zero_custody, zero_drops) = probe();
    let mut owner = ManagedSqliteRegistryOwner::new();
    let failure = owner
        .register([0; 16], zero_custody)
        .expect_err("zero nonce must fail");
    assert_eq!(
        failure.into_parts().0,
        ManagedSqliteRegistryRegistrationRejection::InvalidNonce(
            ManagedSqliteLogicalNameRejection::InvalidRegistryNonce,
        )
    );
    assert_eq!(zero_drops.load(Ordering::SeqCst), 1);

    owner.next_session_id = u64::MAX;
    let (exhausted, exhausted_drops) = probe();
    let failure = owner
        .register(SECOND_NONCE, exhausted)
        .expect_err("session identity exhaustion must fail");
    let (reason, returned) = failure.into_parts();
    assert_eq!(
        reason,
        ManagedSqliteRegistryRegistrationRejection::IdentityExhausted
    );
    assert_eq!(exhausted_drops.load(Ordering::SeqCst), 0);
    drop(returned);
    assert_eq!(exhausted_drops.load(Ordering::SeqCst), 1);
}

#[test]
fn stale_custody_is_returned_before_policy_or_route_creation() {
    let (custody, drops) = probe();
    let mut owner = ManagedSqliteRegistryOwner::new();
    let failure = owner
        .register(FIRST_NONCE, StaleProbe(custody))
        .expect_err("stale custody must fail");
    let (reason, returned) = failure.into_parts();
    assert_eq!(
        reason,
        ManagedSqliteRegistryRegistrationRejection::CustodyNotCurrent
    );
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(returned);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(owner.routes.is_empty());
    assert!(owner.used_tokens.is_empty());
}

#[test]
fn mixed_route_identity_cannot_observe_or_retire_an_entry() {
    let (first, first_drops) = probe();
    let (second, second_drops) = probe();
    let mut owner = ManagedSqliteRegistryOwner::new();
    let first_route = owner.register(FIRST_NONCE, first).expect("first route");
    let second_route = owner.register(SECOND_NONCE, second).expect("second route");
    let mixed = ManagedSqliteRegistryRouteHandle {
        token: first_route.token,
        session_id: second_route.session_id,
        route_epoch: first_route.route_epoch,
    };
    assert_eq!(
        owner.phase(mixed),
        Err(ManagedSqliteRegistryRouteRejection::IdentityMismatch)
    );
    assert_eq!(
        owner.retire_pending(mixed),
        Err(ManagedSqliteRegistryRouteRejection::IdentityMismatch)
    );
    assert_eq!(first_drops.load(Ordering::SeqCst), 0);
    assert_eq!(second_drops.load(Ordering::SeqCst), 0);
    owner.retire_pending(first_route).expect("retire first");
    owner.retire_pending(second_route).expect("retire second");
    assert_eq!(first_drops.load(Ordering::SeqCst), 1);
    assert_eq!(second_drops.load(Ordering::SeqCst), 1);
}
