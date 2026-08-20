use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod fences;
mod historical_accepted;
mod historical_poll_exchange;
mod no_start;
mod receipt_integrity;
mod tables;
mod terminal_ack;
mod v271_active_source;
mod v274_refresh;

pub(super) fn register_receipt_integrity_functions(connection: &Connection) -> Result<()> {
    receipt_integrity::register(connection)?;
    crate::store::compute_external_pool_adapter_route_renewal::register_external_pool_adapter_route_renewal_pending_plan_function(connection)?;
    crate::store::compute_external_pool_adapter_runtime_bundle::register_external_pool_adapter_provider_active_successor_refresh_pending_plan_udf(connection)?;
    crate::store::register_external_pool_adapter_task_reachability_pending_plan_function(connection)
}

pub(crate) fn migration_v278(connection: &Connection) -> Result<()> {
    register_receipt_integrity_functions(connection)?;
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)?;
    tables::create(&transaction)?;
    receipt_integrity::install(&transaction)?;
    fences::install(&transaction)?;
    v271_active_source::install(&transaction)?;
    v274_refresh::install(&transaction)?;
    super::compute_external_pool_adapter_task_delivery::install_v278_reachability_guards(
        &transaction,
    )?;
    terminal_ack::install(&transaction)?;
    no_start::install(&transaction)?;
    historical_accepted::install(&transaction)?;
    historical_poll_exchange::install(&transaction)?;
    transaction.commit()?;
    Ok(())
}
