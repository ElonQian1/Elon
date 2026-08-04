use std::collections::BTreeMap;

use anyhow::{bail, Result};
use rusqlite::params;
use serde::Serialize;

use super::Store;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityLedgerHistoryLeg {
    pub line_no: i64,
    pub leg_role: String,
    pub bucket_id: String,
    pub meter: String,
    pub account: String,
    pub delta_units: i64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityLedgerHistoryTransaction {
    pub transaction_id: String,
    pub transaction_digest: String,
    pub delivery_window_id: String,
    pub ledger_sequence: i64,
    pub event_kind: String,
    pub occurred_at: String,
    pub recorded_at: String,
    pub legs: Vec<ComputeCapacityLedgerHistoryLeg>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityLedgerHistoryPage {
    pub pool_id: String,
    pub capacity_epoch: i64,
    pub next_before_sequence: Option<i64>,
    pub transactions: Vec<ComputeCapacityLedgerHistoryTransaction>,
}

impl Store {
    pub(crate) fn list_compute_capacity_ledger_history(
        &self,
        pool_id: &str,
        capacity_epoch: i64,
        before_sequence: Option<i64>,
        limit: usize,
    ) -> Result<ComputeCapacityLedgerHistoryPage> {
        if pool_id.trim().is_empty() {
            bail!("容量池 ID 不能为空");
        }
        if capacity_epoch <= 0 {
            bail!("容量池 epoch 必须为正整数");
        }
        if before_sequence.is_some_and(|value| value <= 0) {
            bail!("容量账本 before_sequence 必须为正整数");
        }
        if !(1..=100).contains(&limit) {
            bail!("容量账本查询数量必须在 1 到 100 之间");
        }
        let limit = i64::try_from(limit)?;
        let conn = self.conn()?;
        let mut transactions = read_transactions(
            &conn,
            pool_id.trim(),
            capacity_epoch,
            before_sequence,
            limit,
        )?;
        if transactions.is_empty() {
            return Ok(ComputeCapacityLedgerHistoryPage {
                pool_id: pool_id.trim().to_string(),
                capacity_epoch,
                next_before_sequence: None,
                transactions,
            });
        }
        let transaction_indexes = transactions
            .iter()
            .enumerate()
            .map(|(index, transaction)| (transaction.transaction_id.clone(), index))
            .collect::<BTreeMap<_, _>>();
        for (transaction_id, leg) in read_legs(
            &conn,
            pool_id.trim(),
            capacity_epoch,
            before_sequence,
            limit,
        )? {
            let Some(index) = transaction_indexes.get(&transaction_id) else {
                bail!("容量账本历史分录引用了分页外事务");
            };
            transactions[*index].legs.push(leg);
        }
        let next_before_sequence = (i64::try_from(transactions.len())? == limit)
            .then(|| transactions.last().map(|item| item.ledger_sequence))
            .flatten();
        Ok(ComputeCapacityLedgerHistoryPage {
            pool_id: pool_id.trim().to_string(),
            capacity_epoch,
            next_before_sequence,
            transactions,
        })
    }
}

fn read_transactions(
    conn: &rusqlite::Connection,
    pool_id: &str,
    capacity_epoch: i64,
    before_sequence: Option<i64>,
    limit: i64,
) -> Result<Vec<ComputeCapacityLedgerHistoryTransaction>> {
    let mut statement = conn.prepare(
        "SELECT transaction_id, transaction_digest, delivery_window_id,
                ledger_sequence, event_kind, occurred_at, recorded_at
           FROM compute_capacity_ledger_transactions
          WHERE pool_id=?1 AND capacity_epoch=?2
            AND (?3 IS NULL OR ledger_sequence<?3)
          ORDER BY ledger_sequence DESC
          LIMIT ?4",
    )?;
    statement
        .query_map(
            params![pool_id, capacity_epoch, before_sequence, limit],
            |row| {
                Ok(ComputeCapacityLedgerHistoryTransaction {
                    transaction_id: row.get(0)?,
                    transaction_digest: row.get(1)?,
                    delivery_window_id: row.get(2)?,
                    ledger_sequence: row.get(3)?,
                    event_kind: row.get(4)?,
                    occurred_at: row.get(5)?,
                    recorded_at: row.get(6)?,
                    legs: Vec::new(),
                })
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn read_legs(
    conn: &rusqlite::Connection,
    pool_id: &str,
    capacity_epoch: i64,
    before_sequence: Option<i64>,
    limit: i64,
) -> Result<Vec<(String, ComputeCapacityLedgerHistoryLeg)>> {
    let mut statement = conn.prepare(
        "SELECT l.transaction_id, l.line_no, l.leg_role, l.bucket_id,
                l.meter, l.account, l.delta_units
           FROM compute_capacity_ledger_legs l
           JOIN compute_capacity_ledger_transactions t
             ON t.transaction_id=l.transaction_id
          WHERE l.transaction_id IN (
                SELECT page.transaction_id
                  FROM compute_capacity_ledger_transactions page
                 WHERE page.pool_id=?1 AND page.capacity_epoch=?2
                   AND (?3 IS NULL OR page.ledger_sequence<?3)
                 ORDER BY page.ledger_sequence DESC
                 LIMIT ?4
          )
          ORDER BY t.ledger_sequence DESC, l.line_no,
                   CASE l.leg_role WHEN 'from' THEN 0 ELSE 1 END",
    )?;
    statement
        .query_map(
            params![pool_id, capacity_epoch, before_sequence, limit],
            |row| {
                Ok((
                    row.get(0)?,
                    ComputeCapacityLedgerHistoryLeg {
                        line_no: row.get(1)?,
                        leg_role: row.get(2)?,
                        bucket_id: row.get(3)?,
                        meter: row.get(4)?,
                        account: row.get(5)?,
                        delta_units: row.get(6)?,
                    },
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}
