//! Redacted exact-route logical-index observation for Barrier dynamic evidence.

use std::sync::{atomic::Ordering, Arc};

use anyhow::anyhow;

use super::{
    ManagedSqliteLogicalFileRole, ManagedTestVfsRouteCollection, ManagedTestVfsRouteEntry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ManagedTestBarrierLogicalRouteSnapshot {
    live_routes: usize,
    logical_names: usize,
    exact_route_names: usize,
}

impl ManagedTestBarrierLogicalRouteSnapshot {
    pub(in super::super) fn live_routes(self) -> usize {
        self.live_routes
    }

    pub(in super::super) fn logical_names(self) -> usize {
        self.logical_names
    }

    pub(in super::super) fn exact_route_names(self) -> usize {
        self.exact_route_names
    }
}

impl ManagedTestVfsRouteCollection {
    pub(in super::super) fn barrier_logical_route_snapshot(
        &self,
        entry: &Arc<ManagedTestVfsRouteEntry>,
    ) -> anyhow::Result<ManagedTestBarrierLogicalRouteSnapshot> {
        let routes = self
            .by_name
            .lock()
            .map_err(|_| anyhow!("managed VFS logical route index poisoned"))?;
        let live_routes = self.live_routes.load(Ordering::SeqCst);
        let logical_names = live_routes
            .checked_mul(3)
            .ok_or_else(|| anyhow!("managed VFS logical route count overflow"))?;
        if routes.len() != logical_names {
            return Err(anyhow!("managed VFS logical route index count mismatch"));
        }
        let mut exact_route_names = 0;
        for (name, role) in entry.exact_names.iter().zip([
            ManagedSqliteLogicalFileRole::Main,
            ManagedSqliteLogicalFileRole::Journal,
            ManagedSqliteLogicalFileRole::Wal,
        ]) {
            let Some(candidate) = routes.get(name.as_slice()) else {
                continue;
            };
            if !Arc::ptr_eq(&candidate.entry, entry) || candidate.role != role {
                return Err(anyhow!("managed VFS exact logical route binding changed"));
            }
            exact_route_names += 1;
        }
        Ok(ManagedTestBarrierLogicalRouteSnapshot {
            live_routes,
            logical_names,
            exact_route_names,
        })
    }
}
