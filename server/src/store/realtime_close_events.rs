//! Realtime WebSocket close event ledger.
//!
//! The in-process counters are useful for cheap health snapshots, but operators
//! need restart-safe windows such as last hour and last 24 hours. This table
//! keeps a compact close-event stream for those admin views.

use anyhow::Result;
use rusqlite::params;
use serde::Serialize;

use super::{new_id, now, BillingAlertRow, Store};
use crate::realtime_metrics::realtime_diagnostics_catalog;

const REALTIME_CLOSE_EVENT_RETENTION_SECS: i64 = 30 * 24 * 60 * 60;
const REALTIME_ALERT_WINDOW_SECS: i64 = 60 * 60;
const REALTIME_ALERTS: &[&str] = &[
    "realtime:read-errors-last-hour",
    "realtime:write-failures-last-hour",
    "realtime:timeouts-last-hour",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RealtimeCloseMetricRow {
    pub channel: String,
    pub close_reason: String,
    pub count: i64,
}

impl Store {
    pub fn record_realtime_close_event(&self, channel: &str, close_reason: &str) -> Result<()> {
        let now_unix = chrono::Utc::now().timestamp();
        let retention_cutoff = now_unix - REALTIME_CLOSE_EVENT_RETENTION_SECS;
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO realtime_close_events (channel, close_reason, created_at, created_at_unix)
             VALUES (?1, ?2, ?3, ?4)",
            params![channel, close_reason, now(), now_unix],
        )?;
        conn.execute(
            "DELETE FROM realtime_close_events WHERE created_at_unix < ?1",
            params![retention_cutoff],
        )?;
        Ok(())
    }

    pub fn admin_realtime_close_metrics_since(
        &self,
        since_unix: Option<i64>,
    ) -> Result<Vec<RealtimeCloseMetricRow>> {
        let conn = self.conn()?;
        let (sql, bind_since) = if since_unix.is_some() {
            (
                "SELECT channel, close_reason, COUNT(*)
                 FROM realtime_close_events
                 WHERE created_at_unix >= ?1
                 GROUP BY channel, close_reason
                 ORDER BY channel ASC, close_reason ASC",
                true,
            )
        } else {
            (
                "SELECT channel, close_reason, COUNT(*)
                 FROM realtime_close_events
                 GROUP BY channel, close_reason
                 ORDER BY channel ASC, close_reason ASC",
                false,
            )
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = if bind_since {
            stmt.query_map(
                params![since_unix.unwrap_or_default()],
                realtime_metric_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map([], realtime_metric_from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(rows)
    }

    pub fn refresh_realtime_close_alerts(&self) -> Result<Vec<BillingAlertRow>> {
        let now_unix = chrono::Utc::now().timestamp();
        let since_unix = now_unix - REALTIME_ALERT_WINDOW_SECS;
        let read_error_threshold =
            configured_i64(self, "realtime_close_read_error_alert_threshold_1h", 20).max(1);
        let write_failure_threshold =
            configured_i64(self, "realtime_close_write_failure_alert_threshold_1h", 20).max(1);
        let timeout_threshold =
            configured_i64(self, "realtime_close_timeout_alert_threshold_1h", 5).max(1);

        let read_errors = self.count_realtime_close_bucket_since(since_unix, "read_error")?;
        let write_failures = self.count_realtime_close_bucket_since(since_unix, "write_failure")?;
        let timeouts = self.count_realtime_close_bucket_since(since_unix, "timeout")?;

        let mut candidates = Vec::new();
        if read_errors > read_error_threshold {
            candidates.push(RealtimeAlertCandidate {
                fingerprint: "realtime:read-errors-last-hour",
                severity: "critical",
                title: "Realtime WebSocket read errors elevated",
                detail: realtime_alert_detail(
                    "read_error",
                    format!(
                    "过去 1 小时实时通道读错误 {} 次，超过阈值 {} 次；需要检查客户端网络、代理层或服务端 WS 读取链路。",
                    read_errors, read_error_threshold
                    ),
                ),
                metric_value: read_errors,
            });
        }
        if write_failures > write_failure_threshold {
            candidates.push(RealtimeAlertCandidate {
                fingerprint: "realtime:write-failures-last-hour",
                severity: "critical",
                title: "Realtime WebSocket write failures elevated",
                detail: realtime_alert_detail(
                    "write_failure",
                    format!(
                    "过去 1 小时实时通道写失败 {} 次，超过阈值 {} 次；通常意味着客户端半断开、下游发送阻塞或连接保活异常。",
                    write_failures, write_failure_threshold
                    ),
                ),
                metric_value: write_failures,
            });
        }
        if timeouts > timeout_threshold {
            candidates.push(RealtimeAlertCandidate {
                fingerprint: "realtime:timeouts-last-hour",
                severity: "warning",
                title: "Realtime WebSocket timeouts elevated",
                detail: realtime_alert_detail(
                    "timeout",
                    format!(
                    "过去 1 小时实时通道超时 {} 次，超过阈值 {} 次；重点关注 HomeCLI/PC 节点假在线和长连接心跳。",
                    timeouts, timeout_threshold
                    ),
                ),
                metric_value: timeouts,
            });
        }

        let ts = now();
        let active: Vec<&str> = candidates.iter().map(|alert| alert.fingerprint).collect();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        for alert in &candidates {
            tx.execute(
                "INSERT INTO billing_alerts (
                   id, fingerprint, severity, status, title, detail, metric_value,
                   first_seen_at, updated_at
                 ) VALUES (?1,?2,?3,'open',?4,?5,?6,?7,?7)
                 ON CONFLICT(fingerprint) DO UPDATE SET
                   severity = excluded.severity,
                   status = 'open',
                   title = excluded.title,
                   detail = excluded.detail,
                   metric_value = excluded.metric_value,
                   updated_at = excluded.updated_at,
                   resolved_at = NULL",
                params![
                    new_id("ral"),
                    alert.fingerprint,
                    alert.severity,
                    alert.title,
                    alert.detail,
                    alert.metric_value,
                    ts,
                ],
            )?;
        }
        for fingerprint in REALTIME_ALERTS {
            if !active.contains(fingerprint) {
                tx.execute(
                    "UPDATE billing_alerts
                     SET status = 'resolved', resolved_at = ?1, updated_at = ?1
                     WHERE fingerprint = ?2 AND status = 'open'",
                    params![ts, fingerprint],
                )?;
            }
        }
        tx.commit()?;
        drop(conn);
        self.realtime_list_alerts(false, 100)
    }

    pub fn realtime_list_alerts(
        &self,
        include_resolved: bool,
        limit: i64,
    ) -> Result<Vec<BillingAlertRow>> {
        let conn = self.conn()?;
        let limit = limit.clamp(1, 500);
        let sql = if include_resolved {
            "SELECT id, fingerprint, severity, status, title, detail, metric_value,
                    first_seen_at, updated_at, resolved_at
             FROM billing_alerts
             WHERE fingerprint LIKE 'realtime:%'
             ORDER BY
               CASE status WHEN 'open' THEN 0 ELSE 1 END,
               CASE severity WHEN 'critical' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END,
               updated_at DESC
             LIMIT ?1"
        } else {
            "SELECT id, fingerprint, severity, status, title, detail, metric_value,
                    first_seen_at, updated_at, resolved_at
             FROM billing_alerts
             WHERE fingerprint LIKE 'realtime:%' AND status = 'open'
             ORDER BY
               CASE severity WHEN 'critical' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END,
               updated_at DESC
             LIMIT ?1"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt
            .query_map(params![limit], read_alert_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    fn count_realtime_close_bucket_since(
        &self,
        since_unix: i64,
        alert_bucket: &str,
    ) -> Result<i64> {
        let reasons = realtime_close_reasons_for_bucket(alert_bucket);
        self.count_realtime_close_reasons_since(since_unix, &reasons)
    }

    fn count_realtime_close_reasons_since(&self, since_unix: i64, reasons: &[&str]) -> Result<i64> {
        if reasons.is_empty() {
            return Ok(0);
        }
        let placeholders = std::iter::repeat("?")
            .take(reasons.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT COUNT(*)
             FROM realtime_close_events
             WHERE created_at_unix >= ? AND close_reason IN ({placeholders})"
        );
        let mut values = Vec::with_capacity(reasons.len() + 1);
        values.push(rusqlite::types::Value::from(since_unix));
        for reason in reasons {
            values.push(rusqlite::types::Value::from(reason.to_string()));
        }
        self.conn()?
            .query_row(&sql, rusqlite::params_from_iter(values), |row| row.get(0))
            .map_err(Into::into)
    }
}

fn realtime_metric_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RealtimeCloseMetricRow> {
    Ok(RealtimeCloseMetricRow {
        channel: row.get(0)?,
        close_reason: row.get(1)?,
        count: row.get(2)?,
    })
}

struct RealtimeAlertCandidate {
    fingerprint: &'static str,
    severity: &'static str,
    title: &'static str,
    detail: String,
    metric_value: i64,
}

fn configured_i64(store: &Store, key: &str, fallback: i64) -> i64 {
    std::env::var(key.to_ascii_uppercase())
        .ok()
        .and_then(|value| value.trim().parse::<i64>().ok())
        .or_else(|| {
            store
                .billing_get_config(key)
                .ok()
                .flatten()
                .and_then(|value| value.trim().parse::<i64>().ok())
        })
        .unwrap_or(fallback)
}

fn realtime_alert_detail(alert_bucket: &str, base_detail: String) -> String {
    match realtime_first_check_for_bucket(alert_bucket) {
        Some(first_check) => format!("{base_detail} 首查建议：{first_check}"),
        None => base_detail,
    }
}

fn realtime_first_check_for_bucket(alert_bucket: &str) -> Option<&'static str> {
    realtime_diagnostics_catalog()
        .close_reasons
        .iter()
        .find(|reason| reason.alert_bucket == Some(alert_bucket))
        .map(|reason| reason.first_check)
}

fn realtime_close_reasons_for_bucket(alert_bucket: &str) -> Vec<&'static str> {
    realtime_diagnostics_catalog()
        .close_reasons
        .iter()
        .filter(|reason| reason.alert_bucket == Some(alert_bucket))
        .map(|reason| reason.id)
        .collect()
}

fn read_alert_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BillingAlertRow> {
    Ok(BillingAlertRow {
        id: row.get(0)?,
        fingerprint: row.get(1)?,
        severity: row.get(2)?,
        status: row.get(3)?,
        title: row.get(4)?,
        detail: row.get(5)?,
        metric_value: row.get(6)?,
        first_seen_at: row.get(7)?,
        updated_at: row.get(8)?,
        resolved_at: row.get(9)?,
    })
}
