use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row};

use crate::{
    open_commerce_developer_model::{CreateAuthorizationRequest, OpenCommerceAuthorizationRequest},
    open_commerce_model::{normalize_app_id, normalize_capability_key, ACCESS_AUTHORIZED},
};

use super::{new_id, now, open_commerce_app_blocks::ensure_app_not_blocked_on, Store};

impl Store {
    pub(crate) fn create_open_commerce_authorization_request(
        &self,
        requester_user_id: &str,
        request: CreateAuthorizationRequest,
    ) -> Result<OpenCommerceAuthorizationRequest> {
        let merchant = self.open_commerce_merchant(&request.merchant_id)?;
        if !self.open_commerce_directory_is_published(&merchant.id)? {
            bail!("商户节点未发布到开放目录，不能接收外部授权申请");
        }
        let requester_app_id = normalize_app_id(&request.requester_app_id)?;
        let scopes = normalize_scopes(&request.scopes)?;
        for scope in &scopes {
            let capability = self.open_commerce_capability_by_key(&merchant.id, scope)?;
            if capability.access_level != ACCESS_AUTHORIZED {
                bail!("只有 authorized 能力可以申请授权");
            }
        }
        let purpose = normalize_purpose(&request.purpose)?;
        if let Some(existing) =
            self.pending_open_commerce_authorization_request(&merchant.id, &requester_app_id)?
        {
            if existing.scopes == scopes && existing.purpose == purpose {
                return Ok(existing);
            }
            bail!("该 App 已有待处理授权请求，请等待商户决定或调整原请求");
        }
        let id = new_id("authreq");
        let timestamp = now();
        let conn = self.conn()?;
        ensure_app_not_blocked_on(&conn, &merchant.id, &requester_app_id)?;
        conn.execute(
            "INSERT INTO open_commerce_authorization_requests (
               id, merchant_project_id, merchant_id, requester_user_id,
               requester_app_id, scopes_json, purpose, status,
               decided_by_user_id, decision_reason, grant_id,
               created_at, updated_at
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending',
               NULL, NULL, NULL, ?8, ?8
             )",
            params![
                id,
                merchant.project_id,
                merchant.id,
                requester_user_id.trim(),
                requester_app_id,
                serde_json::to_string(&scopes)?,
                purpose,
                timestamp
            ],
        )?;
        drop(conn);
        self.open_commerce_authorization_request(&id)
    }

    pub(crate) fn open_commerce_authorization_request(
        &self,
        request_id: &str,
    ) -> Result<OpenCommerceAuthorizationRequest> {
        self.conn()?
            .query_row(
                &format!("{REQUEST_SELECT} WHERE id = ?1"),
                params![request_id.trim()],
                request_from_row,
            )
            .map_err(|error| anyhow!(error).context("授权请求不存在"))
    }

    pub(crate) fn list_project_open_commerce_authorization_requests(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceAuthorizationRequest>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{REQUEST_SELECT}
             WHERE merchant_project_id = ?1
             ORDER BY CASE status WHEN 'pending' THEN 0 ELSE 1 END, updated_at DESC
             LIMIT ?2"
        ))?;
        let requests = stmt
            .query_map(
                params![project_id.trim(), limit.clamp(1, 200) as i64],
                request_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(requests)
    }

    pub(crate) fn list_requester_project_open_commerce_authorization_requests(
        &self,
        requester_project_id: &str,
        limit: usize,
    ) -> Result<Vec<OpenCommerceAuthorizationRequest>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{REQUEST_SELECT}
             WHERE requester_app_id IN (
               SELECT app_id FROM open_commerce_developer_apps WHERE project_id = ?1
             )
             ORDER BY CASE status WHEN 'pending' THEN 0 ELSE 1 END, updated_at DESC
             LIMIT ?2"
        ))?;
        let requests = stmt
            .query_map(
                params![requester_project_id.trim(), limit.clamp(1, 200) as i64],
                request_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(requests)
    }

    pub(crate) fn list_pending_open_commerce_authorization_requests_for_app(
        &self,
        requester_app_id: &str,
    ) -> Result<Vec<OpenCommerceAuthorizationRequest>> {
        let requester_app_id = normalize_app_id(requester_app_id)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{REQUEST_SELECT}
             WHERE requester_app_id = ?1 AND status = 'pending'
             ORDER BY updated_at DESC"
        ))?;
        let requests = stmt
            .query_map(params![requester_app_id], request_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(requests)
    }

    pub(crate) fn cancel_requester_open_commerce_authorization_request(
        &self,
        requester_project_id: &str,
        request_id: &str,
    ) -> Result<OpenCommerceAuthorizationRequest> {
        let current = self.open_commerce_authorization_request(request_id)?;
        let app = self
            .open_commerce_developer_app_by_app_id(&current.requester_app_id)?
            .ok_or_else(|| anyhow!("授权请求对应的开发者应用不存在"))?;
        if app.project_id != requester_project_id.trim() {
            bail!("授权请求不属于当前开发者项目");
        }
        if current.status != "pending" {
            return Ok(current);
        }
        self.conn()?.execute(
            "UPDATE open_commerce_authorization_requests
                SET status = 'canceled', decision_reason = 'requester_canceled',
                    updated_at = ?1
              WHERE id = ?2 AND status = 'pending'",
            params![now(), request_id.trim()],
        )?;
        self.open_commerce_authorization_request(request_id)
    }

    pub(crate) fn decide_open_commerce_authorization_request(
        &self,
        project_id: &str,
        request_id: &str,
        decided_by_user_id: &str,
        status: &str,
        reason: &str,
        grant_id: Option<&str>,
    ) -> Result<OpenCommerceAuthorizationRequest> {
        if !matches!(status, "approved" | "rejected") {
            bail!("授权决定状态无效");
        }
        let current = self.open_commerce_authorization_request(request_id)?;
        if current.merchant_project_id != project_id.trim() {
            bail!("授权请求不属于当前项目");
        }
        if current.status != "pending" {
            return Ok(current);
        }
        let reason = normalize_decision_reason(reason)?;
        self.conn()?.execute(
            "UPDATE open_commerce_authorization_requests
                SET status = ?1, decided_by_user_id = ?2, decision_reason = ?3,
                    grant_id = ?4, updated_at = ?5
              WHERE id = ?6 AND merchant_project_id = ?7 AND status = 'pending'",
            params![
                status,
                decided_by_user_id.trim(),
                reason,
                grant_id,
                now(),
                request_id.trim(),
                project_id.trim()
            ],
        )?;
        self.open_commerce_authorization_request(request_id)
    }

    pub(crate) fn active_open_commerce_grant_for_app_capability(
        &self,
        merchant_id: &str,
        app_id: &str,
        capability_key: &str,
    ) -> Result<Option<String>> {
        let app_id = normalize_app_id(app_id)?;
        let capability_key = normalize_capability_key(capability_key)?;
        let current = now();
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, scopes_json FROM open_commerce_grants
              WHERE merchant_id = ?1 AND grantee_app_id = ?2
                AND revoked_at IS NULL
                AND (expires_at IS NULL OR expires_at > ?3)
              ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![merchant_id.trim(), app_id, current], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (grant_id, scopes_json) = row?;
            let scopes: Vec<String> = serde_json::from_str(&scopes_json)?;
            if scopes
                .iter()
                .any(|scope| scope == "*" || scope == &capability_key)
            {
                return Ok(Some(grant_id));
            }
        }
        Ok(None)
    }

    pub(crate) fn pending_authorization_for_app_capability(
        &self,
        merchant_id: &str,
        app_id: &str,
        capability_key: &str,
    ) -> Result<Option<String>> {
        let app_id = normalize_app_id(app_id)?;
        let capability_key = normalize_capability_key(capability_key)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, scopes_json FROM open_commerce_authorization_requests
              WHERE merchant_id = ?1 AND requester_app_id = ?2 AND status = 'pending'
              ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![merchant_id.trim(), app_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (request_id, scopes_json) = row?;
            let scopes: Vec<String> = serde_json::from_str(&scopes_json)?;
            if scopes.iter().any(|scope| scope == &capability_key) {
                return Ok(Some(request_id));
            }
        }
        Ok(None)
    }

    fn pending_open_commerce_authorization_request(
        &self,
        merchant_id: &str,
        app_id: &str,
    ) -> Result<Option<OpenCommerceAuthorizationRequest>> {
        self.conn()?
            .query_row(
                &format!(
                    "{REQUEST_SELECT}
                     WHERE merchant_id = ?1 AND requester_app_id = ?2 AND status = 'pending'
                     ORDER BY created_at DESC LIMIT 1"
                ),
                params![merchant_id.trim(), app_id.trim()],
                request_from_row,
            )
            .optional()
            .map_err(Into::into)
    }
}

