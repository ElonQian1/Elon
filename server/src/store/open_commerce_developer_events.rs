use anyhow::{anyhow, Context, Result};
use rusqlite::{params, OptionalExtension};

use crate::open_commerce_developer_event_model::DeveloperTerminalEventRecord;

use super::{
    open_commerce_invocations::{invocation_from_row, INVOCATION_COLUMN_COUNT, INVOCATION_SELECT},
    Store,
};

impl Store {
    pub(crate) fn list_open_commerce_developer_terminal_events(
        &self,
        owner_user_id: &str,
        app_id: &str,
        credential_environment: &str,
        after_sequence: i64,
        limit: usize,
    ) -> Result<Vec<DeveloperTerminalEventRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT i.*, e.seq
               FROM open_commerce_invocation_terminal_events e
               JOIN ({INVOCATION_SELECT}) i ON i.id = e.invocation_id
              WHERE i.requester_user_id = ?1
                AND i.requester_app_id = ?2
                AND (
                  i.credential_environment = ?3
                  OR (?3 = 'sandbox' AND i.credential_environment = 'legacy')
                )
                AND e.seq > ?4
              ORDER BY e.seq ASC
              LIMIT ?5"
        ))?;
        let records = stmt
            .query_map(
                params![
                    owner_user_id.trim(),
                    app_id.trim(),
                    credential_environment.trim(),
                    after_sequence.max(0),
                    limit.clamp(1, 101) as i64
                ],
                |row| {
                    Ok(DeveloperTerminalEventRecord {
                        sequence: row.get(INVOCATION_COLUMN_COUNT)?,
                        invocation: invocation_from_row(row)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(records)
    }

    pub(crate) fn open_commerce_developer_terminal_event(
        &self,
        owner_user_id: &str,
        app_id: &str,
        credential_environment: &str,
        invocation_id: &str,
    ) -> Result<Option<DeveloperTerminalEventRecord>> {
        self.conn()?
            .query_row(
                &format!(
                    "SELECT i.*, e.seq
                       FROM open_commerce_invocation_terminal_events e
                       JOIN ({INVOCATION_SELECT}) i ON i.id = e.invocation_id
                      WHERE i.requester_user_id = ?1
                        AND i.requester_app_id = ?2
                        AND (
                          i.credential_environment = ?3
                          OR (?3 = 'sandbox' AND i.credential_environment = 'legacy')
                        )
                        AND i.id = ?4"
                ),
                params![
                    owner_user_id.trim(),
                    app_id.trim(),
                    credential_environment.trim(),
                    invocation_id.trim()
                ],
                |row| {
                    Ok(DeveloperTerminalEventRecord {
                        sequence: row.get(INVOCATION_COLUMN_COUNT)?,
                        invocation: invocation_from_row(row)?,
                    })
                },
            )
            .optional()
            .map_err(|error| anyhow!(error).context("读取开发者调用事件失败"))
    }
}
