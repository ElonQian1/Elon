use anyhow::{ensure, Result};
use rusqlite::{functions::FunctionFlags, Connection};

use crate::compute_federation::external_pool_adapter_route_renewal::route_renewal_json_is_canonical;
use crate::store::compute_external_pool_adapter_route_renewal::RECEIPT_COLUMNS;

const RECEIPT_EXACT: &str = "elon_v278_external_pool_adapter_route_renewal_receipt_is_exact";
const PENDING: &str = "elon_v278_external_pool_adapter_route_renewal_pending_plan_matches";

pub(super) fn register(connection: &Connection) -> Result<()> {
    connection.create_scalar_function(
        RECEIPT_EXACT,
        1,
        FunctionFlags::SQLITE_UTF8
            | FunctionFlags::SQLITE_DETERMINISTIC
            | FunctionFlags::SQLITE_INNOCUOUS,
        |context| {
            Ok(i64::from(
                context
                    .get_raw(0)
                    .as_str()
                    .ok()
                    .is_some_and(route_renewal_json_is_canonical),
            ))
        },
    )?;
    Ok(())
}

pub(super) fn install(connection: &Connection) -> Result<()> {
    connection.execute_batch(include_str!("guards/receipts.sql"))?;
    let receipt_columns = RECEIPT_COLUMNS.split(',').collect::<Vec<_>>();
    ensure!(
        receipt_columns.len() == 77 && receipt_columns.iter().all(|column| !column.is_empty()),
        "V278 receipt pending guard projection is not exact"
    );
    let pending_arguments = receipt_columns
        .into_iter()
        .map(|column| format!("NEW.{column}"))
        .collect::<Vec<_>>()
        .join(",");
    connection.execute_batch(&format!(
        "DROP TRIGGER IF EXISTS v278_route_renewal_receipt_integrity;
         CREATE TRIGGER v278_route_renewal_receipt_integrity
         BEFORE INSERT ON compute_external_pool_adapter_route_renewal_receipts
         WHEN {RECEIPT_EXACT}(NEW.route_renewal_receipt_json) IS NOT 1
           OR {PENDING}('route_renewal_receipt',{pending_arguments}) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V278 route-renewal canonical/pending integrity mismatch'); END;"
    ))?;
    Ok(())
}
