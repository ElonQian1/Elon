//! Sealed test-VFS route entry for priming production callback admission overflow.

use super::*;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::types::ManagedSqliteRegistryCallbackCounterPrimeReceipt;

impl<Custody, NonceSource> ManagedSqliteTestVfsRoute<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn prime_lock_callback_counter_overflow_for_test(
        &self,
    ) -> Result<ManagedSqliteRegistryCallbackCounterPrimeReceipt, ()> {
        self.owner
            .prime_lock_callback_counter_overflow_for_test(self.route)
            .map_err(drop)
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy) fn lock_callback_counter_overflow_terminal_for_test(
        &self,
    ) -> Result<bool, ()> {
        self.owner
            .lock_callback_counter_overflow_terminal_for_test(self.route)
            .map_err(drop)
    }
}
