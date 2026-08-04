use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension};

use super::Store;

impl Store {
    pub(crate) fn compute_capacity_supply_occurred_at(
        &self,
        idempotency_scope: &str,
        idempotency_key: &str,
    ) -> Result<Option<String>> {
        self.compute_capacity_event_occurred_at(
            idempotency_scope,
            idempotency_key,
            "supply_added",
            "容量发行",
        )
    }

    pub(crate) fn compute_capacity_supply_withdrawal_occurred_at(
        &self,
        idempotency_scope: &str,
        idempotency_key: &str,
    ) -> Result<Option<String>> {
        self.compute_capacity_event_occurred_at(
            idempotency_scope,
            idempotency_key,
            "supply_withdrawn",
            "容量撤出",
        )
    }

    fn compute_capacity_event_occurred_at(
        &self,
        idempotency_scope: &str,
        idempotency_key: &str,
        expected_event_kind: &str,
        operation_label: &str,
    ) -> Result<Option<String>> {
        if idempotency_scope.trim().is_empty() || idempotency_key.trim().is_empty() {
            bail!("{operation_label}幂等范围和键不能为空");
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
            Some((event_kind, _)) if event_kind != expected_event_kind => {
                bail!("{operation_label}幂等键已被其他账本事件使用")
            }
            Some((_, occurred_at)) => Ok(Some(occurred_at)),
            None => Ok(None),
        }
    }
}
