use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Row};

use crate::open_commerce_model::{
    normalize_app_id, normalize_capability_key, CreateGrantRequest, OpenCommerceGrant,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn create_open_commerce_grant(
        &self,
        project_id: &str,
        grantor_user_id: &str,
        request: CreateGrantRequest,
    ) -> Result<OpenCommerceGrant> {
        self.open_commerce_merchant_for_project(project_id, &request.merchant_id)?;
        let grantee_app_id = normalize_app_id(&request.grantee_app_id)?;
        let scopes = normalize_scopes(&request.scopes)?;
        let purpose = validate_purpose(&request.purpose)?;
        let expires_at = validate_expiration(request.expires_at.as_deref())?;
        let id = new_id("grant");
        let timestamp = now();
        self.conn()?.execute(
            "INSERT INTO open_commerce_grants (
                id, project_id, merchant_id, grantor_user_id, grantee_app_id,
                scopes_json, purpose, expires_at, revoked_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?9)",
            params![
                id,
                project_id.trim(),
                request.merchant_id.trim(),
                grantor_user_id.trim(),
                grantee_app_id,
                serde_json::to_string(&scopes)?,
                purpose,
                expires_at,
                timestamp
            ],
        )?;
        self.open_commerce_grant(&id)
    }

    pub(crate) fn revoke_open_commerce_grant(
        &self,
        project_id: &str,
        grant_id: &str,
    ) -> Result<OpenCommerceGrant> {
        let timestamp = now();
        let updated = self.conn()?.execute(
            "UPDATE open_commerce_grants
                SET revoked_at = COALESCE(revoked_at, ?1), updated_at = ?1
              WHERE id = ?2 AND project_id = ?3",
            params![timestamp, grant_id.trim(), project_id.trim()],
        )?;
        if updated == 0 {
            bail!("授权不存在");
        }
        self.open_commerce_grant(grant_id)
    }

    pub(crate) fn open_commerce_grant(&self, grant_id: &str) -> Result<OpenCommerceGrant> {
        self.conn()?
            .query_row(
                &format!("{GRANT_SELECT} WHERE id = ?1"),
                params![grant_id.trim()],
                grant_from_row,
            )
            .map_err(|error| anyhow!(error).context("授权不存在"))
    }

    pub(crate) fn active_open_commerce_grant(
        &self,
        grant_id: &str,
        merchant_id: &str,
        grantee_app_id: &str,
        capability_key: &str,
    ) -> Result<OpenCommerceGrant> {
        let grantee_app_id = normalize_app_id(grantee_app_id)?;
        let capability_key = normalize_capability_key(capability_key)?;
        let grant = self
            .conn()?
            .query_row(
                &format!(
                    "{GRANT_SELECT}
                     WHERE id = ?1 AND merchant_id = ?2 AND grantee_app_id = ?3
                       AND revoked_at IS NULL
                       AND (expires_at IS NULL OR expires_at > ?4)"
                ),
                params![grant_id.trim(), merchant_id.trim(), grantee_app_id, now()],
                grant_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("授权不存在、已撤销、已过期或不属于当前调用方"))?;
        if !grant
            .scopes
            .iter()
            .any(|scope| scope == "*" || scope == &capability_key)
        {
            bail!("授权 scope 不包含能力 {capability_key}");
        }
        Ok(grant)
    }

    pub(crate) fn list_project_open_commerce_grants(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceGrant>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{GRANT_SELECT}
             WHERE project_id = ?1
             ORDER BY created_at DESC LIMIT ?2"
        ))?;
        Ok(stmt
            .query_map(
                params![project_id.trim(), limit.clamp(1, 200) as i64],
                grant_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn normalize_scopes(scopes: &[String]) -> Result<Vec<String>> {
    if scopes.is_empty() {
        bail!("授权至少需要一个 scope");
    }
    if scopes.len() > 32 {
        bail!("单个授权最多包含 32 个 scope");
    }
    let mut normalized = scopes
        .iter()
        .map(|scope| {
            if scope.trim() == "*" {
                Ok("*".to_string())
            } else {
                normalize_capability_key(scope)
            }
        })
        .collect::<Result<Vec<_>>>()?;
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn validate_purpose(value: &str) -> Result<String> {
    let value = value.trim();
    let length = value.chars().count();
    if !(3..=200).contains(&length) {
        bail!("授权用途长度必须在 3 到 200 个字符之间");
    }
    Ok(value.to_string())
}

fn validate_expiration(value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let parsed = DateTime::parse_from_rfc3339(value).context("expires_at 必须是 RFC 3339 时间")?;
    if parsed.with_timezone(&Utc) <= Utc::now() {
        bail!("授权过期时间必须晚于当前时间");
    }
    Ok(Some(parsed.with_timezone(&Utc).to_rfc3339()))
}

fn grant_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceGrant> {
    let scopes_json: String = row.get(5)?;
    let scopes = serde_json::from_str(&scopes_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            scopes_json.len(),
            rusqlite::types::Type::Text,
            anyhow!("授权 scopes JSON 无效: {error}").into(),
        )
    })?;
    Ok(OpenCommerceGrant {
        id: row.get(0)?,
        project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        grantor_user_id: row.get(3)?,
        grantee_app_id: row.get(4)?,
        scopes,
        purpose: row.get(6)?,
        expires_at: row.get(7)?,
        revoked_at: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

const GRANT_SELECT: &str = "SELECT id, project_id, merchant_id, grantor_user_id, grantee_app_id,
            scopes_json, purpose, expires_at, revoked_at, created_at, updated_at
       FROM open_commerce_grants";
