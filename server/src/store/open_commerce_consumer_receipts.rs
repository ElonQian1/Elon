use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use crate::open_commerce_model::OpenCommerceInvocation;

use super::{
    open_commerce_invocations::{invocation_from_row, INVOCATION_SELECT},
    Store,
};

impl Store {
    pub(crate) fn list_user_open_commerce_terminal_invocations(
        &self,
        requester_user_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceInvocation>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{INVOCATION_SELECT}
             WHERE requester_user_id = ?1 AND status IN ('succeeded', 'failed')
             ORDER BY created_at DESC LIMIT ?2"
        ))?;
        let invocations = stmt
            .query_map(
                params![requester_user_id.trim(), limit.clamp(1, 200) as i64],
                invocation_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(invocations)
    }

    pub(crate) fn user_open_commerce_terminal_invocation(
        &self,
        requester_user_id: &str,
        invocation_id: &str,
    ) -> Result<Option<OpenCommerceInvocation>> {
        self.conn()?
            .query_row(
                &format!(
                    "{INVOCATION_SELECT}
                     WHERE requester_user_id = ?1 AND id = ?2
                       AND status IN ('succeeded', 'failed')"
                ),
                params![requester_user_id.trim(), invocation_id.trim()],
                invocation_from_row,
            )
            .optional()
            .map_err(Into::into)
    }
}
