use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, OptionalExtension, Row};
use sha2::{Digest, Sha256};

use crate::open_commerce_integration_model::{
    normalize_connection_mode, normalize_integration_key, normalize_provider_key,
    normalize_receipt_key, normalize_string_list, normalize_sync_kind, normalize_sync_status,
    CreateIntegrationRequest, OpenCommerceIntegration, OpenCommerceSyncReceipt,
    RecordSyncReceiptRequest, INTEGRATION_STATUS_CONFIGURED, INTEGRATION_STATUS_CONNECTED,
    INTEGRATION_STATUS_DEGRADED, INTEGRATION_STATUS_DISABLED,
};
use crate::open_commerce_model::validate_display_name;

use super::{new_id, now, Store};

pub(crate) struct RecordOpenCommerceSyncReceipt<'a> {
    pub project_id: &'a str,
    pub actor_user_id: &'a str,
    pub actor_app_id: &'a str,
    pub request: RecordSyncReceiptRequest,
}

impl Store {
    pub(crate) fn create_open_commerce_integration(
        &self,
        project_id: &str,
        actor_user_id: &str,
        request: CreateIntegrationRequest,
    ) -> Result<OpenCommerceIntegration> {
        self.open_commerce_merchant_for_project(project_id, &request.merchant_id)?;
        let id = new_id("integration");
        let timestamp = now();
        let integration_key = normalize_integration_key(&request.integration_key)?;
        let provider_key = normalize_provider_key(&request.provider_key)?;
        let display_name = validate_display_name(&request.display_name, "数据接入名称")?;
        let connection_mode = normalize_connection_mode(&request.connection_mode)?;
        let scopes = normalize_string_list(&request.scopes, "授权范围", 32)?;
        let data_domains = normalize_string_list(&request.data_domains, "数据域", 32)?;
        self.conn()?
            .execute(
                "INSERT INTO open_commerce_integrations (
                    id, project_id, merchant_id, integration_key, provider_key,
                    display_name, connection_mode, status, scopes_json,
                    data_domains_json, created_by_user_id, last_verified_at,
                    last_sync_at, created_at, updated_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                    NULL, NULL, ?12, ?12
                 )",
                params![
                    id,
                    project_id.trim(),
                    request.merchant_id.trim(),
                    integration_key,
                    provider_key,
                    display_name,
                    connection_mode,
                    INTEGRATION_STATUS_CONFIGURED,
                    serde_json::to_string(&scopes)?,
                    serde_json::to_string(&data_domains)?,
                    actor_user_id.trim(),
                    timestamp
                ],
            )
            .map_err(map_integration_conflict)?;
        self.open_commerce_integration_for_project(project_id, &id)
    }

    pub(crate) fn set_open_commerce_integration_enabled(
        &self,
        project_id: &str,
        integration_id: &str,
        enabled: bool,
    ) -> Result<OpenCommerceIntegration> {
        let current = self.open_commerce_integration_for_project(project_id, integration_id)?;
        let status = if enabled {
            if current.last_verified_at.is_some() || current.last_sync_at.is_some() {
                INTEGRATION_STATUS_CONNECTED
            } else {
                INTEGRATION_STATUS_CONFIGURED
            }
        } else {
            INTEGRATION_STATUS_DISABLED
        };
        self.conn()?.execute(
            "UPDATE open_commerce_integrations
                SET status = ?1, updated_at = ?2
              WHERE project_id = ?3 AND id = ?4",
            params![status, now(), project_id.trim(), integration_id.trim()],
        )?;
        self.open_commerce_integration_for_project(project_id, integration_id)
    }

    pub(crate) fn record_open_commerce_sync_receipt(
        &self,
        input: RecordOpenCommerceSyncReceipt<'_>,
    ) -> Result<OpenCommerceSyncReceipt> {
        let integration = self.open_commerce_integration_for_project(
            input.project_id,
            &input.request.integration_id,
        )?;
        if integration.status == INTEGRATION_STATUS_DISABLED {
            bail!("数据接入已停用，不能记录同步回执");
        }
        let receipt_key = normalize_receipt_key(&input.request.receipt_key)?;
        let sync_kind = normalize_sync_kind(&input.request.sync_kind)?;
        let status = normalize_sync_status(&input.request.status)?;
        validate_counts(input.request.records_seen, input.request.records_changed)?;
        let cursor_digest = clean_digest(input.request.cursor_digest.as_deref())?;
        let error_code = clean_error_code(input.request.error_code.as_deref())?;
        let started_at = validate_timestamp(&input.request.started_at, "同步开始时间")?;
        let completed_at = validate_timestamp(&input.request.completed_at, "同步完成时间")?;
        if completed_at < started_at {
            bail!("同步完成时间不能早于开始时间");
        }
        let started_at = started_at.to_rfc3339();
        let completed_at = completed_at.to_rfc3339();
        let fingerprint = receipt_fingerprint(
            &integration.id,
            &receipt_key,
            &sync_kind,
            &status,
            input.request.records_seen,
            input.request.records_changed,
            cursor_digest.as_deref(),
            error_code.as_deref(),
            &started_at,
            &completed_at,
        );

        if let Some(existing) =
            self.find_open_commerce_sync_receipt(&integration.id, &receipt_key)?
        {
            let stored_fingerprint = self.conn()?.query_row(
                "SELECT receipt_fingerprint FROM open_commerce_sync_receipts WHERE id = ?1",
                params![existing.id],
                |row| row.get::<_, String>(0),
            )?;
            if stored_fingerprint != fingerprint {
                bail!("相同同步回执键不能用于不同结果");
            }
            return Ok(existing);
        }

        let id = new_id("sync");
        let created_at = now();
        self.conn()?
            .execute(
                "INSERT INTO open_commerce_sync_receipts (
                    id, project_id, integration_id, receipt_key, receipt_fingerprint,
                    sync_kind, status, records_seen, records_changed, cursor_digest,
                    error_code, recorded_by_user_id, recorded_by_app_id,
                    started_at, completed_at, created_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                    ?12, ?13, ?14, ?15, ?16
                 )",
                params![
                    id,
                    input.project_id.trim(),
                    integration.id,
                    receipt_key,
                    fingerprint,
                    sync_kind,
                    status,
                    input.request.records_seen,
                    input.request.records_changed,
                    cursor_digest,
                    error_code,
                    input.actor_user_id.trim(),
                    input.actor_app_id.trim(),
                    started_at,
                    completed_at,
                    created_at
                ],
            )
            .map_err(map_receipt_conflict)?;

        let integration_status = match status.as_str() {
            "succeeded" => INTEGRATION_STATUS_CONNECTED,
            _ => INTEGRATION_STATUS_DEGRADED,
        };
        let last_verified_at =
            (sync_kind == "health_check" && status == "succeeded").then_some(completed_at.as_str());
        self.conn()?.execute(
            "UPDATE open_commerce_integrations
                SET status = ?1,
                    last_verified_at = COALESCE(?2, last_verified_at),
                    last_sync_at = CASE
                      WHEN ?3 = 'health_check' THEN last_sync_at ELSE ?4
                    END,
                    updated_at = ?4
              WHERE id = ?5",
            params![
                integration_status,
                last_verified_at,
                sync_kind,
                completed_at,
                integration.id
            ],
        )?;
        self.open_commerce_sync_receipt(&id)
    }

    pub(crate) fn open_commerce_integration_for_project(
        &self,
        project_id: &str,
        integration_id: &str,
    ) -> Result<OpenCommerceIntegration> {
        self.conn()?
            .query_row(
                &format!("{INTEGRATION_SELECT} WHERE project_id = ?1 AND id = ?2"),
                params![project_id.trim(), integration_id.trim()],
                integration_from_row,
            )
            .map_err(|error| anyhow!(error).context("当前项目中不存在该数据接入"))
    }

    pub(crate) fn list_project_open_commerce_integrations(
        &self,
        project_id: &str,
    ) -> Result<Vec<OpenCommerceIntegration>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{INTEGRATION_SELECT} WHERE project_id = ?1 ORDER BY updated_at DESC"
        ))?;
        let values = stmt
            .query_map(params![project_id.trim()], integration_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(values)
    }

    pub(crate) fn list_project_open_commerce_sync_receipts(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceSyncReceipt>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{SYNC_RECEIPT_SELECT} WHERE project_id = ?1 ORDER BY created_at DESC LIMIT ?2"
        ))?;
        let values = stmt
            .query_map(
                params![project_id.trim(), limit.clamp(1, 200) as i64],
                sync_receipt_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(values)
    }

    fn open_commerce_sync_receipt(&self, receipt_id: &str) -> Result<OpenCommerceSyncReceipt> {
        self.conn()?
            .query_row(
                &format!("{SYNC_RECEIPT_SELECT} WHERE id = ?1"),
                params![receipt_id.trim()],
                sync_receipt_from_row,
            )
            .map_err(|error| anyhow!(error).context("同步回执不存在"))
    }

    fn find_open_commerce_sync_receipt(
        &self,
        integration_id: &str,
        receipt_key: &str,
    ) -> Result<Option<OpenCommerceSyncReceipt>> {
        self.conn()?
            .query_row(
                &format!(
                    "{SYNC_RECEIPT_SELECT}
                     WHERE integration_id = ?1 AND receipt_key = ?2"
                ),
                params![integration_id.trim(), receipt_key.trim()],
                sync_receipt_from_row,
            )
            .optional()
            .map_err(Into::into)
    }
}

