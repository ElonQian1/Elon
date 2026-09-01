//! Consuming no-registry-entry seal for q11 raw-state rejection.
//!
//! The route ledger is armed before the controlled representation is installed. Any real registry
//! `Event::Entry`, including an unexpected request, contaminates the entry and prevents this seal.

use super::*;

pub(super) fn finish(
    state: &mut ManagedTestPreManagedLockState,
    route: ManagedTestRouteOrdinal,
) -> Result<ManagedTestPreManagedLockSnapshot, &'static str> {
    if state.entries.get(&route).map(|entry| entry.path)
        != Some(ManagedTestPreManagedLockPath::RawRejected)
    {
        return Err("raw-rejected Lock observation route/path mismatch");
    }
    let entry = state
        .entries
        .remove(&route)
        .ok_or("raw-rejected Lock observation was not armed")?;
    abi_rejected::snapshot_entry(&entry)
}
