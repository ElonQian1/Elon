use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row};
use sha2::{Digest, Sha256};

use crate::{
    open_commerce_developer_model::{
        CreateDeveloperAppRequest, OpenCommerceDeveloperApp, OpenCommerceDeveloperAppCredential,
    },
    open_commerce_model::{normalize_app_id, validate_display_name},
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn create_open_commerce_developer_app(
        &self,
        project_id: &str,
        owner_user_id: &str,
        request: CreateDeveloperAppRequest,
    ) -> Result<OpenCommerceDeveloperAppCredential> {
        let id = new_id("devapp");
        let app_id = normalize_app_id(&request.app_id)?;
        if matches!(app_id.as_str(), "pc-web" | "mcp-client") {
            bail!("pc-web 和 mcp-client 是系统保留 App ID");
        }
        let display_name = validate_display_name(&request.display_name, "开发者应用名称")?;
        let test_token = new_test_token();
        let token_hash = token_hash(&test_token);
        let token_hint = token_hint(&test_token);
        let timestamp = now();
        self.conn()?
            .execute(
                "INSERT INTO open_commerce_developer_apps (
                   id, project_id, owner_user_id, app_id, display_name,
                   environment, status, test_token_hash, token_hint,
                   created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'sandbox', 'active', ?6, ?7, ?8, ?8)",
                params![
                    id,
                    project_id.trim(),
                    owner_user_id.trim(),
                    app_id,
                    display_name,
                    token_hash,
                    token_hint,
                    timestamp
                ],
            )
            .map_err(map_app_conflict)?;
        let app = self.open_commerce_developer_app_for_project(project_id, &id)?;
        Ok(OpenCommerceDeveloperAppCredential {
            schema: "open_commerce.developer_credential.v1",
            app,
            test_token,
            token_visible_once: true,
        })
    }

    pub(crate) fn rotate_open_commerce_developer_app_token(
        &self,
        project_id: &str,
        app_record_id: &str,
    ) -> Result<OpenCommerceDeveloperAppCredential> {
        let current = self.open_commerce_developer_app_for_project(project_id, app_record_id)?;
        if current.status != "active" {
            bail!("开发者应用已停用，请先重新启用");
        }
        let test_token = new_test_token();
        let timestamp = now();
        self.conn()?.execute(
            "UPDATE open_commerce_developer_apps
                SET test_token_hash = ?1, token_hint = ?2, updated_at = ?3
              WHERE project_id = ?4 AND id = ?5",
            params![
                token_hash(&test_token),
                token_hint(&test_token),
                timestamp,
                project_id.trim(),
                app_record_id.trim()
            ],
        )?;
        let app = self.open_commerce_developer_app_for_project(project_id, app_record_id)?;
        Ok(OpenCommerceDeveloperAppCredential {
            schema: "open_commerce.developer_credential.v1",
            app,
            test_token,
            token_visible_once: true,
        })
    }

    pub(crate) fn disable_open_commerce_developer_app(
        &self,
        project_id: &str,
        app_record_id: &str,
    ) -> Result<(OpenCommerceDeveloperApp, usize)> {
        let current = self.open_commerce_developer_app_for_project(project_id, app_record_id)?;
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        if current.status == "active" {
            let revoked_token = new_test_token();
            tx.execute(
                "UPDATE open_commerce_developer_apps
                    SET status = 'disabled', test_token_hash = ?1,
                        token_hint = 'disabled', updated_at = ?2
                  WHERE project_id = ?3 AND id = ?4 AND status = 'active'",
                params![
                    token_hash(&revoked_token),
                    timestamp,
                    project_id.trim(),
                    app_record_id.trim()
                ],
            )?;
        }
        let canceled = tx.execute(
            "UPDATE open_commerce_authorization_requests
                SET status = 'canceled', decision_reason = 'developer_app_disabled',
                    updated_at = ?1
              WHERE requester_app_id = ?2 AND status = 'pending'",
            params![timestamp, current.app_id],
        )?;
        tx.execute(
            "UPDATE open_commerce_developer_webhook_subscriptions
                SET status='disabled', last_error_code='developer_app_disabled',
                    updated_at=?1, disabled_at=?1
              WHERE app_record_id=?2 AND status='active'",
            params![timestamp, current.id],
        )?;
        tx.execute(
            "UPDATE open_commerce_developer_webhook_deliveries
                SET status='dead', error_code='developer_app_disabled',
                    lease_owner=NULL, lease_expires_at=NULL
              WHERE subscription_id IN (
                SELECT id FROM open_commerce_developer_webhook_subscriptions
                 WHERE app_record_id=?1
              ) AND status IN ('pending', 'retry', 'delivering')",
            params![current.id],
        )?;
        tx.commit()?;
        drop(conn);
        Ok((
            self.open_commerce_developer_app_for_project(project_id, app_record_id)?,
            canceled,
        ))
    }

    pub(crate) fn reactivate_open_commerce_developer_app(
        &self,
        project_id: &str,
        app_record_id: &str,
    ) -> Result<OpenCommerceDeveloperAppCredential> {
        let current = self.open_commerce_developer_app_for_project(project_id, app_record_id)?;
        if current.status == "active" {
            bail!("开发者应用当前已启用");
        }
        let test_token = new_test_token();
        let timestamp = now();
        self.conn()?.execute(
            "UPDATE open_commerce_developer_apps
                SET status = 'active', test_token_hash = ?1, token_hint = ?2,
                    updated_at = ?3
              WHERE project_id = ?4 AND id = ?5 AND status = 'disabled'",
            params![
                token_hash(&test_token),
                token_hint(&test_token),
                timestamp,
                project_id.trim(),
                app_record_id.trim()
            ],
        )?;
        let app = self.open_commerce_developer_app_for_project(project_id, app_record_id)?;
        Ok(OpenCommerceDeveloperAppCredential {
            schema: "open_commerce.developer_credential.v1",
            app,
            test_token,
            token_visible_once: true,
        })
    }

    pub(crate) fn list_project_open_commerce_developer_apps(
        &self,
        project_id: &str,
    ) -> Result<Vec<OpenCommerceDeveloperApp>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{APP_SELECT} WHERE project_id = ?1 ORDER BY updated_at DESC"
        ))?;
        let apps = stmt
            .query_map(params![project_id.trim()], app_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(anyhow::Error::from)?;
        Ok(apps)
    }

    pub(crate) fn open_commerce_developer_app_for_project(
        &self,
        project_id: &str,
        app_record_id: &str,
    ) -> Result<OpenCommerceDeveloperApp> {
        self.conn()?
            .query_row(
                &format!("{APP_SELECT} WHERE project_id = ?1 AND id = ?2"),
                params![project_id.trim(), app_record_id.trim()],
                app_from_row,
            )
            .map_err(|error| anyhow!(error).context("开发者应用不存在"))
    }

    pub(crate) fn open_commerce_developer_app_by_app_id(
        &self,
        app_id: &str,
    ) -> Result<Option<OpenCommerceDeveloperApp>> {
        let app_id = normalize_app_id(app_id)?;
        self.conn()?
            .query_row(
                &format!("{APP_SELECT} WHERE app_id = ?1"),
                params![app_id],
                app_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn authenticate_open_commerce_developer_app(
        &self,
        test_token: &str,
    ) -> Result<OpenCommerceDeveloperApp> {
        let test_token = test_token.trim();
        if !test_token.starts_with("oc_test_") || test_token.len() < 40 {
            bail!("开发者测试凭据无效");
        }
        self.conn()?
            .query_row(
                &format!("{APP_SELECT} WHERE test_token_hash = ?1 AND status = 'active'"),
                params![token_hash(test_token)],
                app_from_row,
            )
            .map_err(|error| anyhow!(error).context("开发者测试凭据无效或已停用"))
    }

    pub(crate) fn ensure_open_commerce_developer_app_owned_by_user(
        &self,
        app_id: &str,
        owner_user_id: &str,
    ) -> Result<OpenCommerceDeveloperApp> {
        let app = self
            .open_commerce_developer_app_by_app_id(app_id)?
            .ok_or_else(|| anyhow!("开发者应用不存在"))?;
        if app.owner_user_id != owner_user_id.trim() {
            bail!("当前用户不能代表该开发者应用发起请求");
        }
        if app.status != "active" {
            bail!("开发者应用已停用");
        }
        Ok(app)
    }
}

fn app_from_row(row: &Row<'_>) -> rusqlite::Result<OpenCommerceDeveloperApp> {
    Ok(OpenCommerceDeveloperApp {
        id: row.get(0)?,
        project_id: row.get(1)?,
        owner_user_id: row.get(2)?,
        app_id: row.get(3)?,
        display_name: row.get(4)?,
        environment: row.get(5)?,
        status: row.get(6)?,
        token_hint: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn new_test_token() -> String {
    format!(
        "oc_test_{}{}",
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

fn map_app_conflict(error: rusqlite::Error) -> anyhow::Error {
    if error.to_string().contains("UNIQUE") {
        anyhow!("App ID 已被使用")
    } else {
        error.into()
    }
}

const APP_SELECT: &str = "SELECT id, project_id, owner_user_id, app_id, display_name,
           environment, status, token_hint, created_at, updated_at
      FROM open_commerce_developer_apps";
