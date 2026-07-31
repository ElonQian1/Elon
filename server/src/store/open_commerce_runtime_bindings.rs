use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, OptionalExtension, Row};

use crate::open_commerce_runtime_model::{
    normalize_credential_ref, normalize_manifest_sha256, normalize_timeout_ms,
    OpenCommerceRuntimeBinding, UpsertRuntimeBindingRequest, RUNTIME_STATUS_ACTIVE,
    RUNTIME_STATUS_CONFIGURED, RUNTIME_STATUS_DEGRADED,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn upsert_open_commerce_runtime_binding(
        &self,
        project_id: &str,
        merchant_id: &str,
        actor_user_id: &str,
        request: UpsertRuntimeBindingRequest,
    ) -> Result<OpenCommerceRuntimeBinding> {
        self.open_commerce_merchant_for_project(project_id, merchant_id)?;
        let endpoint = crate::open_commerce_runtime_security::validate_endpoint_base_url(
            &request.endpoint_base_url,
        )?;
        let credential_ref = normalize_credential_ref(&request.credential_ref)?;
        let manifest_sha256 = normalize_manifest_sha256(request.manifest_sha256.as_deref())?;
        let timeout_ms = normalize_timeout_ms(request.timeout_ms)?;
        let current = self.open_commerce_runtime_binding_optional(merchant_id)?;
        let id = current
            .as_ref()
            .map(|binding| binding.id.clone())
            .unwrap_or_else(|| new_id("runtime"));
        let created_at = current
            .as_ref()
            .map(|binding| binding.created_at.clone())
            .unwrap_or_else(now);
        let timestamp = now();
        self.conn()?.execute(
            "INSERT INTO open_commerce_runtime_bindings (
                id, project_id, merchant_id, endpoint_base_url, credential_ref,
                manifest_sha256, timeout_ms, status, last_verified_at, last_error_code,
                created_by_user_id, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'configured', NULL, NULL, ?8, ?9, ?10)
             ON CONFLICT(merchant_id) DO UPDATE SET
                endpoint_base_url = excluded.endpoint_base_url,
                credential_ref = excluded.credential_ref,
                manifest_sha256 = excluded.manifest_sha256,
                timeout_ms = excluded.timeout_ms,
                status = 'configured',
                last_verified_at = NULL,
                last_error_code = NULL,
                updated_at = excluded.updated_at",
            params![
                id,
                project_id.trim(),
                merchant_id.trim(),
                endpoint,
                credential_ref,
                manifest_sha256,
                timeout_ms,
                actor_user_id.trim(),
                created_at,
                timestamp,
            ],
        )?;
        self.open_commerce_runtime_binding(merchant_id)
    }

    pub(crate) fn open_commerce_runtime_binding(
        &self,
        merchant_id: &str,
    ) -> Result<OpenCommerceRuntimeBinding> {
        self.open_commerce_runtime_binding_optional(merchant_id)?
            .ok_or_else(|| anyhow!("商户尚未配置运行绑定"))
    }

    pub(crate) fn active_open_commerce_runtime_binding(
        &self,
        merchant_id: &str,
    ) -> Result<OpenCommerceRuntimeBinding> {
        let binding = self.open_commerce_runtime_binding(merchant_id)?;
        if binding.status != RUNTIME_STATUS_ACTIVE {
            bail!("商户运行绑定尚未通过验证");
        }
        Ok(binding)
    }

    pub(crate) fn list_project_open_commerce_runtime_bindings(
        &self,
        project_id: &str,
    ) -> Result<Vec<OpenCommerceRuntimeBinding>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{RUNTIME_SELECT} WHERE project_id = ?1 ORDER BY updated_at DESC"
        ))?;
        let rows = stmt
            .query_map(params![project_id.trim()], runtime_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub(crate) fn mark_open_commerce_runtime_verified(
        &self,
        merchant_id: &str,
        manifest_sha256: Option<&str>,
    ) -> Result<OpenCommerceRuntimeBinding> {
        let updated = self.conn()?.execute(
            "UPDATE open_commerce_runtime_bindings
                SET status = ?1, manifest_sha256 = COALESCE(?2, manifest_sha256),
                    last_verified_at = ?3, last_error_code = NULL, updated_at = ?3
              WHERE merchant_id = ?4",
            params![
                RUNTIME_STATUS_ACTIVE,
                manifest_sha256,
                now(),
                merchant_id.trim()
            ],
        )?;
        if updated == 0 {
            bail!("商户尚未配置运行绑定");
        }
        self.open_commerce_runtime_binding(merchant_id)
    }

    pub(crate) fn mark_open_commerce_runtime_degraded(
        &self,
        merchant_id: &str,
        error_code: &str,
    ) -> Result<OpenCommerceRuntimeBinding> {
        self.conn()?.execute(
            "UPDATE open_commerce_runtime_bindings
                SET status = ?1, last_error_code = ?2, updated_at = ?3
              WHERE merchant_id = ?4 AND status != 'disabled'",
            params![
                RUNTIME_STATUS_DEGRADED,
                error_code.trim(),
                now(),
                merchant_id.trim()
            ],
        )?;
        self.open_commerce_runtime_binding(merchant_id)
    }

    fn open_commerce_runtime_binding_optional(
        &self,
        merchant_id: &str,
    ) -> Result<Option<OpenCommerceRuntimeBinding>> {
        self.conn()?
            .query_row(
                &format!("{RUNTIME_SELECT} WHERE merchant_id = ?1"),
                params![merchant_id.trim()],
                runtime_from_row,
            )
            .optional()
            .map_err(|error| anyhow!(error).context("读取商户运行绑定失败"))
    }
}

fn runtime_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceRuntimeBinding> {
    Ok(OpenCommerceRuntimeBinding {
        id: row.get(0)?,
        project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        endpoint_base_url: row.get(3)?,
        credential_ref: row.get(4)?,
        manifest_sha256: row.get(5)?,
        timeout_ms: row.get(6)?,
        status: row.get(7)?,
        last_verified_at: row.get(8)?,
        last_error_code: row.get(9)?,
        created_by_user_id: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

const RUNTIME_SELECT: &str =
    "SELECT id, project_id, merchant_id, endpoint_base_url, credential_ref,
            manifest_sha256, timeout_ms, status, last_verified_at, last_error_code,
            created_by_user_id, created_at, updated_at
       FROM open_commerce_runtime_bindings";
