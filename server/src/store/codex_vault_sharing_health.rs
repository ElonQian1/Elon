use anyhow::Result;
use rusqlite::params;
use serde::Serialize;

use super::{common::now, Store};

#[derive(Debug, Clone, Serialize)]
pub struct CodexVaultSharingHealth {
    pub status: String,
    pub alert_count: i64,
    pub active_lease_count: i64,
    pub expired_uncleared_count: i64,
    pub accounting_anomaly_count: i64,
    pub unavailable_grant_count: i64,
    pub recent_failed_event_count: i64,
    pub alerts: Vec<CodexVaultSharingAlert>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexVaultSharingAlert {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub count: i64,
}

impl Store {
    pub fn codex_vault_sharing_health(&self, user_id: &str) -> Result<CodexVaultSharingHealth> {
        let current = now();
        let conn = self.conn()?;
        let active_lease_count: i64 = conn.query_row(
            "SELECT COUNT(*)
               FROM codex_vault_emergency_leases
              WHERE (provider_user_id = ?1 OR consumer_user_id = ?1)
                AND status = 'active'
                AND cleared_at IS NULL
                AND expires_at > ?2",
            params![user_id, current],
            |row| row.get(0),
        )?;
        let expired_uncleared_count: i64 = conn.query_row(
            "SELECT COUNT(*)
               FROM codex_vault_emergency_leases
              WHERE (provider_user_id = ?1 OR consumer_user_id = ?1)
                AND status = 'active'
                AND cleared_at IS NULL
                AND expires_at <= ?2",
            params![user_id, current],
            |row| row.get(0),
        )?;
        let accounting_anomaly_count: i64 = conn.query_row(
            "SELECT COUNT(*)
               FROM codex_vault_emergency_leases
              WHERE (provider_user_id = ?1 OR consumer_user_id = ?1)
                AND total_tokens > 0
                AND (
                  token_usage_event_id IS NULL
                  OR billing_event_id IS NULL
                  OR node_transaction_id IS NULL
                  OR COALESCE(NULLIF(TRIM(accounting_status), ''), 'missing') NOT IN ('billed', 'settled')
                )",
            params![user_id],
            |row| row.get(0),
        )?;
        let unavailable_grant_count: i64 = conn.query_row(
            "SELECT COUNT(*)
               FROM codex_vault_emergency_grants g
              WHERE g.provider_user_id = ?1
                AND g.status = 'active'
                AND (g.expires_at IS NULL OR g.expires_at > ?2)
                AND NOT EXISTS (
                  SELECT 1
                    FROM user_codex_credential_slots s
                   WHERE s.user_id = g.provider_user_id
                     AND s.status IN ('active', 'degraded')
                )",
            params![user_id, current],
            |row| row.get(0),
        )?;
        let recent_failed_event_count: i64 = conn.query_row(
            "SELECT COUNT(*)
               FROM user_codex_credential_events
              WHERE user_id = ?1
                AND success = 0
                AND (
                  event_type LIKE 'emergency_%'
                  OR event_type LIKE 'sharing_%'
                )
                AND created_at > datetime('now', '-1 day')",
            params![user_id],
            |row| row.get(0),
        )?;
        let mut alerts = Vec::new();
        push_health_alert(
            &mut alerts,
            expired_uncleared_count,
            "expired_uncleared_lease",
            "warning",
            "存在已过期但未清理的 Codex 保险箱共享租约",
        );
        push_health_alert(
            &mut alerts,
            accounting_anomaly_count,
            "shared_codex_accounting_anomaly",
            "critical",
            "存在 token 用量已产生但缺少完整计费/结算链路的共享租约",
        );
        push_health_alert(
            &mut alerts,
            unavailable_grant_count,
            "shared_provider_vault_unavailable",
            "warning",
            "存在已共享给其他机器人的授权，但本账号没有可用保险箱槽位",
        );
        push_health_alert(
            &mut alerts,
            recent_failed_event_count,
            "recent_sharing_failure",
            "warning",
            "过去 24 小时存在 Codex 保险箱共享失败事件",
        );
        let alert_count = alerts.len() as i64;
        let status = if accounting_anomaly_count > 0 {
            "critical"
        } else if alert_count > 0 {
            "warning"
        } else {
            "ok"
        }
        .to_string();
        Ok(CodexVaultSharingHealth {
            status,
            alert_count,
            active_lease_count,
            expired_uncleared_count,
            accounting_anomaly_count,
            unavailable_grant_count,
            recent_failed_event_count,
            alerts,
        })
    }
}

fn push_health_alert(
    alerts: &mut Vec<CodexVaultSharingAlert>,
    count: i64,
    code: &str,
    severity: &str,
    message: &str,
) {
    if count > 0 {
        alerts.push(CodexVaultSharingAlert {
            code: code.to_string(),
            severity: severity.to_string(),
            message: message.to_string(),
            count,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::codex_vault_emergency::CodexVaultEmergencyLeaseCreate;
    use super::Store;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-codex-sharing-health-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn sharing_health_flags_expired_uncleared_and_accounting_anomalies() {
        let (store, path) = temp_store();
        let provider = store
            .create_user(
                "health-provider@example.com",
                "secret1",
                Some("provider"),
                None,
            )
            .unwrap();
        let consumer = store
            .create_user(
                "health-consumer@example.com",
                "secret1",
                Some("consumer"),
                None,
            )
            .unwrap();
        let grant = store
            .upsert_codex_vault_emergency_grant(
                &provider.id,
                &consumer.id,
                Some("provider to consumer"),
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
                purpose: Some("unit_test"),
                failure_reason: None,
                max_lease_seconds: 900,
            })
            .unwrap();
        {
            let conn = store.conn().unwrap();
            conn.execute(
                "UPDATE codex_vault_emergency_leases
                    SET expires_at = '2000-01-01T00:00:00+00:00',
                        total_tokens = 42
                  WHERE id = ?1",
                rusqlite::params![lease.id],
            )
            .unwrap();
        }

        let health = store.codex_vault_sharing_health(&consumer.id).unwrap();
        assert_eq!(health.status, "critical");
        assert_eq!(health.expired_uncleared_count, 1);
        assert_eq!(health.accounting_anomaly_count, 1);
        assert!(health
            .alerts
            .iter()
            .any(|alert| alert.code == "expired_uncleared_lease"));
        assert!(health
            .alerts
            .iter()
            .any(|alert| alert.code == "shared_codex_accounting_anomaly"));

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
