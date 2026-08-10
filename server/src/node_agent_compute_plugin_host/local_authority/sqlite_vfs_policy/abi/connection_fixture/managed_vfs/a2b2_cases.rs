//! Unrun A2b2 source inventory for the full managed-VFS close authority boundary.
//!
//! The inventory is wired through `managed_vfs.rs` only as an unrun `cfg(test)` static contract;
//! it is not compiled/run evidence and is not Windows dynamic evidence.

mod barrier;
mod close_physical;
mod close_registry;
mod expected;
mod invariants;
mod model;
mod registration;
mod unmap_delete;
mod unmap_nonfinal;
mod unmap_teardown;

#[test]
fn a2b2_declared_static_inventory_is_exact_and_self_consistent() {
    let cases = invariants::inventory();
    invariants::validate(&cases).expect("A2b2 static inventory contract");
}
