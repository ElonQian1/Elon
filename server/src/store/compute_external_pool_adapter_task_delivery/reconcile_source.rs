//! Exact original-send classification shared by fresh and replayed reconcile closure.

use anyhow::{ensure, Result};
use rusqlite::{params, Connection};

use crate::compute_federation::external_pool_adapter_task_protocol_production::ExternalPoolAdapterTaskExchangeReceiptEnvelope;

pub(super) fn reconcile_source_operation_on(
    connection: &Connection,
    receipt: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
) -> Result<(String, bool)> {
    let command = &receipt.receipt.identity.command;
    let result = connection.query_row(
        "SELECT send.operation_kind,
                EXISTS(SELECT 1 FROM compute_attempt_dispatch_acks ack
                        WHERE ack.command_id=send.command_id AND ack.outcome='accepted'
                          AND ack.disposition='accepted_applied')
           FROM compute_attempt_start_send_attempts send
          WHERE send.send_attempt_id=?1 AND send.send_attempt_digest=?2
            AND send.command_id=?3 AND send.command_digest=?4",
        params![
            command.send_attempt_id,
            command.send_attempt_digest,
            command.command_id,
            command.command_digest
        ],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?)),
    )?;
    ensure!(
        matches!(result.0.as_str(), "prepare" | "commit" | "cancel"),
        "V278 reconcile source operation is invalid"
    );
    Ok(result)
}
