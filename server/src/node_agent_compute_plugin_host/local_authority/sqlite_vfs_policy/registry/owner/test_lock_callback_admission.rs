//! Exact-route owner delegate for the Lock callback-counter overflow fixture.

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::types::ManagedSqliteRegistryCallbackCounterPrimeReceipt;

impl<Custody> ManagedSqliteRegistryOwner<Custody> {
    pub(super) fn prime_lock_callback_counter_overflow_for_test(
        &mut self,
        handle: ManagedSqliteRegistryRouteHandle,
    ) -> Result<ManagedSqliteRegistryCallbackCounterPrimeReceipt, ManagedSqliteRegistryRouteRejection>
    {
        self.exact_entry_mut(handle)?
            .state
            .prime_lock_callback_counter_overflow_for_test()
            .map_err(ManagedSqliteRegistryRouteRejection::State)
    }
}
