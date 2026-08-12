//! Unrun A2b2 source inventory for the full managed-VFS close authority boundary.
//!
//! The inventory is wired through `managed_vfs.rs` only as an unrun `cfg(test)` static contract;
//! it is not compiled/run evidence and is not Windows dynamic evidence.

mod barrier;
mod close_physical;
mod close_registry;
#[cfg(windows)]
mod dynamic_dms_shared_release;
#[cfg(windows)]
mod dynamic_mapping_close;
#[cfg(windows)]
mod dynamic_registration;
#[cfg(windows)]
mod dynamic_shm_file_close;
#[cfg(windows)]
mod dynamic_view_unmap;
mod expected;
mod invariants;
mod model;
mod registration;
mod unmap_delete;
mod unmap_nonfinal;
mod unmap_teardown;

#[cfg(windows)]
pub(super) use dynamic_dms_shared_release::{
    validate_dms_shared_release_after_success_physical_subset, DmsSharedReleasePhysicalSubsetActual,
};
#[cfg(windows)]
pub(super) use dynamic_mapping_close::{
    validate_mapping_close_after_success_physical_subset, MappingClosePhysicalSubsetActual,
};
#[cfg(windows)]
pub(super) use dynamic_registration::{
    validate_dynamic_registration, DynamicRegistrationActual,
    DynamicRegistrationRetainedDisposition, DynamicRegistrationTiming,
};
#[cfg(windows)]
pub(super) use dynamic_shm_file_close::{
    validate_shm_file_close_after_success_physical_subset, ShmFileClosePhysicalSubsetActual,
};
#[cfg(windows)]
pub(super) use dynamic_view_unmap::{
    validate_view_unmap_after_success_physical_subset, ViewUnmapPhysicalSubsetActual,
};

#[test]
fn a2b2_declared_static_inventory_is_exact_and_self_consistent() {
    let cases = invariants::inventory();
    invariants::validate(&cases).expect("A2b2 static inventory contract");
}