fn integration_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceIntegration> {
    Ok(OpenCommerceIntegration {
        id: row.get(0)?,
        project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        integration_key: row.get(3)?,
        provider_key: row.get(4)?,
        display_name: row.get(5)?,
        connection_mode: row.get(6)?,
        status: row.get(7)?,
        scopes: parse_string_list(row.get(8)?, "接入授权范围 JSON 无效")?,
        data_domains: parse_string_list(row.get(9)?, "接入数据域 JSON 无效")?,
        created_by_user_id: row.get(10)?,
        last_verified_at: row.get(11)?,
        last_sync_at: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn sync_receipt_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceSyncReceipt> {
    Ok(OpenCommerceSyncReceipt {
        id: row.get(0)?,
        project_id: row.get(1)?,
        integration_id: row.get(2)?,
        receipt_key: row.get(3)?,
        sync_kind: row.get(4)?,
        status: row.get(5)?,
        records_seen: row.get(6)?,
        records_changed: row.get(7)?,
        cursor_digest: row.get(8)?,
        error_code: row.get(9)?,
        recorded_by_user_id: row.get(10)?,
        recorded_by_app_id: row.get(11)?,
        started_at: row.get(12)?,
        completed_at: row.get(13)?,
        created_at: row.get(14)?,
    })
}

fn parse_string_list(value: String, label: &str) -> rusqlite::Result<Vec<String>> {
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            value.len(),
            rusqlite::types::Type::Text,
            anyhow!("{label}: {error}").into(),
        )
    })
}

