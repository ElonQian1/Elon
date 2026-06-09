//! Compute metering ledger for non-token and token-shaped billable work.
//!
//! Token usage remains the billing source of truth. This ledger adds an
//! inspectable unit breakdown so operators can explain why a call was charged.

use anyhow::Result;
use rusqlite::params;
use serde::Serialize;

use super::{new_id, now, Store};

pub struct ComputeMeterEvent<'a> {
    pub user_id: &'a str,
    pub compute_call_id: Option<&'a str>,
    pub feature: &'a str,
    pub usage_mode: &'a str,
    pub model: Option<&'a str>,
    pub source: &'a str,
    pub input_unit_kind: &'a str,
    pub output_unit_kind: &'a str,
    pub input_units: i64,
    pub output_units: i64,
    pub metered_input_tokens: i64,
    pub metered_output_tokens: i64,
    pub token_usage_event_id: Option<&'a str>,
    pub billing_event_id: Option<&'a str>,
    pub cost_rmb_fen: i64,
    pub accounting_status: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminComputeMeterSummaryRow {
    pub feature: String,
    pub usage_mode: String,
    pub model: Option<String>,
    pub source: String,
    pub input_unit_kind: String,
    pub output_unit_kind: String,
    pub input_units: i64,
    pub output_units: i64,
    pub metered_tokens: i64,
    pub call_count: i64,
    pub billed_cost_rmb_fen: i64,
    pub last_call_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminComputeMeterEventRow {
    pub id: String,
    pub user_id: String,
    pub account: Option<String>,
    pub nickname: Option<String>,
    pub compute_call_id: Option<String>,
    pub feature: String,
    pub usage_mode: String,
    pub model: Option<String>,
    pub source: String,
    pub input_unit_kind: String,
    pub output_unit_kind: String,
    pub input_units: i64,
    pub output_units: i64,
    pub metered_tokens: i64,
    pub cost_rmb_fen: i64,
    pub accounting_status: String,
    pub token_usage_event_id: Option<String>,
    pub billing_event_id: Option<String>,
    pub created_at: String,
}

impl Store {
    pub fn record_compute_meter_event(&self, event: &ComputeMeterEvent<'_>) -> Result<()> {
        let id = new_id("cmp");
        let created = now();
        self.conn()?.execute(
            r#"INSERT INTO compute_meter_events (
               id, user_id, compute_call_id, feature, usage_mode, model, source,
               input_unit_kind, output_unit_kind, input_units, output_units,
               metered_input_tokens, metered_output_tokens, metered_total_tokens,
               token_usage_event_id, billing_event_id, cost_rmb_fen,
               accounting_status, created_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)"#,
            params![
                id,
                event.user_id,
                event.compute_call_id,
                event.feature,
                event.usage_mode,
                event.model,
                event.source,
                event.input_unit_kind,
                event.output_unit_kind,
                event.input_units.max(0),
                event.output_units.max(0),
                event.metered_input_tokens.max(0),
                event.metered_output_tokens.max(0),
                event.metered_input_tokens.max(0) + event.metered_output_tokens.max(0),
                event.token_usage_event_id,
                event.billing_event_id,
                event.cost_rmb_fen.max(0),
                event.accounting_status,
                created,
            ],
        )?;
        Ok(())
    }

    pub fn admin_compute_meter_summary(
        &self,
        days: i64,
        limit: i64,
    ) -> Result<Vec<AdminComputeMeterSummaryRow>> {
        let conn = self.conn()?;
        let days = days.clamp(1, 365);
        let limit = limit.clamp(1, 500);
        let since = format!("-{} days", days);
        let mut stmt = conn.prepare(
            r#"SELECT feature, usage_mode, model, source,
                      input_unit_kind, output_unit_kind,
                      COALESCE(SUM(input_units),0),
                      COALESCE(SUM(output_units),0),
                      COALESCE(SUM(metered_total_tokens),0),
                      COUNT(*),
                      COALESCE(SUM(cost_rmb_fen),0),
                      MAX(created_at)
               FROM compute_meter_events
               WHERE created_at >= datetime('now', ?1)
               GROUP BY feature, usage_mode, model, source, input_unit_kind, output_unit_kind
               ORDER BY 9 DESC, 10 DESC
               LIMIT ?2"#,
        )?;
        let rows = stmt
            .query_map(params![since, limit], |row| {
                Ok(AdminComputeMeterSummaryRow {
                    feature: row.get(0)?,
                    usage_mode: row.get(1)?,
                    model: row.get(2)?,
                    source: row.get(3)?,
                    input_unit_kind: row.get(4)?,
                    output_unit_kind: row.get(5)?,
                    input_units: row.get(6)?,
                    output_units: row.get(7)?,
                    metered_tokens: row.get(8)?,
                    call_count: row.get(9)?,
                    billed_cost_rmb_fen: row.get(10)?,
                    last_call_at: row.get(11)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn admin_compute_meter_events(
        &self,
        days: i64,
        limit: i64,
    ) -> Result<Vec<AdminComputeMeterEventRow>> {
        let conn = self.conn()?;
        let days = days.clamp(1, 365);
        let limit = limit.clamp(1, 500);
        let since = format!("-{} days", days);
        let mut stmt = conn.prepare(
            r#"SELECT c.id, c.user_id, COALESCE(u.phone, u.email), u.nickname,
                      c.compute_call_id, c.feature, c.usage_mode, c.model, c.source,
                      c.input_unit_kind, c.output_unit_kind,
                      c.input_units, c.output_units, c.metered_total_tokens,
                      c.cost_rmb_fen, c.accounting_status,
                      c.token_usage_event_id, c.billing_event_id, c.created_at
               FROM compute_meter_events c
               LEFT JOIN users u ON u.id = c.user_id
               WHERE c.created_at >= datetime('now', ?1)
               ORDER BY c.created_at DESC
               LIMIT ?2"#,
        )?;
        let rows = stmt
            .query_map(params![since, limit], |row| {
                Ok(AdminComputeMeterEventRow {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    account: row.get(2)?,
                    nickname: row.get(3)?,
                    compute_call_id: row.get(4)?,
                    feature: row.get(5)?,
                    usage_mode: row.get(6)?,
                    model: row.get(7)?,
                    source: row.get(8)?,
                    input_unit_kind: row.get(9)?,
                    output_unit_kind: row.get(10)?,
                    input_units: row.get(11)?,
                    output_units: row.get(12)?,
                    metered_tokens: row.get(13)?,
                    cost_rmb_fen: row.get(14)?,
                    accounting_status: row.get(15)?,
                    token_usage_event_id: row.get(16)?,
                    billing_event_id: row.get(17)?,
                    created_at: row.get(18)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}
