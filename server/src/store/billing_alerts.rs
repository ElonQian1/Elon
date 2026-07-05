//! Billing reconciliation alerts.
//!
//! These rows make accounting risks durable instead of leaving them as a
//! dashboard-only summary that an operator must happen to inspect.

use anyhow::Result;
use rusqlite::params;
use serde::Serialize;

use super::{new_id, now, Store};

#[derive(Debug, Clone, Serialize)]
pub struct BillingAlertRow {
    pub id: String,
    pub fingerprint: String,
    pub severity: String,
    pub status: String,
    pub title: String,
    pub detail: String,
    pub metric_value: i64,
    pub first_seen_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
}

struct BillingAlertCandidate {
    fingerprint: &'static str,
    severity: &'static str,
    title: &'static str,
    detail: String,
    metric_value: i64,
}

const KNOWN_ALERTS: &[&str] = &[
    "billing:unbilled-events",
    "billing:legacy-events",
    "billing:expired-reservations",
    "billing:negative-balances",
    "billing:duplicate-idempotency-keys",
    "billing:open-reservations-threshold",
    "billing:orphan-compute-token-events",
    "billing:orphan-compute-billing-events",
    "billing:orphan-node-token-events",
    "billing:orphan-node-billing-events",
    "billing:codex-sharing-active-leases-threshold",
    "billing:codex-sharing-expired-uncleared-leases",
    "billing:codex-sharing-accounting-anomalies",
    "billing:codex-sharing-recent-failures",
];

