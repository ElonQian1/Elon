use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, OptionalExtension, Row};
use sha2::{Digest, Sha256};

use crate::task_settlement::sui_preflight_model::{
    SuiPreflightAdapter, SuiPreflightAdapterIssue, SUI_PREFLIGHT_ADAPTER_ISSUE_SCHEMA,
    SUI_PREFLIGHT_ADAPTER_SCHEMA,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn create_task_sui_preflight_adapter(
        &self,
        project_id: &str,
        display_name: &str,
        allowed_networks: &[String],
        allowed_package_kinds: &[String],
        expires_at: &str,
        created_by_user_id: &str,
    ) -> Result<SuiPreflightAdapterIssue> {
        let token = new_adapter_token();
        let timestamp = now();
        let adapter_id = new_id("sui_preflight_adapter");
        self.conn()?.execute(
            "INSERT INTO task_sui_preflight_adapters (
               id, project_id, display_name, status, allowed_networks_json,
               allowed_package_kinds_json, token_hash, token_hint,
               credential_version, created_by_user_id, last_used_at,
               expires_at, disabled_at, created_at, updated_at
             ) VALUES (
               ?1, ?2, ?3, 'active', ?4, ?5, ?6, ?7, 1, ?8,
               NULL, ?9, NULL, ?10, ?10
             )",
            params![
                adapter_id,
                project_id.trim(),
                display_name.trim(),
                serde_json::to_string(allowed_networks)?,
                serde_json::to_string(allowed_package_kinds)?,
                token_hash(&token),
                token_hint(&token),
                created_by_user_id.trim(),
                expires_at.trim(),
                timestamp,
            ],
        )?;
        let adapter = self.task_sui_preflight_adapter(project_id, &adapter_id)?;
        Ok(SuiPreflightAdapterIssue {
            schema: SUI_PREFLIGHT_ADAPTER_ISSUE_SCHEMA,
            adapter,
            adapter_token: token,
            token_visible_once: true,
        })
    }

    pub(crate) fn rotate_task_sui_preflight_adapter(
        &self,
        project_id: &str,
        adapter_id: &str,
        expires_at: &str,
    ) -> Result<SuiPreflightAdapterIssue> {
        let current = self.task_sui_preflight_adapter(project_id, adapter_id)?;
        if current.status != "active" {
            bail!("已停用的 Sui 预检适配器不能轮换凭据");
        }
        let token = new_adapter_token();
        let timestamp = now();
        let changed = self.conn()?.execute(
            "UPDATE task_sui_preflight_adapters
                SET token_hash=?1, token_hint=?2,
                    credential_version=credential_version+1,
                    last_used_at=NULL, expires_at=?3, updated_at=?4
              WHERE project_id=?5 AND id=?6 AND status='active'",
            params![
                token_hash(&token),
                token_hint(&token),
                expires_at.trim(),
                timestamp,
                project_id.trim(),
                adapter_id.trim(),
            ],
        )?;
        if changed != 1 {
            bail!("Sui 预检适配器凭据轮换发生并发冲突");
        }
        let adapter = self.task_sui_preflight_adapter(project_id, adapter_id)?;
        Ok(SuiPreflightAdapterIssue {
            schema: SUI_PREFLIGHT_ADAPTER_ISSUE_SCHEMA,
            adapter,
            adapter_token: token,
            token_visible_once: true,
        })
    }

    pub(crate) fn disable_task_sui_preflight_adapter(
        &self,
        project_id: &str,
        adapter_id: &str,
    ) -> Result<SuiPreflightAdapter> {
        let current = self.task_sui_preflight_adapter(project_id, adapter_id)?;
        if current.status == "active" {
            let replacement = new_adapter_token();
            let timestamp = now();
            self.conn()?.execute(
                "UPDATE task_sui_preflight_adapters
                    SET status='disabled', token_hash=?1, token_hint='disabled',
                        credential_version=credential_version+1,
                        disabled_at=?2, updated_at=?2
                  WHERE project_id=?3 AND id=?4 AND status='active'",
                params![
                    token_hash(&replacement),
                    timestamp,
                    project_id.trim(),
                    adapter_id.trim(),
                ],
            )?;
        }
        self.task_sui_preflight_adapter(project_id, adapter_id)
    }

    pub(crate) fn list_task_sui_preflight_adapters(
        &self,
        project_id: &str,
    ) -> Result<Vec<SuiPreflightAdapter>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(&format!(
            "{ADAPTER_SELECT} WHERE project_id=?1 ORDER BY updated_at DESC"
        ))?;
        let adapters = statement
            .query_map(params![project_id.trim()], adapter_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(adapters)
    }

    pub(crate) fn task_sui_preflight_adapter(
        &self,
        project_id: &str,
        adapter_id: &str,
    ) -> Result<SuiPreflightAdapter> {
        self.conn()?
            .query_row(
                &format!("{ADAPTER_SELECT} WHERE project_id=?1 AND id=?2"),
                params![project_id.trim(), adapter_id.trim()],
                adapter_from_row,
            )
            .map_err(|error| anyhow!(error).context("Sui 预检适配器不存在"))
    }

    pub(crate) fn authenticate_task_sui_preflight_adapter(
        &self,
        token: &str,
    ) -> Result<SuiPreflightAdapter> {
        let token = token.trim();
        if !token.starts_with("sui_preflight_") || token.len() < 52 {
            bail!("Sui 预检适配器凭据无效");
        }
        self.conn()?
            .query_row(
                &format!(
                    "{ADAPTER_SELECT} WHERE token_hash=?1 AND status='active'
                       AND julianday(expires_at) > julianday(?2)"
                ),
                params![token_hash(token), now()],
                adapter_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("Sui 预检适配器凭据无效、已停用或已到期"))
    }

    pub(crate) fn touch_task_sui_preflight_adapter(&self, adapter_id: &str) -> Result<()> {
        self.conn()?.execute(
            "UPDATE task_sui_preflight_adapters
                SET last_used_at=?1, updated_at=?1
              WHERE id=?2 AND status='active'",
            params![now(), adapter_id.trim()],
        )?;
        Ok(())
    }
}

