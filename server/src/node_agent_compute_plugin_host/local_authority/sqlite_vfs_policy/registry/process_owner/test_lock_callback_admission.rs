//! Process-owner delegate which preserves the production exact-route rejection path.

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::types::ManagedSqliteRegistryCallbackCounterPrimeReceipt;

impl<Custody, NonceSource> ManagedSqliteRegistryProcessOwner<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn prime_lock_callback_counter_overflow_for_test(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<
        ManagedSqliteRegistryCallbackCounterPrimeReceipt,
        ManagedSqliteRegistryProcessRouteRejection,
    > {
        self.apply_route(route, |routes| {
            routes.prime_lock_callback_counter_overflow_for_test(route)
        })
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn lock_callback_counter_overflow_terminal_for_test(
        &self,
        route: ManagedSqliteRegistryRouteHandle,
    ) -> Result<bool, ManagedSqliteRegistryProcessRouteRejection> {
        match self.lock_routes()?.phase(route) {
            Err(ManagedSqliteRegistryRouteRejection::UnknownOrRetired) => Ok(true),
            Ok(_) | Err(_) => Ok(false),
        }
    }
}
