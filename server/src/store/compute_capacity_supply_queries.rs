use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension};

use super::Store;

impl Store {
    pub(crate) fn compute_capacity_supply_occurred_at(
        &self,
        idempotency_scope: &str,
        idempotency_key: &str,
    ) -> Result<Option<String>> {
        if idempotency_scope.trim().is_empty() || idempotency_key.trim().is_empty() {
            bail!("容量发行幂等范围和键不能为空");
        }
        let stored = self
            .conn()?
            .query_row(
                "SELECT event_kind, occurred_at
                   FROM compute_capacity_ledger_transactions
                  WHERE idempotency_scope=?1 AND idempotency_key=?2",
                params![idempotency_scope.trim(), idempotency_key.trim()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        match stored {
            Some((event_kind, _)) if event_kind != "supply_added" => {
                bail!("容量发行幂等键已被其他账本事件使用")
            }
            Some((_, occurred_at)) => Ok(Some(occurred_at)),
            None => Ok(None),
        }
    }
}