fn adapter_from_row(row: &Row<'_>) -> rusqlite::Result<SuiPreflightAdapter> {
    let networks_json: String = row.get(4)?;
    let kinds_json: String = row.get(5)?;
    Ok(SuiPreflightAdapter {
        schema: SUI_PREFLIGHT_ADAPTER_SCHEMA,
        id: row.get(0)?,
        project_id: row.get(1)?,
        display_name: row.get(2)?,
        status: row.get(3)?,
        allowed_networks: parse_string_list(&networks_json, 4)?,
        allowed_package_kinds: parse_string_list(&kinds_json, 5)?,
        token_hint: row.get(6)?,
        credential_version: row.get(7)?,
        created_by_user_id: row.get(8)?,
        last_used_at: row.get(9)?,
        expires_at: row.get(10)?,
        is_expired: row.get(11)?,
        disabled_at: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn parse_string_list(value: &str, index: usize) -> rusqlite::Result<Vec<String>> {
    serde_json::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, error.into())
    })
}

fn new_adapter_token() -> String {
    format!(
        "sui_preflight_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn token_hash(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn token_hint(value: &str) -> String {
    format!("...{}", &value[value.len().saturating_sub(6)..])
}

const ADAPTER_SELECT: &str = "SELECT id, project_id, display_name, status, allowed_networks_json,
            allowed_package_kinds_json, token_hint, credential_version,
            created_by_user_id, last_used_at, expires_at,
            (julianday(expires_at) <= julianday('now')) AS is_expired,
            disabled_at, created_at, updated_at
       FROM task_sui_preflight_adapters";
