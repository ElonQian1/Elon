//! Registry-lifecycle retirement for one closed managed VFS route.

use std::sync::{atomic::Ordering, Arc};

use anyhow::anyhow;

use super::{
    ManagedTestLifecycleFaultPhase, ManagedTestLogicalRouteRemovalReceipt,
    ManagedTestVfsRouteCollection, ManagedTestVfsRouteEntry,
};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::ManagedSqliteRegistryLifecycleStage;

pub(super) fn retire_closed_route(
    routes_collection: &ManagedTestVfsRouteCollection,
    entry: &Arc<ManagedTestVfsRouteEntry>,
) -> anyhow::Result<ManagedTestLogicalRouteRemovalReceipt> {
    let lifecycle = entry.lifecycle();
    let retirement = lifecycle
        .claim_retirement()
        .map_err(|()| anyhow!("managed VFS registry retirement receipt missing"))?;
    if lifecycle
        .before(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval)
        .unwrap_or(true)
    {
        lifecycle.retain_registry_retirement(retirement);
        return Err(anyhow!("injected before managed VFS logical route removal"));
    }
    if entry.custody_drops() != 1 {
        lifecycle.native_failure(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval);
        lifecycle.retain_registry_retirement(retirement);
        return Err(anyhow!(
            "managed VFS route custody was not retired exactly once"
        ));
    }
    let mut routes = match routes_collection.by_name.lock() {
        Ok(routes) => routes,
        Err(_) => {
            lifecycle.native_failure(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval);
            lifecycle.retain_registry_retirement(retirement);
            return Err(anyhow!("managed VFS logical route index poisoned"));
        }
    };
    for name in &entry.exact_names {
        let Some(candidate) = routes.get(name.as_slice()) else {
            drop(routes);
            lifecycle.native_failure(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval);
            lifecycle.retain_registry_retirement(retirement);
            return Err(anyhow!("managed VFS exact route name already retired"));
        };
        if !Arc::ptr_eq(&candidate.entry, entry) {
            drop(routes);
            lifecycle.native_failure(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval);
            lifecycle.retain_registry_retirement(retirement);
            return Err(anyhow!("managed VFS exact route identity mismatch"));
        }
    }
    let live_before = routes_collection.live_routes.load(Ordering::SeqCst);
    let Some(live_after) = live_before.checked_sub(1) else {
        drop(routes);
        lifecycle.native_failure(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval);
        lifecycle.retain_registry_retirement(retirement);
        return Err(anyhow!("managed VFS live route count underflow"));
    };
    if lifecycle
        .observe_registry_lifecycle_stage(
            ManagedSqliteRegistryLifecycleStage::LogicalRemovalAttempt,
        )
        .is_err()
    {
        drop(routes);
        lifecycle.retain_registry_retirement(retirement);
        return Err(anyhow!(
            "managed VFS logical route lifecycle ledger rejected action"
        ));
    }
    if lifecycle
        .claim_native_failure_gate(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval)
        .unwrap_or(true)
    {
        drop(routes);
        lifecycle.native_failure(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval);
        lifecycle.retain_registry_retirement(retirement);
        return Err(anyhow!("managed VFS logical route index rejected removal"));
    }
    let removed = [
        routes
            .remove(entry.exact_names[0].as_slice())
            .expect("validated main route remains present"),
        routes
            .remove(entry.exact_names[1].as_slice())
            .expect("validated journal route remains present"),
        routes
            .remove(entry.exact_names[2].as_slice())
            .expect("validated WAL route remains present"),
    ];
    routes_collection
        .live_routes
        .store(live_after, Ordering::SeqCst);
    drop(routes);
    let receipt = ManagedTestLogicalRouteRemovalReceipt {
        _registry: retirement,
        _removed: removed,
        _live_before: live_before,
        _live_after: live_after,
    };
    if lifecycle
        .observe_registry_lifecycle_stage(
            ManagedSqliteRegistryLifecycleStage::LogicalRemovalSucceeded { removed_names: 3 },
        )
        .is_err()
    {
        lifecycle.retain_logical_removal(receipt);
        return Err(anyhow!(
            "managed VFS logical route lifecycle ledger rejected completion"
        ));
    }
    if lifecycle
        .after_success(ManagedTestLifecycleFaultPhase::LogicalRouteRemoval)
        .unwrap_or(true)
    {
        lifecycle.retain_logical_removal(receipt);
        return Err(anyhow!("injected after managed VFS logical route removal"));
    }
    Ok(receipt)
}