impl Store {
    pub fn refresh_billing_alerts(&self) -> Result<Vec<BillingAlertRow>> {
        let summary = self.admin_billing_reconciliation_summary(7)?;
        let open_threshold =
            configured_i64(self, "billing_open_reservation_alert_threshold", 100).max(1);
        let orphan_token_events = self.count_orphan_compute_token_events()?;
        let orphan_billing_events = self.count_orphan_compute_billing_events()?;
        let orphan_node_token_events = self.count_orphan_node_token_events()?;
        let orphan_node_billing_events = self.count_orphan_node_billing_events()?;
        let codex_active_lease_threshold =
            configured_i64(self, "codex_sharing_active_lease_alert_threshold", 20).max(1);
        let codex_active_leases = self.count_codex_sharing_active_leases()?;
        let codex_expired_uncleared = self.count_codex_sharing_expired_uncleared_leases()?;
        let codex_accounting_anomalies = self.count_codex_sharing_accounting_anomalies()?;
        let codex_recent_failures = self.count_codex_sharing_recent_failures()?;

        let mut candidates = Vec::new();
        push_if_positive(
            &mut candidates,
            "billing:unbilled-events",
            "critical",
            "存在可信用量未扣费",
            summary.unbilled_events,
            "最近 7 天有可信 token/算力事件进入 unbilled_no_balance 状态，需要补余额开通策略或回补扣费。",
        );
        push_if_positive(
            &mut candidates,
            "billing:legacy-events",
            "warning",
            "存在旧版未标记用量",
            summary.legacy_events,
            "最近 7 天仍有 legacy 用量事件，说明还有调用链没有走新的可信记账状态。",
        );
        push_if_positive(
            &mut candidates,
            "billing:expired-reservations",
            "warning",
            "存在过期未结算预授权",
            summary.expired_reservations,
            "有 reserved 预授权已过期但尚未释放，后台清理器或调用释放链路需要检查。",
        );
        push_if_positive(
            &mut candidates,
            "billing:negative-balances",
            "critical",
            "存在负余额用户",
            summary.negative_balance_users,
            "已有用户余额为负数，后续调用会被拦截，但需要确认是否由补扣或预授权差额导致。",
        );
        push_if_positive(
            &mut candidates,
            "billing:duplicate-idempotency-keys",
            "warning",
            "存在重复幂等键用量",
            summary.duplicate_idempotency_keys,
            "同一用户出现重复 idempotency_key，虽然扣费会去重，但调用方需要保持 compute_call_id 唯一。",
        );
        if summary.open_reservations > open_threshold {
            candidates.push(BillingAlertCandidate {
                fingerprint: "billing:open-reservations-threshold",
                severity: "warning",
                title: "预授权冻结数量过高",
                detail: format!(
                    "当前冻结中的预授权 {} 条，超过阈值 {} 条，可能存在调用结束未释放或长任务堆积。",
                    summary.open_reservations, open_threshold
                ),
                metric_value: summary.open_reservations,
            });
        }
        push_if_positive(
            &mut candidates,
            "billing:orphan-compute-token-events",
            "critical",
            "算力明细引用了不存在的 token 事件",
            orphan_token_events,
            "compute_meter_events.token_usage_event_id 找不到对应 token_usage_events，说明明细账和扣费账本出现断链。",
        );
        push_if_positive(
            &mut candidates,
            "billing:orphan-compute-billing-events",
            "critical",
            "算力明细引用了不存在的扣费事件",
            orphan_billing_events,
            "compute_meter_events.billing_event_id 找不到对应 billing_events，说明扣费明细链路需要排查。",
        );
        push_if_positive(
            &mut candidates,
            "billing:orphan-node-token-events",
            "critical",
            "节点收益流水引用了不存在的 token 事件",
            orphan_node_token_events,
            "node_transactions.token_usage_event_id 找不到对应 token_usage_events，说明节点分账和用量账本出现断链。",
        );
        push_if_positive(
            &mut candidates,
            "billing:orphan-node-billing-events",
            "critical",
            "节点收益流水引用了不存在的扣费事件",
            orphan_node_billing_events,
            "node_transactions.billing_event_id 找不到对应 billing_events，说明节点收益没有可靠扣费来源。",
        );
        if codex_active_leases > codex_active_lease_threshold {
            candidates.push(BillingAlertCandidate {
                fingerprint: "billing:codex-sharing-active-leases-threshold",
                severity: "warning",
                title: "Codex 保险箱共享活跃租约过多",
                detail: format!(
                    "当前活跃共享租约 {} 条，超过阈值 {} 条。若不是演练或高峰，需要检查是否有节点未及时清理租约。",
                    codex_active_leases, codex_active_lease_threshold
                ),
                metric_value: codex_active_leases,
            });
        }
        push_if_positive(
            &mut candidates,
            "billing:codex-sharing-expired-uncleared-leases",
            "warning",
            "存在过期未清理的 Codex 共享租约",
            codex_expired_uncleared,
            "Codex 保险箱共享租约已过期但节点未清理，可能导致节点重启后状态误判或运营页面显示异常。",
        );
        push_if_positive(
            &mut candidates,
            "billing:codex-sharing-accounting-anomalies",
            "critical",
            "存在 shared_codex 计费链路缺失",
            codex_accounting_anomalies,
            "共享租约已有 token 消费，但缺少 token_usage、billing_event、node_transaction 或幂等用量明细，需要立即对账。",
        );
        push_if_positive(
            &mut candidates,
            "billing:codex-sharing-recent-failures",
            "warning",
            "过去 24 小时存在 Codex 共享失败",
            codex_recent_failures,
            "Codex 保险箱共享恢复、解密或清理失败，可能影响机器人之间的授权共享可用性。",
        );

        let ts = now();
        let active: Vec<&str> = candidates.iter().map(|alert| alert.fingerprint).collect();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        for alert in &candidates {
            let id = new_id("bal");
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
                    id,
                    alert.fingerprint,
                    alert.severity,
                    alert.title,
                    alert.detail,
                    alert.metric_value,
                    ts,
                ],
            )?;
        }
        for fingerprint in KNOWN_ALERTS {
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
        self.billing_list_alerts(false, 100)
    }

    pub fn billing_list_alerts(
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
             ORDER BY
               CASE status WHEN 'open' THEN 0 ELSE 1 END,
               CASE severity WHEN 'critical' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END,
               updated_at DESC
             LIMIT ?1"
        } else {
            "SELECT id, fingerprint, severity, status, title, detail, metric_value,
                    first_seen_at, updated_at, resolved_at
             FROM billing_alerts
             WHERE status = 'open'
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

    fn count_orphan_compute_token_events(&self) -> Result<i64> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT COUNT(*)
             FROM compute_meter_events c
             LEFT JOIN token_usage_events t ON t.id = c.token_usage_event_id
             WHERE c.token_usage_event_id IS NOT NULL AND t.id IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }

    fn count_orphan_compute_billing_events(&self) -> Result<i64> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT COUNT(*)
             FROM compute_meter_events c
             LEFT JOIN billing_events b ON b.id = c.billing_event_id
             WHERE c.billing_event_id IS NOT NULL AND b.id IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }

    fn count_orphan_node_token_events(&self) -> Result<i64> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT COUNT(*)
             FROM node_transactions n
             LEFT JOIN token_usage_events t ON t.id = n.token_usage_event_id
             WHERE n.token_usage_event_id IS NOT NULL AND t.id IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }

    fn count_orphan_node_billing_events(&self) -> Result<i64> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT COUNT(*)
             FROM node_transactions n
             LEFT JOIN billing_events b ON b.id = n.billing_event_id
             WHERE n.billing_event_id IS NOT NULL AND b.id IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }

    fn count_codex_sharing_active_leases(&self) -> Result<i64> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT COUNT(*)
             FROM codex_vault_emergency_leases
             WHERE status = 'active'
               AND cleared_at IS NULL
               AND expires_at > strftime('%Y-%m-%dT%H:%M:%f+00:00','now')",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }

    fn count_codex_sharing_expired_uncleared_leases(&self) -> Result<i64> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT COUNT(*)
             FROM codex_vault_emergency_leases
             WHERE status = 'active'
               AND cleared_at IS NULL
               AND expires_at <= strftime('%Y-%m-%dT%H:%M:%f+00:00','now')",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }

    fn count_codex_sharing_accounting_anomalies(&self) -> Result<i64> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT COUNT(*)
             FROM codex_vault_emergency_leases l
             WHERE l.total_tokens > 0
               AND (
                 l.token_usage_event_id IS NULL
                 OR l.billing_event_id IS NULL
                 OR l.node_transaction_id IS NULL
                 OR COALESCE(NULLIF(TRIM(l.accounting_status), ''), 'missing') NOT IN ('billed', 'settled')
                 OR NOT EXISTS (
                   SELECT 1
                   FROM codex_vault_emergency_lease_usage_events u
                   WHERE u.lease_id = l.id
                     AND u.token_usage_event_id = l.token_usage_event_id
                 )
               )",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }

    fn count_codex_sharing_recent_failures(&self) -> Result<i64> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT COUNT(*)
             FROM user_codex_credential_events
             WHERE success = 0
               AND (event_type LIKE 'emergency_%' OR event_type LIKE 'sharing_%')
               AND created_at > datetime('now', '-1 day')",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
    }
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

