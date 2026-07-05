use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{
    common::{clean_optional, new_id, now},
    Store,
};

#[derive(Debug, Clone, Serialize)]
pub struct CodexVaultEmergencyUserSummary {
    pub id: String,
    pub account: String,
    pub nickname: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexVaultEmergencyGrantRecord {
    pub id: String,
    pub provider_user_id: String,
    pub provider_account: String,
    pub provider_nickname: Option<String>,
    pub consumer_user_id: String,
    pub consumer_account: String,
    pub consumer_nickname: Option<String>,
    pub status: String,
    pub label: Option<String>,
    pub purpose: Option<String>,
    pub max_lease_seconds: i64,
    pub expires_at: Option<String>,
    pub created_by_user_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub revoked_at: Option<String>,
    pub reciprocal_active: bool,
    pub provider_vault_available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexVaultEmergencyLeaseRecord {
    pub id: String,
    pub grant_id: String,
    pub provider_user_id: String,
    pub provider_account: String,
    pub provider_nickname: Option<String>,
    pub consumer_user_id: String,
    pub consumer_account: String,
    pub consumer_nickname: Option<String>,
    pub consumer_node_id: String,
    pub provider_slot_id: String,
    pub account_hint_hash: Option<String>,
    pub purpose: Option<String>,
    pub failure_reason: Option<String>,
    pub billing_source: String,
    pub status: String,
    pub leased_at: String,
    pub expires_at: String,
    pub cleared_at: Option<String>,
    pub token_usage_event_id: Option<String>,
    pub billing_event_id: Option<String>,
    pub node_transaction_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub billed_cost_rmb_fen: i64,
    pub provider_earned_fen: i64,
    pub accounting_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct CodexVaultEmergencyLeaseCreate<'a> {
    pub grant_id: &'a str,
    pub provider_user_id: &'a str,
    pub consumer_user_id: &'a str,
    pub consumer_node_id: &'a str,
    pub provider_slot_id: &'a str,
    pub account_hint_hash: Option<&'a str>,
    pub purpose: Option<&'a str>,
    pub failure_reason: Option<&'a str>,
    pub max_lease_seconds: i64,
}

impl Store {
    pub fn resolve_codex_vault_emergency_user(
        &self,
        query: &str,
    ) -> Result<Option<CodexVaultEmergencyUserSummary>> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(None);
        }
        self.conn()?
            .query_row(
                "SELECT id, COALESCE(email, phone, id), nickname
                   FROM users
                  WHERE status = 'active'
                    AND (id = ?1 OR email = ?1 OR phone = ?1 OR lower(trim(nickname)) = lower(trim(?1)))
                  ORDER BY created_at DESC
                  LIMIT 1",
                params![query],
                read_user_summary,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn upsert_codex_vault_emergency_grant(
        &self,
        provider_user_id: &str,
        consumer_user_id: &str,
        label: Option<&str>,
        purpose: Option<&str>,
        max_lease_seconds: Option<i64>,
        expires_at: Option<&str>,
        created_by_user_id: &str,
    ) -> Result<CodexVaultEmergencyGrantRecord> {
        if provider_user_id == consumer_user_id {
            return Err(anyhow!("不能把 Codex 保险箱应急授权给自己"));
        }
        let ts = now();
        let max_lease_seconds = max_lease_seconds.unwrap_or(900).clamp(60, 7200);
        let label = clean_optional(label);
        let purpose = clean_optional(purpose);
        let expires_at = clean_optional(expires_at);
        let conn = self.conn()?;
        let existing: Option<String> = conn
            .query_row(
                "SELECT id
                   FROM codex_vault_emergency_grants
                  WHERE provider_user_id = ?1
                    AND consumer_user_id = ?2
                    AND status = 'active'
                  LIMIT 1",
                params![provider_user_id, consumer_user_id],
                |row| row.get(0),
            )
            .optional()?;
        let id = existing.unwrap_or_else(|| new_id("cveg"));
        if conn.execute(
            "UPDATE codex_vault_emergency_grants
                    SET label = ?3,
                        purpose = ?4,
                        max_lease_seconds = ?5,
                        expires_at = ?6,
                        updated_at = ?7
                  WHERE id = ?1
                    AND provider_user_id = ?2
                    AND status = 'active'",
            params![
                id,
                provider_user_id,
                label,
                purpose,
                max_lease_seconds,
                expires_at,
                ts
            ],
        )? == 0
        {
            conn.execute(
                "INSERT INTO codex_vault_emergency_grants
                 (id, provider_user_id, consumer_user_id, status, label, purpose,
                  max_lease_seconds, expires_at, created_by_user_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'active', ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                params![
                    id,
                    provider_user_id,
                    consumer_user_id,
                    label,
                    purpose,
                    max_lease_seconds,
                    expires_at,
                    created_by_user_id,
                    ts
                ],
            )?;
        }
        drop(conn);
        self.get_codex_vault_emergency_grant(&id)?
            .ok_or_else(|| anyhow!("应急授权保存后无法读取"))
    }

    pub fn get_codex_vault_emergency_grant(
        &self,
        grant_id: &str,
    ) -> Result<Option<CodexVaultEmergencyGrantRecord>> {
        self.conn()?
            .query_row(
                grant_select_sql("WHERE g.id = ?1").as_str(),
                params![grant_id],
                read_grant_record,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_codex_vault_emergency_grants(
        &self,
        user_id: &str,
    ) -> Result<Vec<CodexVaultEmergencyGrantRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            grant_select_sql(
                "WHERE g.provider_user_id = ?1 OR g.consumer_user_id = ?1
                 ORDER BY
                   CASE g.status WHEN 'active' THEN 0 ELSE 1 END,
                   g.updated_at DESC",
            )
            .as_str(),
        )?;
        let rows = stmt.query_map(params![user_id], read_grant_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn find_active_codex_vault_emergency_grant(
        &self,
        provider_user_id: &str,
        consumer_user_id: &str,
    ) -> Result<Option<CodexVaultEmergencyGrantRecord>> {
        let current = now();
        self.conn()?
            .query_row(
                grant_select_sql(
                    "WHERE g.provider_user_id = ?1
                       AND g.consumer_user_id = ?2
                       AND g.status = 'active'
                       AND (g.expires_at IS NULL OR g.expires_at > ?3)",
                )
                .as_str(),
                params![provider_user_id, consumer_user_id, current],
                read_grant_record,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn revoke_codex_vault_emergency_grant(
        &self,
        grant_id: &str,
        provider_user_id: &str,
    ) -> Result<bool> {
        let ts = now();
        let changed = self.conn()?.execute(
            "UPDATE codex_vault_emergency_grants
                SET status = 'revoked',
                    revoked_at = ?3,
                    updated_at = ?3
              WHERE id = ?1
                AND provider_user_id = ?2
                AND status = 'active'",
            params![grant_id, provider_user_id, ts],
        )?;
        Ok(changed > 0)
    }

    pub fn create_codex_vault_emergency_lease(
        &self,
        p: CodexVaultEmergencyLeaseCreate<'_>,
    ) -> Result<CodexVaultEmergencyLeaseRecord> {
        let id = new_id("cvel");
        let leased_at = now();
        let lease_seconds = p.max_lease_seconds.clamp(60, 7200);
        let expires_at = (Utc::now() + Duration::seconds(lease_seconds)).to_rfc3339();
        self.conn()?.execute(
            "INSERT INTO codex_vault_emergency_leases
             (id, grant_id, provider_user_id, consumer_user_id, consumer_node_id,
              provider_slot_id, account_hint_hash, purpose, failure_reason, billing_source,
              status, leased_at, expires_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'shared_codex',
                     'active', ?10, ?11, ?10, ?10)",
            params![
                id,
                p.grant_id,
                p.provider_user_id,
                p.consumer_user_id,
                p.consumer_node_id,
                p.provider_slot_id,
                clean_optional(p.account_hint_hash),
                clean_optional(p.purpose),
                clean_optional(p.failure_reason),
                leased_at,
                expires_at
            ],
        )?;
        self.get_codex_vault_emergency_lease(&id)?
            .ok_or_else(|| anyhow!("应急租约保存后无法读取"))
    }

    pub fn get_codex_vault_emergency_lease(
        &self,
        lease_id: &str,
    ) -> Result<Option<CodexVaultEmergencyLeaseRecord>> {
        self.conn()?
            .query_row(
                lease_select_sql("WHERE l.id = ?1").as_str(),
                params![lease_id],
                read_lease_record,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn get_active_codex_vault_emergency_lease_for_node(
        &self,
        consumer_user_id: &str,
        consumer_node_id: &str,
    ) -> Result<Option<CodexVaultEmergencyLeaseRecord>> {
        let current = now();
        self.conn()?
            .query_row(
                lease_select_sql(
                    "WHERE l.consumer_user_id = ?1
                       AND l.consumer_node_id = ?2
                       AND l.status = 'active'
                       AND l.expires_at > ?3
                     ORDER BY l.leased_at DESC
                     LIMIT 1",
                )
                .as_str(),
                params![consumer_user_id, consumer_node_id, current],
                read_lease_record,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_codex_vault_emergency_leases(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<CodexVaultEmergencyLeaseRecord>> {
        let limit = limit.clamp(1, 100);
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            lease_select_sql(
                "WHERE l.provider_user_id = ?1 OR l.consumer_user_id = ?1
                 ORDER BY l.leased_at DESC
                 LIMIT ?2",
            )
            .as_str(),
        )?;
        let rows = stmt.query_map(params![user_id, limit], read_lease_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn attach_codex_vault_emergency_usage(
        &self,
        lease_id: &str,
        token_usage_event_id: Option<&str>,
        billing_event_id: Option<&str>,
        node_transaction_id: Option<&str>,
        input_tokens: i64,
        output_tokens: i64,
        billed_cost_rmb_fen: i64,
        provider_earned_fen: i64,
        accounting_status: Option<&str>,
    ) -> Result<bool> {
        let total_tokens = input_tokens.max(0) + output_tokens.max(0);
        let ts = now();
        let changed = self.conn()?.execute(
            "UPDATE codex_vault_emergency_leases
                SET token_usage_event_id = COALESCE(?2, token_usage_event_id),
                    billing_event_id = COALESCE(?3, billing_event_id),
                    node_transaction_id = COALESCE(?4, node_transaction_id),
                    input_tokens = input_tokens + ?5,
                    output_tokens = output_tokens + ?6,
                    total_tokens = total_tokens + ?7,
                    billed_cost_rmb_fen = billed_cost_rmb_fen + ?8,
                    provider_earned_fen = provider_earned_fen + ?9,
                    accounting_status = COALESCE(?10, accounting_status),
                    updated_at = ?11
              WHERE id = ?1",
            params![
                lease_id,
                clean_optional(token_usage_event_id),
                clean_optional(billing_event_id),
                clean_optional(node_transaction_id),
                input_tokens.max(0),
                output_tokens.max(0),
                total_tokens,
                billed_cost_rmb_fen.max(0),
                provider_earned_fen.max(0),
                clean_optional(accounting_status),
                ts
            ],
        )?;
        Ok(changed > 0)
    }
}

fn grant_select_sql(where_clause: &str) -> String {
    format!(
        "SELECT
           g.id, g.provider_user_id, COALESCE(p.email, p.phone, p.id), p.nickname,
           g.consumer_user_id, COALESCE(c.email, c.phone, c.id), c.nickname,
           g.status, g.label, g.purpose, g.max_lease_seconds, g.expires_at,
           g.created_by_user_id, g.created_at, g.updated_at, g.revoked_at,
           EXISTS(
             SELECT 1 FROM codex_vault_emergency_grants r
              WHERE r.provider_user_id = g.consumer_user_id
                AND r.consumer_user_id = g.provider_user_id
                AND r.status = 'active'
                AND (r.expires_at IS NULL OR r.expires_at > strftime('%Y-%m-%dT%H:%M:%f+00:00','now'))
           ),
           EXISTS(
             SELECT 1 FROM user_codex_credential_slots s
              WHERE s.user_id = g.provider_user_id
                AND s.status IN ('active', 'degraded')
           )
         FROM codex_vault_emergency_grants g
         JOIN users p ON p.id = g.provider_user_id
         JOIN users c ON c.id = g.consumer_user_id
         {where_clause}"
    )
}

fn lease_select_sql(where_clause: &str) -> String {
    format!(
        "SELECT
           l.id, l.grant_id,
           l.provider_user_id, COALESCE(p.email, p.phone, p.id), p.nickname,
           l.consumer_user_id, COALESCE(c.email, c.phone, c.id), c.nickname,
           l.consumer_node_id, l.provider_slot_id, l.account_hint_hash,
           l.purpose, l.failure_reason, l.billing_source, l.status,
           l.leased_at, l.expires_at, l.cleared_at,
           l.token_usage_event_id, l.billing_event_id, l.node_transaction_id,
           l.input_tokens, l.output_tokens, l.total_tokens,
           l.billed_cost_rmb_fen, l.provider_earned_fen, l.accounting_status,
           l.created_at, l.updated_at
         FROM codex_vault_emergency_leases l
         JOIN users p ON p.id = l.provider_user_id
         JOIN users c ON c.id = l.consumer_user_id
         {where_clause}"
    )
}

fn read_user_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<CodexVaultEmergencyUserSummary> {
    Ok(CodexVaultEmergencyUserSummary {
        id: row.get(0)?,
        account: row.get(1)?,
        nickname: row.get(2)?,
    })
}

fn read_grant_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<CodexVaultEmergencyGrantRecord> {
    Ok(CodexVaultEmergencyGrantRecord {
        id: row.get(0)?,
        provider_user_id: row.get(1)?,
        provider_account: row.get(2)?,
        provider_nickname: row.get(3)?,
        consumer_user_id: row.get(4)?,
        consumer_account: row.get(5)?,
        consumer_nickname: row.get(6)?,
        status: row.get(7)?,
        label: row.get(8)?,
        purpose: row.get(9)?,
        max_lease_seconds: row.get(10)?,
        expires_at: row.get(11)?,
        created_by_user_id: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        revoked_at: row.get(15)?,
        reciprocal_active: row.get::<_, i64>(16)? != 0,
        provider_vault_available: row.get::<_, i64>(17)? != 0,
    })
}

fn read_lease_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<CodexVaultEmergencyLeaseRecord> {
    Ok(CodexVaultEmergencyLeaseRecord {
        id: row.get(0)?,
        grant_id: row.get(1)?,
        provider_user_id: row.get(2)?,
        provider_account: row.get(3)?,
        provider_nickname: row.get(4)?,
        consumer_user_id: row.get(5)?,
        consumer_account: row.get(6)?,
        consumer_nickname: row.get(7)?,
        consumer_node_id: row.get(8)?,
        provider_slot_id: row.get(9)?,
        account_hint_hash: row.get(10)?,
        purpose: row.get(11)?,
        failure_reason: row.get(12)?,
        billing_source: row.get(13)?,
        status: row.get(14)?,
        leased_at: row.get(15)?,
        expires_at: row.get(16)?,
        cleared_at: row.get(17)?,
        token_usage_event_id: row.get(18)?,
        billing_event_id: row.get(19)?,
        node_transaction_id: row.get(20)?,
        input_tokens: row.get(21)?,
        output_tokens: row.get(22)?,
        total_tokens: row.get(23)?,
        billed_cost_rmb_fen: row.get(24)?,
        provider_earned_fen: row.get(25)?,
        accounting_status: row.get(26)?,
        created_at: row.get(27)?,
        updated_at: row.get(28)?,
    })
}

#[cfg(test)]
mod tests {
    use super::CodexVaultEmergencyLeaseCreate;
    use crate::store::Store;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-codex-emergency-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn grant_reciprocal_and_active_lease_are_queryable() {
        let (store, path) = temp_store();
        let a = store
            .create_user("vault-a@example.com", "secret1", Some("A"), None)
            .unwrap();
        let b = store
            .create_user("vault-b@example.com", "secret1", Some("B"), None)
            .unwrap();
        let ab = store
            .upsert_codex_vault_emergency_grant(
                &a.id,
                &b.id,
                Some("A to B"),
                Some("test"),
                Some(900),
                None,
                &a.id,
            )
            .unwrap();
        store
            .upsert_codex_vault_emergency_grant(
                &b.id,
                &a.id,
                Some("B to A"),
                Some("test"),
                Some(900),
                None,
                &b.id,
            )
            .unwrap();

        let grants = store.list_codex_vault_emergency_grants(&a.id).unwrap();
        assert_eq!(grants.len(), 2);
        assert!(grants.iter().any(|grant| grant.reciprocal_active));

        let lease = store
            .create_codex_vault_emergency_lease(CodexVaultEmergencyLeaseCreate {
                grant_id: &ab.id,
                provider_user_id: &a.id,
                consumer_user_id: &b.id,
                consumer_node_id: "node-b",
                provider_slot_id: "slot-a",
                account_hint_hash: Some("hint-a"),
                purpose: Some("unit_test"),
                failure_reason: None,
                max_lease_seconds: 900,
            })
            .unwrap();
        assert_eq!(lease.billing_source, "shared_codex");
        let active = store
            .get_active_codex_vault_emergency_lease_for_node(&b.id, "node-b")
            .unwrap()
            .expect("active lease");
        assert_eq!(active.provider_user_id, a.id);

        store
            .attach_codex_vault_emergency_usage(
                &lease.id,
                Some("tok_1"),
                Some("bill_1"),
                Some("ntx_1"),
                100,
                50,
                7,
                5,
                Some("billed"),
            )
            .unwrap();
        let updated = store
            .get_codex_vault_emergency_lease(&lease.id)
            .unwrap()
            .unwrap();
        assert_eq!(updated.total_tokens, 150);
        assert_eq!(updated.billed_cost_rmb_fen, 7);

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
