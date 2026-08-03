//! Production developer credential persistence and fail-closed authentication.

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, types::Type, OptionalExtension, Row, Transaction};

use crate::{
    open_commerce_developer_credential_model::{
        production_credentials_enabled, AuthenticatedDeveloperCredential,
        DeveloperProductionCredential, DeveloperProductionCredentialSecret,
    },
    open_commerce_developer_model::OpenCommerceDeveloperApp,
};

use super::{hash_token, new_id, now, Store};

impl Store {
    pub(crate) fn issue_open_commerce_developer_production_credential(
        &self,
        app: &OpenCommerceDeveloperApp,
        admission_id: &str,
        scopes: &[String],
        issued_by_user_id: &str,
        expires_at: &str,
    ) -> Result<DeveloperProductionCredentialSecret> {
        let live_token = new_live_token();
        let id = new_id("devcred");
        let timestamp = now();
        let scopes_json = serde_json::to_string(scopes)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let eligible: bool = tx.query_row(
            "SELECT EXISTS(
                SELECT 1
                  FROM open_commerce_developer_apps app
                  JOIN open_commerce_developer_app_admissions admission
                    ON admission.app_record_id=app.id
                 WHERE app.id=?1 AND app.project_id=?2 AND app.status='active'
                   AND app.manifest_status='approved' AND app.manifest_revision=?3
                   AND app.domain_verification_status='verified'
                   AND app.domain_verification_revision=?3
                   AND admission.id=?4 AND admission.status='approved'
                   AND admission.manifest_revision=?3
             )",
            params![app.id, app.project_id, app.manifest_revision, admission_id],
            |row| row.get(0),
        )?;
        if !eligible {
            bail!("App 当前资料、域名证明或准入记录不再满足生产凭据签发条件");
        }
        revoke_active_production_credentials(&tx, &app.id, "credential_rotated", &timestamp)?;
        tx.execute(
            "INSERT INTO open_commerce_developer_production_credentials (
                id, app_record_id, project_id, admission_id, manifest_revision,
                scopes_json, status, token_hash, token_hint, issued_by_user_id,
                issued_at, expires_at, last_used_at, revoked_at,
                revocation_reason, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?8, ?9,
                       ?10, ?11, NULL, NULL, NULL, ?10, ?10)",
            params![
                id,
                app.id,
                app.project_id,
                admission_id.trim(),
                app.manifest_revision,
                scopes_json,
                hash_token(&live_token),
                token_hint(&live_token),
                issued_by_user_id.trim(),
                timestamp,
                expires_at.trim(),
            ],
        )?;
        let credential = production_credential_on(&tx, &id)?;
        tx.commit()?;
        Ok(DeveloperProductionCredentialSecret {
            schema: "open_commerce.developer_production_credential_secret.v1",
            credential,
            live_token,
            token_visible_once: true,
            funds_moved: false,
        })
    }

    pub(crate) fn list_open_commerce_developer_production_credentials(
        &self,
        project_id: &str,
        app_record_id: &str,
        limit: usize,
    ) -> Result<Vec<DeveloperProductionCredential>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{PRODUCTION_CREDENTIAL_SELECT}
              WHERE project_id=?1 AND app_record_id=?2
              ORDER BY issued_at DESC LIMIT ?3"
        ))?;
        stmt.query_map(
            params![
                project_id.trim(),
                app_record_id.trim(),
                limit.clamp(1, 100) as i64
            ],
            production_credential_from_row,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
    }

    pub(crate) fn revoke_open_commerce_developer_production_credential(
        &self,
        project_id: &str,
        app_record_id: &str,
        credential_id: &str,
        reason: &str,
    ) -> Result<DeveloperProductionCredential> {
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let current = production_credential_on(&tx, credential_id)?;
        if current.project_id != project_id.trim() || current.app_record_id != app_record_id.trim()
        {
            bail!("生产凭据不属于当前 App");
        }
        if current.status == "active" {
            tx.execute(
                "UPDATE open_commerce_developer_production_credentials
                    SET status='revoked', revoked_at=?1, revocation_reason=?2,
                        updated_at=?1
                  WHERE id=?3 AND status='active'",
                params![timestamp, reason.trim(), credential_id.trim()],
            )?;
        }
        let credential = production_credential_on(&tx, credential_id)?;
        tx.commit()?;
        Ok(credential)
    }

    pub(crate) fn authenticate_open_commerce_developer_credential(
        &self,
        token: &str,
    ) -> Result<AuthenticatedDeveloperCredential> {
        let token = token.trim();
        if token.starts_with("oc_test_") {
            return self
                .authenticate_open_commerce_developer_app(token)
                .map(AuthenticatedDeveloperCredential::sandbox);
        }
        self.authenticate_open_commerce_live_credential(token)
    }

    fn authenticate_open_commerce_live_credential(
        &self,
        token: &str,
    ) -> Result<AuthenticatedDeveloperCredential> {
        if !production_credentials_enabled() {
            bail!("生产开发者凭据入口当前未启用");
        }
        if !token.starts_with("oc_live_") || token.len() < 40 {
            bail!("生产开发者凭据无效");
        }
        let timestamp = now();
        let conn = self.conn()?;
        let credential = conn
            .query_row(
                &format!("{PRODUCTION_CREDENTIAL_SELECT} WHERE token_hash=?1"),
                params![hash_token(token)],
                production_credential_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("生产开发者凭据无效或已撤销"))?;
        if credential.status != "active" {
            bail!("生产开发者凭据无效或已撤销");
        }
        let expires_at = DateTime::parse_from_rfc3339(&credential.expires_at)
            .context("生产开发者凭据到期时间无效")?;
        if expires_at <= Utc::now() {
            bail!("生产开发者凭据已到期");
        }
        let eligible: bool = conn.query_row(
            "SELECT EXISTS(
                SELECT 1
                  FROM open_commerce_developer_apps app
                  JOIN open_commerce_developer_app_admissions admission
                    ON admission.app_record_id=app.id
                 WHERE app.id=?1 AND app.project_id=?2 AND app.status='active'
                   AND app.manifest_status='approved'
                   AND app.manifest_revision=?3
                   AND app.domain_verification_status='verified'
                   AND app.domain_verification_revision=?3
                   AND admission.id=?4 AND admission.status='approved'
                   AND admission.manifest_revision=?3
             )",
            params![
                credential.app_record_id,
                credential.project_id,
                credential.manifest_revision,
                credential.admission_id,
            ],
            |row| row.get(0),
        )?;
        if !eligible {
            bail!("生产开发者凭据所绑定的资料、域名证明或准入记录已失效");
        }
        conn.execute(
            "UPDATE open_commerce_developer_production_credentials
                SET last_used_at=?1, updated_at=?1
              WHERE id=?2 AND status='active'",
            params![timestamp, credential.id],
        )?;
        drop(conn);
        let app = self.open_commerce_developer_app_by_record_id(&credential.app_record_id)?;
        Ok(AuthenticatedDeveloperCredential::production(
            app,
            credential.id,
            credential.scopes,
        ))
    }
}

