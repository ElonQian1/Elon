use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, OptionalExtension, Row};
use sha2::{Digest, Sha256};

use crate::open_commerce_adapter_model::{
    OpenCommerceAdapterCredential, OpenCommerceAdapterCredentialIssue,
    ADAPTER_CREDENTIAL_ISSUE_SCHEMA, ADAPTER_CREDENTIAL_SCHEMA, ADAPTER_HANDOFF_SCOPE,
};
use crate::open_commerce_integration_model::INTEGRATION_STATUS_DISABLED;

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn rotate_open_commerce_adapter_credential(
        &self,
        project_id: &str,
        integration_id: &str,
        actor_user_id: &str,
        expires_at: &str,
    ) -> Result<OpenCommerceAdapterCredentialIssue> {
        let integration = self.open_commerce_integration_for_project(project_id, integration_id)?;
        if integration.status == INTEGRATION_STATUS_DISABLED {
            bail!("数据接入已停用，不能签发适配器凭据");
        }
        let token = new_adapter_token();
        let timestamp = now();
        let existing_id = self
            .conn()?
            .query_row(
                "SELECT id FROM open_commerce_adapter_credentials WHERE integration_id=?1",
                params![integration.id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(id) = existing_id {
            self.conn()?.execute(
                "UPDATE open_commerce_adapter_credentials
                    SET status='active', scopes_json=?1, token_hash=?2, token_hint=?3,
                        credential_version=credential_version+1, last_used_at=NULL,
                        expires_at=?4, updated_at=?5
                  WHERE id=?6",
                params![
                    serde_json::to_string(&vec![ADAPTER_HANDOFF_SCOPE])?,
                    token_hash(&token),
                    token_hint(&token),
                    expires_at,
                    timestamp,
                    id
                ],
            )?;
        } else {
            self.conn()?.execute(
                "INSERT INTO open_commerce_adapter_credentials (
                    id, project_id, merchant_id, integration_id, status, scopes_json,
                    token_hash, token_hint, credential_version, created_by_user_id,
                    last_used_at, created_at, updated_at, expires_at
                 ) VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6, ?7, 1, ?8, NULL, ?9, ?9, ?10)",
                params![
                    new_id("adaptercred"),
                    project_id.trim(),
                    integration.merchant_id,
                    integration.id,
                    serde_json::to_string(&vec![ADAPTER_HANDOFF_SCOPE])?,
                    token_hash(&token),
                    token_hint(&token),
                    actor_user_id.trim(),
                    timestamp,
                    expires_at
                ],
            )?;
        }
        let credential =
            self.open_commerce_adapter_credential_for_integration(project_id, integration_id)?;
        Ok(OpenCommerceAdapterCredentialIssue {
            schema: ADAPTER_CREDENTIAL_ISSUE_SCHEMA,
            credential,
            adapter_token: token,
            token_visible_once: true,
        })
    }

    pub(crate) fn revoke_open_commerce_adapter_credential(
        &self,
        project_id: &str,
        credential_id: &str,
    ) -> Result<OpenCommerceAdapterCredential> {
        let credential =
            self.open_commerce_adapter_credential_for_project(project_id, credential_id)?;
        if credential.status == "active" {
            let replacement = new_adapter_token();
            self.conn()?.execute(
                "UPDATE open_commerce_adapter_credentials
                    SET status='revoked', token_hash=?1, token_hint='revoked',
                        credential_version=credential_version+1, updated_at=?2
                  WHERE project_id=?3 AND id=?4 AND status='active'",
                params![
                    token_hash(&replacement),
                    now(),
                    project_id.trim(),
                    credential_id.trim()
                ],
            )?;
        }
        self.open_commerce_adapter_credential_for_project(project_id, credential_id)
    }

    pub(crate) fn list_project_open_commerce_adapter_credentials(
        &self,
        project_id: &str,
    ) -> Result<Vec<OpenCommerceAdapterCredential>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(&format!(
            "{ADAPTER_CREDENTIAL_SELECT} WHERE project_id=?1 ORDER BY updated_at DESC"
        ))?;
        let credentials = statement
            .query_map(params![project_id.trim()], adapter_credential_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(credentials)
    }

    pub(crate) fn authenticate_open_commerce_adapter_credential(
        &self,
        token: &str,
    ) -> Result<OpenCommerceAdapterCredential> {
        let token = token.trim();
        if !token.starts_with("oc_adapter_") || token.len() < 48 {
            bail!("适配器凭据无效");
        }
        self.conn()?
            .query_row(
                &format!(
                    "SELECT c.id, c.project_id, c.merchant_id, c.integration_id,
                            c.status, c.scopes_json, c.token_hint, c.credential_version,
                            c.created_by_user_id, c.last_used_at, c.created_at, c.updated_at,
                            c.expires_at,
                            (julianday(c.expires_at) <= julianday(?2)) AS is_expired
                       FROM open_commerce_adapter_credentials c
                       JOIN open_commerce_integrations i ON i.id=c.integration_id
                      WHERE c.token_hash=?1 AND c.status='active' AND i.status<>'disabled'
                        AND julianday(c.expires_at) > julianday(?2)"
                ),
                params![token_hash(token), now()],
                adapter_credential_from_row,
            )
            .map_err(|error| anyhow!(error).context("适配器凭据无效、已撤销、已到期或接入已停用"))
    }

    pub(crate) fn touch_open_commerce_adapter_credential(&self, credential_id: &str) -> Result<()> {
        self.conn()?.execute(
            "UPDATE open_commerce_adapter_credentials
                SET last_used_at=?1, updated_at=?1
              WHERE id=?2 AND status='active'",
            params![now(), credential_id.trim()],
        )?;
        Ok(())
    }

    pub(crate) fn open_commerce_adapter_credential_for_integration(
        &self,
        project_id: &str,
        integration_id: &str,
    ) -> Result<OpenCommerceAdapterCredential> {
        self.conn()?
            .query_row(
                &format!("{ADAPTER_CREDENTIAL_SELECT} WHERE project_id=?1 AND integration_id=?2"),
                params![project_id.trim(), integration_id.trim()],
                adapter_credential_from_row,
            )
            .map_err(|error| anyhow!(error).context("当前数据接入尚未签发适配器凭据"))
    }

    pub(crate) fn open_commerce_adapter_credential_for_project(
        &self,
        project_id: &str,
        credential_id: &str,
    ) -> Result<OpenCommerceAdapterCredential> {
        self.conn()?
            .query_row(
                &format!("{ADAPTER_CREDENTIAL_SELECT} WHERE project_id=?1 AND id=?2"),
                params![project_id.trim(), credential_id.trim()],
                adapter_credential_from_row,
            )
            .map_err(|error| anyhow!(error).context("适配器凭据不存在"))
    }
}

fn adapter_credential_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceAdapterCredential> {
    let scopes_json: String = row.get(5)?;
    let scopes = serde_json::from_str(&scopes_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            scopes_json.len(),
            rusqlite::types::Type::Text,
            error.into(),
        )
    })?;
    Ok(OpenCommerceAdapterCredential {
        schema: ADAPTER_CREDENTIAL_SCHEMA,
        id: row.get(0)?,
        project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        integration_id: row.get(3)?,
        status: row.get(4)?,
        scopes,
        token_hint: row.get(6)?,
        credential_version: row.get(7)?,
        created_by_user_id: row.get(8)?,
        last_used_at: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
        expires_at: row.get(12)?,
        is_expired: row.get(13)?,
    })
}

fn new_adapter_token() -> String {
    format!(
        "oc_adapter_{}{}",
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

const ADAPTER_CREDENTIAL_SELECT: &str =
    "SELECT id, project_id, merchant_id, integration_id, status, scopes_json,
            token_hint, credential_version, created_by_user_id, last_used_at,
            created_at, updated_at, expires_at,
            (julianday(expires_at) <= julianday('now')) AS is_expired
       FROM open_commerce_adapter_credentials";
