use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

mod fences;
mod guards;
mod precheck;
mod receipt_integrity;
mod tables;
mod v253_active_bridge;
mod v271_projected_source;
mod v274_rebuild;

pub(super) fn register_receipt_integrity_functions(conn: &Connection) -> Result<()> {
    receipt_integrity::register(conn)?;
    crate::store::compute_external_pool_adapter_runtime_bundle::register_external_pool_adapter_atomic_activation_pending_plan_udf(conn)
}

pub(crate) fn migration_v277(conn: &Connection) -> Result<()> {
    register_receipt_integrity_functions(conn)?;
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    let receipt_count_before = precheck::before(&transaction)?;
    tables::create(&transaction)?;
    v274_rebuild::rebuild_if_required(&transaction)?;
    receipt_integrity::install(&transaction)?;
    guards::install(&transaction)?;
    v253_active_bridge::install(&transaction)?;
    v271_projected_source::install(&transaction)?;
    fences::install(&transaction)?;
    precheck::after(&transaction, receipt_count_before)?;
    transaction.commit()?;
    Ok(())
}