pub(super) fn revoke_active_production_credentials(
    tx: &Transaction<'_>,
    app_record_id: &str,
    reason: &str,
    timestamp: &str,
) -> Result<usize> {
    tx.execute(
        "UPDATE open_commerce_developer_production_credentials
            SET status='revoked', revoked_at=?1, revocation_reason=?2,
                updated_at=?1
          WHERE app_record_id=?3 AND status='active'",
        params![timestamp, reason.trim(), app_record_id.trim()],
    )
    .map_err(Into::into)
}

fn production_credential_on(
    tx: &Transaction<'_>,
    credential_id: &str,
) -> Result<DeveloperProductionCredential> {
    tx.query_row(
        &format!("{PRODUCTION_CREDENTIAL_SELECT} WHERE id=?1"),
        params![credential_id.trim()],
        production_credential_from_row,
    )
    .map_err(|error| anyhow!(error).context("生产凭据不存在"))
}

fn production_credential_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<DeveloperProductionCredential> {
    let scopes_json: String = row.get(5)?;
    let scopes = serde_json::from_str(&scopes_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(5, Type::Text, Box::new(error))
    })?;
    Ok(DeveloperProductionCredential {
        schema: "open_commerce.developer_production_credential.v1",
        id: row.get(0)?,
        app_record_id: row.get(1)?,
        project_id: row.get(2)?,
        admission_id: row.get(3)?,
        manifest_revision: row.get(4)?,
        environment: "production",
        scopes,
        status: row.get(6)?,
        token_hint: row.get(7)?,
        issued_by_user_id: row.get(8)?,
        issued_at: row.get(9)?,
        expires_at: row.get(10)?,
        last_used_at: row.get(11)?,
        revoked_at: row.get(12)?,
        revocation_reason: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn new_live_token() -> String {
    format!(
        "oc_live_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn token_hint(value: &str) -> String {
    format!("...{}", &value[value.len().saturating_sub(6)..])
}

const PRODUCTION_CREDENTIAL_SELECT: &str = "SELECT id, app_record_id, project_id,
            admission_id, manifest_revision, scopes_json, status, token_hint,
            issued_by_user_id, issued_at, expires_at, last_used_at, revoked_at,
            revocation_reason, created_at, updated_at
       FROM open_commerce_developer_production_credentials";
