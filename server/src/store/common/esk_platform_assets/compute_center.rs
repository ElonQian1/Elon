//! A single authenticated, read-only snapshot. Never infers ESK from CNY credit.
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use super::{history::scan_authenticated_history_on, sellback::center_summary_on};
use crate::{
    esk_asset::platform::{compute_center::*, sellback::SellbackConfiguration, PlatformError},
    store::Store,
};

const PAGE_SIZE: usize = 20;

impl Store {
    pub(crate) fn esk_compute_center(
        &self,
        user: &str,
        token: &str,
        page: usize,
        config: &SellbackConfiguration,
    ) -> Result<CenterSnapshot> {
        if !(1..=1000).contains(&page) {
            return Err(PlatformError::InvalidInput.into());
        }
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let history = scan_authenticated_history_on(&tx, user, token, PAGE_SIZE, None)?;
        let summary = center_summary_on(&tx, user, token, config)?;
        let asset = CenterAsset {
            total_base_units: summary.total_base_units.to_string(),
            reserved_base_units: summary.reserved_base_units.to_string(),
            remaining_base_units: summary.available_base_units.to_string(),
            entry_count: history.entry_count.to_string(),
            snapshot_digest: history.snapshot_digest,
            history_next_cursor: history.next_cursor,
            entries: history
                .entries
                .into_iter()
                .map(|row| CenterPurchase {
                    entry_id: row.entry_id,
                    allocation_id: row.allocation_id,
                    amount_base_units: row.amount_base_units.to_string(),
                    created_at: row.created_at,
                })
                .collect(),
        };
        let balance: Option<i64> = tx
            .query_row(
                "SELECT balance_fen FROM user_balance WHERE user_id = ?1",
                [user],
                |row| row.get(0),
            )
            .optional()?;
        let month: i64 = tx.query_row(
            "SELECT COALESCE(SUM(cost_rmb_fen),0) FROM billing_events
             WHERE user_id=?1 AND created_at >= strftime('%Y-%m-01','now')",
            [user],
            |row| row.get(0),
        )?;
        let usage_sources = usage_on(&tx, user)?;
        let mut bills = bills_on(&tx, user, (page - 1) * PAGE_SIZE)?;
        let bills_has_more = bills.len() > PAGE_SIZE;
        bills.truncate(PAGE_SIZE);
        let mut holds = holds_on(&tx, user)?;
        let holds_has_more = holds.len() > PAGE_SIZE;
        holds.truncate(PAGE_SIZE);
        tx.commit()?;
        Ok(CenterSnapshot {
            asset,
            legacy_balance_fen: balance.map(|v| v.to_string()),
            month_cost_fen: month.to_string(),
            usage_sources,
            bills,
            bills_has_more,
            holds,
            holds_has_more,
        })
    }
}

fn usage_on(conn: &Connection, user: &str) -> Result<Vec<UsageSource>> {
    let mut stmt = conn.prepare(
        "SELECT CASE WHEN billing_source IN
           ('platform','own_codex','shared_codex','user_api_key','client_reported')
           THEN billing_source ELSE 'other' END AS source,
         COALESCE(SUM(total_tokens),0),COALESCE(SUM(input_tokens),0),
         COALESCE(SUM(cached_input_tokens),0),COALESCE(SUM(output_tokens),0),COUNT(*)
         FROM token_usage_events WHERE user_id=?1
           AND created_at >= strftime('%Y-%m-01','now') GROUP BY source ORDER BY source",
    )?;
    let values = stmt
        .query_map([user], |row| {
            Ok(UsageSource {
                billing_source: row.get(0)?,
                total_tokens: row.get::<_, i64>(1)?.to_string(),
                input_tokens: row.get::<_, i64>(2)?.to_string(),
                cached_input_tokens: row.get::<_, i64>(3)?.to_string(),
                output_tokens: row.get::<_, i64>(4)?.to_string(),
                call_count: row.get::<_, i64>(5)?.to_string(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(values)
}

fn bills_on(conn: &Connection, user: &str, offset: usize) -> Result<Vec<CenterBill>> {
    let mut stmt = conn.prepare(
        "SELECT b.id,b.token_usage_event_id,u.idempotency_key,u.feature,b.model,
          b.input_tokens,b.cached_input_tokens,b.output_tokens,b.cost_rmb_fen,
          b.price_rule_version,b.price_source,b.created_at
         FROM billing_events b LEFT JOIN token_usage_events u
          ON u.id=b.token_usage_event_id AND u.user_id=b.user_id
         WHERE b.user_id=?1 ORDER BY b.created_at DESC,b.id DESC LIMIT 21 OFFSET ?2",
    )?;
    let values = stmt
        .query_map(params![user, offset as i64], |row| {
            Ok(CenterBill {
                id: row.get(0)?,
                token_usage_event_id: row.get(1)?,
                task_reference: row.get(2)?,
                feature: row.get(3)?,
                model: row.get(4)?,
                input_tokens: row.get::<_, i64>(5)?.to_string(),
                cached_input_tokens: row.get::<_, i64>(6)?.to_string(),
                output_tokens: row.get::<_, i64>(7)?.to_string(),
                cost_fen: row.get::<_, i64>(8)?.to_string(),
                price_rule_version: row.get(9)?,
                price_source: row.get(10)?,
                created_at: row.get(11)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(values)
}

fn holds_on(conn: &Connection, user: &str) -> Result<Vec<CenterHold>> {
    let mut stmt = conn.prepare(
        "SELECT id,compute_call_id,feature,model,reserved_fen,status,expires_at
         FROM billing_reservations WHERE user_id=?1
          AND status IN ('reserved','dispatch_hold','verification_hold')
         ORDER BY created_at DESC,id DESC LIMIT 21",
    )?;
    let values = stmt
        .query_map([user], |row| {
            Ok(CenterHold {
                id: row.get(0)?,
                task_reference: row.get(1)?,
                feature: row.get(2)?,
                model: row.get(3)?,
                reserved_fen: row.get::<_, i64>(4)?.to_string(),
                status: row.get(5)?,
                expires_at: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(values)
}
