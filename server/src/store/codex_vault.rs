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
        let conn = self.conn()?;
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
        self.record_codex_vault_event(user_id, "backup", None, true, None)?;
        self.get_user_codex_credential(user_id)?
            .ok_or_else(|| anyhow::anyhow!("Codex 凭据保存后无法读取"))
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
        let conn = self.conn()?;
        conn.execute(
            "UPDATE user_codex_credentials
                SET last_lease_at = ?2,
                    updated_at = ?2
              WHERE user_id = ?1",
            params![user_id, now],
        )?;
        self.record_codex_vault_event(user_id, "lease", node_id, true, None)
    }

    pub fn delete_user_codex_credential(&self, user_id: &str) -> Result<bool> {
        let conn = self.conn()?;
        let changed = conn.execute(
            "DELETE FROM user_codex_credentials WHERE user_id = ?1",
            params![user_id],
        )?;
        self.record_codex_vault_event(user_id, "delete", None, true, None)?;
        Ok(changed > 0)
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