fn validate_counts(records_seen: i64, records_changed: i64) -> Result<()> {
    if records_seen < 0 || records_changed < 0 {
        bail!("同步记录数不能为负数");
    }
    if records_changed > records_seen {
        bail!("变更记录数不能超过扫描记录数");
    }
    Ok(())
}

fn validate_timestamp(value: &str, label: &str) -> Result<chrono::DateTime<chrono::FixedOffset>> {
    let parsed = chrono::DateTime::parse_from_rfc3339(value.trim())
        .with_context(|| format!("{label}必须是 RFC3339 时间"))?;
    Ok(parsed)
}

fn clean_digest(value: Option<&str>) -> Result<Option<String>> {
    clean_optional_identifier(value, "游标摘要", 128)
}

fn clean_error_code(value: Option<&str>) -> Result<Option<String>> {
    clean_optional_identifier(value, "错误代码", 96)
}

fn clean_optional_identifier(
    value: Option<&str>,
    label: &str,
    max_len: usize,
) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.len() > max_len
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ':'))
    {
        bail!("{label}格式无效");
    }
    Ok(Some(value.to_string()))
}

#[allow(clippy::too_many_arguments)]
fn receipt_fingerprint(
    integration_id: &str,
    receipt_key: &str,
    sync_kind: &str,
    status: &str,
    records_seen: i64,
    records_changed: i64,
    cursor_digest: Option<&str>,
    error_code: Option<&str>,
    started_at: &str,
    completed_at: &str,
) -> String {
    let canonical = format!(
        "{integration_id}\n{receipt_key}\n{sync_kind}\n{status}\n{records_seen}\n\
         {records_changed}\n{}\n{}\n{started_at}\n{completed_at}",
        cursor_digest.unwrap_or_default(),
        error_code.unwrap_or_default()
    );
    format!("{:x}", Sha256::digest(canonical.as_bytes()))
}

fn map_integration_conflict(error: rusqlite::Error) -> anyhow::Error {
    if error.to_string().contains("UNIQUE constraint failed") {
        anyhow!("当前商户已存在相同接入键")
    } else {
        anyhow!(error)
    }
}

fn map_receipt_conflict(error: rusqlite::Error) -> anyhow::Error {
    if error.to_string().contains("UNIQUE constraint failed") {
        anyhow!("同步回执键发生并发冲突，请读取已有回执")
    } else {
        anyhow!(error)
    }
}

const INTEGRATION_SELECT: &str =
    "SELECT id, project_id, merchant_id, integration_key, provider_key,
            display_name, connection_mode, status, scopes_json, data_domains_json,
            created_by_user_id, last_verified_at, last_sync_at, created_at, updated_at
       FROM open_commerce_integrations";

const SYNC_RECEIPT_SELECT: &str =
    "SELECT id, project_id, integration_id, receipt_key, sync_kind, status,
            records_seen, records_changed, cursor_digest, error_code,
            recorded_by_user_id, recorded_by_app_id, started_at, completed_at, created_at
       FROM open_commerce_sync_receipts";