fn request_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceAuthorizationRequest> {
    let scopes_json: String = row.get(5)?;
    let scopes = serde_json::from_str(&scopes_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            scopes_json.len(),
            rusqlite::types::Type::Text,
            anyhow!("授权请求 scopes JSON 无效: {error}").into(),
        )
    })?;
    Ok(OpenCommerceAuthorizationRequest {
        id: row.get(0)?,
        merchant_project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        requester_user_id: row.get(3)?,
        requester_app_id: row.get(4)?,
        scopes,
        purpose: row.get(6)?,
        status: row.get(7)?,
        decided_by_user_id: row.get(8)?,
        decision_reason: row.get(9)?,
        grant_id: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn normalize_scopes(scopes: &[String]) -> Result<Vec<String>> {
    if scopes.is_empty() || scopes.len() > 32 {
        bail!("授权请求必须包含 1 到 32 个能力 scope");
    }
    let mut values = scopes
        .iter()
        .map(|scope| normalize_capability_key(scope))
        .collect::<Result<Vec<_>>>()?;
    values.sort();
    values.dedup();
    Ok(values)
}

fn normalize_purpose(value: &str) -> Result<String> {
    let value = value.trim();
    if !(3..=200).contains(&value.chars().count()) {
        bail!("授权用途长度必须在 3 到 200 个字符之间");
    }
    Ok(value.to_string())
}

fn normalize_decision_reason(value: &str) -> Result<Option<String>> {
    let value = value.trim();
    if value.chars().count() > 240 {
        bail!("授权决定说明不能超过 240 个字符");
    }
    Ok((!value.is_empty()).then(|| value.to_string()))
}

const REQUEST_SELECT: &str = "SELECT id, merchant_project_id, merchant_id,
           requester_user_id, requester_app_id, scopes_json, purpose, status,
           decided_by_user_id, decision_reason, grant_id, created_at, updated_at
      FROM open_commerce_authorization_requests";