fn push_if_positive(
    candidates: &mut Vec<BillingAlertCandidate>,
    fingerprint: &'static str,
    severity: &'static str,
    title: &'static str,
    metric_value: i64,
    detail: &'static str,
) {
    if metric_value > 0 {
        candidates.push(BillingAlertCandidate {
            fingerprint,
            severity,
            title,
            detail: detail.to_string(),
            metric_value,
        });
    }
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

#[cfg(test)]
mod tests {
    use super::super::codex_vault_emergency::CodexVaultEmergencyLeaseCreate;
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon_billing_alerts_{}.db",
            Uuid::new_v4().simple()
        ));
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn negative_balance_creates_critical_alert() {
        let (store, path) = temp_store();
        let user = store
            .create_user("billing-alert-negative@example.com", "secret1", None, None)
            .unwrap();
        store
            .billing_recharge(&user.id, 1, "test", "test", None)
            .unwrap();
        store
            .conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE user_balance SET balance_fen = -1 WHERE user_id = ?1",
                params![user.id],
            )
            .unwrap();

        let alerts = store.refresh_billing_alerts().unwrap();
        assert!(alerts.iter().any(|alert| {
            alert.fingerprint == "billing:negative-balances" && alert.severity == "critical"
        }));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn codex_sharing_anomalies_create_admin_alerts() {
        let (store, path) = temp_store();
        let provider = store
            .create_user(
                "billing-alert-codex-provider@example.com",
                "secret1",
                Some("provider"),
                None,
            )
            .unwrap();
        let consumer = store
            .create_user(
                "billing-alert-codex-consumer@example.com",
                "secret1",
                Some("consumer"),
                None,
            )
            .unwrap();
        let grant = store
            .upsert_codex_vault_emergency_grant(
                &provider.id,
                &consumer.id,
                Some("provider shares to consumer"),
                Some("robot_codex_vault_shared_access"),
                Some(900),
                None,
                &provider.id,
            )
            .unwrap();
        let lease = store
            .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
                grant_id: &grant.id,
                provider_user_id: &provider.id,
                consumer_user_id: &consumer.id,
                consumer_node_id: "node-consumer",
                provider_slot_id: "slot-provider",
                account_hint_hash: Some("hint-provider"),
                purpose: Some("billing_alert_test"),
                failure_reason: None,
                max_lease_seconds: 900,
            })
            .unwrap();
        store
            .conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE codex_vault_emergency_leases
                    SET expires_at = '2000-01-01T00:00:00+00:00',
                        total_tokens = 42
                  WHERE id = ?1",
                params![lease.id],
            )
            .unwrap();
        store
            .record_codex_vault_event(
                &consumer.id,
                "sharing_restore_failed",
                Some("node-consumer"),
                false,
                Some("unit test failure"),
            )
            .unwrap();

        let alerts = store.refresh_billing_alerts().unwrap();
        assert!(alerts.iter().any(|alert| {
            alert.fingerprint == "billing:codex-sharing-expired-uncleared-leases"
                && alert.severity == "warning"
        }));
        assert!(alerts.iter().any(|alert| {
            alert.fingerprint == "billing:codex-sharing-accounting-anomalies"
                && alert.severity == "critical"
        }));
        assert!(alerts.iter().any(|alert| {
            alert.fingerprint == "billing:codex-sharing-recent-failures"
                && alert.severity == "warning"
        }));

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
