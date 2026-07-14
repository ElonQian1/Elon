use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{
    common::{clean_optional, new_id, now},
    Store,
};

#[derive(Debug, Clone, Serialize)]
pub struct CodexVaultRecord {
    pub user_id: String,
    pub auth_mode: String,
    pub account_hint_hash: Option<String>,
    pub source_device: Option<String>,
    #[serde(skip_serializing)]
    pub ciphertext_b64: String,
    #[serde(skip_serializing)]
    pub nonce_b64: String,
    pub credential_version: i64,
    pub last_backup_at: Option<String>,
    pub last_lease_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodexVaultSlotRecord {
    pub slot_id: String,
    pub user_id: String,
    pub auth_mode: String,
    pub account_hint_hash: Option<String>,
    pub source_device: Option<String>,
    #[serde(skip_serializing)]
    pub ciphertext_b64: String,
    #[serde(skip_serializing)]
    pub nonce_b64: String,
    pub credential_version: i64,
    pub status: String,
    pub priority: i64,
    pub failure_count: i64,
    pub last_backup_at: Option<String>,
    pub last_lease_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Store {
    pub fn upsert_user_codex_credential(
        &self,
        user_id: &str,
        auth_mode: &str,
        account_hint_hash: Option<&str>,
        source_device: Option<&str>,
        ciphertext_b64: &str,
        nonce_b64: &str,
    ) -> Result<CodexVaultRecord> {
        let now = now();
        let source_device = clean_optional(source_device);
        let slot_id = {
            let conn = self.conn()?;
            let slot = upsert_slot(
                &conn,
                user_id,
                auth_mode,
                account_hint_hash,
                source_device,
                ciphertext_b64,
                nonce_b64,
                &now,
            )?;
            conn.execute(
                "INSERT INTO user_codex_credentials (
                   user_id, auth_mode, account_hint_hash, source_device,
                   ciphertext_b64, nonce_b64, credential_version,
                   last_backup_at, created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7, ?7)
                 ON CONFLICT(user_id) DO UPDATE SET
                   auth_mode = excluded.auth_mode,
                   account_hint_hash = excluded.account_hint_hash,
                   source_device = excluded.source_device,
                   ciphertext_b64 = excluded.ciphertext_b64,
                   nonce_b64 = excluded.nonce_b64,
                   credential_version = user_codex_credentials.credential_version + 1,
                   last_backup_at = excluded.last_backup_at,
                   updated_at = excluded.updated_at",
                params![
                    user_id,
                    auth_mode,
                    account_hint_hash,
                    source_device,
                    ciphertext_b64,
                    nonce_b64,
                    now
                ],
            )?;
            slot.slot_id
        };
        self.record_codex_vault_event(user_id, "backup", Some(&slot_id), true, None)?;
        self.get_user_codex_credential(user_id)?
            .ok_or_else(|| anyhow::anyhow!("Codex 凭据保存后无法读取"))
    }

    pub fn list_user_codex_credential_slots(
        &self,
        user_id: &str,
    ) -> Result<Vec<CodexVaultSlotRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT slot_id, user_id, auth_mode, account_hint_hash, source_device,
                    ciphertext_b64, nonce_b64, credential_version, status, priority,
                    failure_count, last_backup_at, last_lease_at, last_failure_at,
                    last_success_at, last_error, created_at, updated_at
               FROM user_codex_credential_slots
              WHERE user_id = ?1
                AND status != 'deleted'
              ORDER BY
                CASE status WHEN 'active' THEN 0 WHEN 'degraded' THEN 1 ELSE 2 END,
                failure_count ASC,
                priority ASC,
                COALESCE(last_lease_at, '') ASC,
                updated_at DESC",
        )?;
        let rows = stmt.query_map(params![user_id], slot_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn select_user_codex_credential_slot(
        &self,
        user_id: &str,
        avoid_account_hint_hash: Option<&str>,
    ) -> Result<Option<CodexVaultSlotRecord>> {
        let avoid = clean_optional(avoid_account_hint_hash);
        let slots = self.list_user_codex_credential_slots(user_id)?;
        Ok(slots.into_iter().find(|slot| {
            matches!(slot.status.as_str(), "active" | "degraded")
                && !avoid.is_some_and(|hint| slot.account_hint_hash.as_deref() == Some(hint))
        }))
    }

    pub fn mark_user_codex_credential_slot_leased(
        &self,
        user_id: &str,
        slot_id: &str,
        node_id: Option<&str>,
    ) -> Result<()> {
        let now = now();
        {
            let conn = self.conn()?;
            conn.execute(
                "UPDATE user_codex_credential_slots
                    SET last_lease_at = ?3,
                        last_success_at = ?3,
                        status = 'active',
                        updated_at = ?3
                  WHERE user_id = ?1 AND slot_id = ?2",
                params![user_id, slot_id, now],
            )?;
        }
        self.record_codex_vault_event(user_id, "lease", node_id.or(Some(slot_id)), true, None)
    }

    pub fn mark_user_codex_credential_slot_failed(
        &self,
        user_id: &str,
        account_hint_hash: Option<&str>,
        error: &str,
    ) -> Result<bool> {
        let Some(account_hint_hash) = clean_optional(account_hint_hash) else {
            return Ok(false);
        };
        let now = now();
        let changed = {
            let conn = self.conn()?;
            conn.execute(
                "UPDATE user_codex_credential_slots
                    SET failure_count = failure_count + 1,
                        status = CASE WHEN failure_count + 1 >= 2 THEN 'degraded' ELSE status END,
                        last_failure_at = ?3,
                        last_error = ?4,
                        updated_at = ?3
                  WHERE user_id = ?1
                    AND account_hint_hash = ?2
                    AND status != 'deleted'",
                params![user_id, account_hint_hash, now, clean_optional(Some(error))],
            )?
        };
        if changed > 0 {
            self.record_codex_vault_event(
                user_id,
                "slot_failure",
                Some(account_hint_hash),
                false,
                Some(error),
            )?;
        }
        Ok(changed > 0)
    }

    pub fn get_user_codex_credential(&self, user_id: &str) -> Result<Option<CodexVaultRecord>> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT user_id, auth_mode, account_hint_hash, source_device,
                    ciphertext_b64, nonce_b64, credential_version,
                    last_backup_at, last_lease_at, created_at, updated_at
               FROM user_codex_credentials
              WHERE user_id = ?1",
            params![user_id],
            |row| {
                Ok(CodexVaultRecord {
                    user_id: row.get(0)?,
                    auth_mode: row.get(1)?,
                    account_hint_hash: row.get(2)?,
                    source_device: row.get(3)?,
                    ciphertext_b64: row.get(4)?,
                    nonce_b64: row.get(5)?,
                    credential_version: row.get(6)?,
                    last_backup_at: row.get(7)?,
                    last_lease_at: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn mark_user_codex_credential_leased(
        &self,
        user_id: &str,
        node_id: Option<&str>,
    ) -> Result<()> {
        let now = now();
        {
            let conn = self.conn()?;
            conn.execute(
                "UPDATE user_codex_credentials
                    SET last_lease_at = ?2,
                        updated_at = ?2
                  WHERE user_id = ?1",
                params![user_id, now],
            )?;
        }
        self.record_codex_vault_event(user_id, "lease", node_id, true, None)
    }

    pub fn delete_user_codex_credential(&self, user_id: &str) -> Result<bool> {
        let (changed, slot_changed) = {
            let conn = self.conn()?;
            let changed = conn.execute(
                "DELETE FROM user_codex_credentials WHERE user_id = ?1",
                params![user_id],
            )?;
            let slot_changed = conn.execute(
                "UPDATE user_codex_credential_slots
                    SET status = 'deleted',
                        updated_at = ?2
                  WHERE user_id = ?1
                    AND status != 'deleted'",
                params![user_id, now()],
            )?;
            (changed, slot_changed)
        };
        self.record_codex_vault_event(user_id, "delete", None, true, None)?;
        Ok(changed > 0 || slot_changed > 0)
    }

    pub fn record_codex_vault_event(
        &self,
        user_id: &str,
        event_type: &str,
        node_id: Option<&str>,
        success: bool,
        error: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO user_codex_credential_events
               (id, user_id, event_type, node_id, success, error, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                new_id("cve"),
                user_id,
                event_type,
                clean_optional(node_id),
                if success { 1 } else { 0 },
                clean_optional(error),
                now()
            ],
        )?;
        Ok(())
    }
}

fn upsert_slot(
    conn: &rusqlite::Connection,
    user_id: &str,
    auth_mode: &str,
    account_hint_hash: Option<&str>,
    source_device: Option<&str>,
    ciphertext_b64: &str,
    nonce_b64: &str,
    now: &str,
) -> Result<CodexVaultSlotRecord> {
    let existing_slot_id = existing_slot_id(conn, user_id, account_hint_hash)?;
    let slot_id = existing_slot_id.unwrap_or_else(|| new_id("cvs"));
    let updated = conn.execute(
        "UPDATE user_codex_credential_slots
            SET auth_mode = ?3,
                account_hint_hash = ?4,
                source_device = ?5,
                ciphertext_b64 = ?6,
                nonce_b64 = ?7,
                credential_version = credential_version + 1,
                status = 'active',
                failure_count = 0,
                last_backup_at = ?8,
                last_error = NULL,
                updated_at = ?8
          WHERE user_id = ?1 AND slot_id = ?2",
        params![
            user_id,
            slot_id,
            auth_mode,
            account_hint_hash,
            source_device,
            ciphertext_b64,
            nonce_b64,
            now
        ],
    )?;
    if updated == 0 {
        conn.execute(
            "INSERT INTO user_codex_credential_slots (
               slot_id, user_id, auth_mode, account_hint_hash, source_device,
               ciphertext_b64, nonce_b64, credential_version, status, priority,
               failure_count, last_backup_at, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 'active', 100, 0, ?8, ?8, ?8)",
            params![
                slot_id,
                user_id,
                auth_mode,
                account_hint_hash,
                source_device,
                ciphertext_b64,
                nonce_b64,
                now
            ],
        )?;
    }
    load_slot(conn, user_id, &slot_id)?
        .ok_or_else(|| anyhow::anyhow!("Codex 凭据槽位保存后无法读取"))
}

fn existing_slot_id(
    conn: &rusqlite::Connection,
    user_id: &str,
    account_hint_hash: Option<&str>,
) -> Result<Option<String>> {
    if let Some(hint) = clean_optional(account_hint_hash) {
        return conn
            .query_row(
                "SELECT slot_id
                   FROM user_codex_credential_slots
                  WHERE user_id = ?1
                    AND account_hint_hash = ?2
                    AND status != 'deleted'
                  ORDER BY updated_at DESC
                  LIMIT 1",
                params![user_id, hint],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into);
    }
    conn.query_row(
        "SELECT slot_id
           FROM user_codex_credential_slots
          WHERE user_id = ?1
            AND account_hint_hash IS NULL
            AND status != 'deleted'
          ORDER BY updated_at DESC
          LIMIT 1",
        params![user_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn load_slot(
    conn: &rusqlite::Connection,
    user_id: &str,
    slot_id: &str,
) -> Result<Option<CodexVaultSlotRecord>> {
    conn.query_row(
        "SELECT slot_id, user_id, auth_mode, account_hint_hash, source_device,
                ciphertext_b64, nonce_b64, credential_version, status, priority,
                failure_count, last_backup_at, last_lease_at, last_failure_at,
                last_success_at, last_error, created_at, updated_at
           FROM user_codex_credential_slots
          WHERE user_id = ?1 AND slot_id = ?2",
        params![user_id, slot_id],
        slot_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn slot_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CodexVaultSlotRecord> {
    Ok(CodexVaultSlotRecord {
        slot_id: row.get(0)?,
        user_id: row.get(1)?,
        auth_mode: row.get(2)?,
        account_hint_hash: row.get(3)?,
        source_device: row.get(4)?,
        ciphertext_b64: row.get(5)?,
        nonce_b64: row.get(6)?,
        credential_version: row.get(7)?,
        status: row.get(8)?,
        priority: row.get(9)?,
        failure_count: row.get(10)?,
        last_backup_at: row.get(11)?,
        last_lease_at: row.get(12)?,
        last_failure_at: row.get(13)?,
        last_success_at: row.get(14)?,
        last_error: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

#[cfg(test)]
#[path = "codex_vault_tests.rs"]
mod codex_vault_tests;
