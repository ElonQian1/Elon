//! Sealed exact-route custody projection for registration shutdown acceptance.

use std::sync::{atomic::Ordering, Arc};

use anyhow::anyhow;

use super::{ManagedSqliteTestVfsRouteCustodySnapshot, ManagedTestVfsRouteCollection, TestRoute};

pub(in super::super) struct ManagedTestRegistrationShutdownRouteSnapshot {
    live_routes: usize,
    logical_names: usize,
    only_route: Option<Arc<TestRoute>>,
    only_route_custody: Option<ManagedSqliteTestVfsRouteCustodySnapshot>,
}

impl ManagedTestRegistrationShutdownRouteSnapshot {
    pub(in super::super) fn live_routes(&self) -> usize {
        self.live_routes
    }

    pub(in super::super) fn logical_names(&self) -> usize {
        self.logical_names
    }

    pub(in super::super) fn only_route(&self) -> Option<&Arc<TestRoute>> {
        self.only_route.as_ref()
    }

    pub(in super::super) fn only_route_custody(
        &self,
    ) -> Option<ManagedSqliteTestVfsRouteCustodySnapshot> {
        self.only_route_custody
    }
}

impl ManagedTestVfsRouteCollection {
    pub(in super::super) fn registration_shutdown_snapshot(
        &self,
    ) -> anyhow::Result<ManagedTestRegistrationShutdownRouteSnapshot> {
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
        let (only_route, only_route_custody) = match live_routes {
            0 => (None, None),
            1 => {
                let first = routes
                    .values()
                    .next()
                    .ok_or_else(|| anyhow!("managed VFS route index lost its only route"))?;
                if !routes
                    .values()
                    .all(|candidate| Arc::ptr_eq(&candidate.entry, &first.entry))
                {
                    return Err(anyhow!("managed VFS route index split one logical route"));
                }
                let route = Arc::clone(first.entry.route());
                let custody = route
                    .registration_shutdown_custody_snapshot()
                    .map_err(|()| anyhow!("observe exact registration route custody"))?;
                (Some(route), Some(custody))
            }
            _ => (None, None),
        };
        Ok(ManagedTestRegistrationShutdownRouteSnapshot {
            live_routes,
            logical_names,
            only_route,
            only_route_custody,
        })
    }
}
