//! 用户、会话、登录态相关的 `Store` 方法。
//!
//! 从巨型 `store.rs` 中抽出，专注账号注册、登录、token 校验、管理员列表。

use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use super::{
    account_columns, clean_optional, hash_password, hash_token, new_id, normalize_account, now,
    validate_password, verify_password, AdminProjectDetail, AdminSessionEntry, AdminUserSummary,
    PublicUser, Store,
};

impl Store {
    pub fn create_user(
        &self,
        account: &str,
        password: &str,
        nickname: Option<&str>,
        role: Option<&str>,
    ) -> Result<PublicUser> {
        let account = normalize_account(account)?;
        validate_password(password)?;
        let now = now();
        let id = new_id("usr");
        let password_hash = hash_password(password);
        let role = role
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("user");
        if !matches!(role, "user" | "admin") {
            return Err(anyhow!("用户角色只能是 user 或 admin"));
        }

        let (phone, email) = account_columns(&account);
        self.conn()?.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?7)",
            params![
                id,
                phone,
                email,
                password_hash,
                clean_optional(nickname),
                role,
                now
            ],
        )?;

        let user = PublicUser {
            id,
            account,
            nickname: clean_optional(nickname).map(ToOwned::to_owned),
            role: role.to_string(),
            status: "active".into(),
            avatar_data_url: None,
        };
        Ok(user)
    }

    pub fn authenticate_password(&self, account: &str, password: &str) -> Result<PublicUser> {
        let account = normalize_account(account)?;
        let row = self
            .conn()?
            .query_row(
                "SELECT id, phone, email, password_hash, nickname, role, status
                 FROM users
                 WHERE (phone = ?1 OR email = ?1 OR id = ?1) AND status = 'active'",
                params![account],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("账号不存在或已停用"))?;

        if !verify_password(password, &row.3) {
            return Err(anyhow!("密码错误"));
        }

        let user = PublicUser {
            id: row.0,
            account: row.2.or(row.1).unwrap_or(account),
            nickname: row.4,
            role: row.5,
            status: row.6,
            avatar_data_url: None,
        };
        Ok(user)
    }

    pub fn create_session(
        &self,
        user_id: &str,
        device_name: Option<&str>,
        apk_version: Option<&str>,
    ) -> Result<(String, String)> {
        let token = format!("tok_{}", Uuid::new_v4().simple());
        let token_hash = hash_token(&token);
        let session_id = new_id("ses");
        let created_at = now();
        let expires_at = (Utc::now() + Duration::days(30)).to_rfc3339();

        self.conn()?.execute(
            "INSERT INTO sessions (id, user_id, token_hash, device_name, apk_version, expires_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                session_id,
                user_id,
                token_hash,
                clean_optional(device_name),
                clean_optional(apk_version),
                expires_at,
                created_at
            ],
        )?;

        Ok((token, expires_at))
    }

    pub fn authenticate_token(&self, token: &str) -> Result<PublicUser> {
        let token = token.trim();
        if token.is_empty() {
            return Err(anyhow!("缺少登录 token"));
        }

        let token_hash = hash_token(token);
        let user = {
            let conn = self.conn()?;
            conn.query_row(
                "SELECT u.id, u.phone, u.email, u.nickname, u.role, u.status, u.avatar_data_url
                 FROM sessions s
                 JOIN users u ON u.id = s.user_id
                 WHERE s.token_hash = ?1
                   AND s.expires_at > ?2
                   AND u.status = 'active'",
                params![token_hash, now()],
                |row| {
                    let phone: Option<String> = row.get(1)?;
                    let email: Option<String> = row.get(2)?;
                    Ok(PublicUser {
                        id: row.get(0)?,
                        account: email
                            .or(phone)
                            .unwrap_or_else(|| row.get(0).unwrap_or_default()),
                        nickname: row.get(3)?,
                        role: row.get(4)?,
                        status: row.get(5)?,
                        avatar_data_url: row.get(6)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("登录已过期，请重新登录"))?
        };
        Ok(user)
    }

    pub fn update_user_nickname(&self, user_id: &str, nickname: &str) -> Result<PublicUser> {
        let nickname = clean_optional(Some(nickname)).ok_or_else(|| anyhow!("昵称不能为空"))?;
        let updated_at = now();
        let updated = self.conn()?.execute(
            "UPDATE users
             SET nickname = ?1, updated_at = ?2
             WHERE id = ?3 AND status = 'active'",
            params![nickname, updated_at, user_id],
        )?;
        if updated == 0 {
            return Err(anyhow!("用户不存在或已停用"));
        }

        self.conn()?
            .query_row(
                "SELECT id, phone, email, nickname, role, status
                 FROM users
                 WHERE id = ?1 AND status = 'active'",
                params![user_id],
                |row| {
                    let phone: Option<String> = row.get(1)?;
                    let email: Option<String> = row.get(2)?;
                    Ok(PublicUser {
                        id: row.get(0)?,
                        account: email
                            .or(phone)
                            .unwrap_or_else(|| row.get(0).unwrap_or_default()),
                        nickname: row.get(3)?,
                        role: row.get(4)?,
                        status: row.get(5)?,
                        avatar_data_url: None,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("用户不存在或已停用"))
    }

    pub fn list_users(&self) -> Result<Vec<AdminUserSummary>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT u.id, u.phone, u.email, u.nickname, u.role, u.status,
                    COUNT(pm.project_id) AS project_count, u.created_at, u.updated_at
             FROM users u
             LEFT JOIN project_members pm ON pm.user_id = u.id
             GROUP BY u.id
             ORDER BY u.created_at DESC",
        )?;

        let users = stmt
            .query_map([], |row| {
                let phone: Option<String> = row.get(1)?;
                let email: Option<String> = row.get(2)?;
                Ok(AdminUserSummary {
                    id: row.get(0)?,
                    account: email
                        .or(phone)
                        .unwrap_or_else(|| row.get(0).unwrap_or_default()),
                    nickname: row.get(3)?,
                    role: row.get(4)?,
                    status: row.get(5)?,
                    project_count: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(users)
    }

    /// 管理员总览：返回所有项目列表，包含创建者信息和最近任务状态
    pub fn list_all_projects_admin(&self) -> Result<Vec<AdminProjectDetail>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.workspace_key, p.workspace_path, p.node_id,
                    p.source_type, p.template, p.status,
                    u.id AS created_by_id,
                    COALESCE(u.email, u.phone, u.id) AS created_by_account,
                    (
                        SELECT t.status FROM tasks t
                        WHERE t.project_id = p.id
                        ORDER BY t.created_at DESC LIMIT 1
                    ) AS last_task_status,
                    (
                        SELECT t.apk_url FROM tasks t
                        WHERE t.project_id = p.id AND t.apk_url IS NOT NULL
                        ORDER BY t.created_at DESC LIMIT 1
                    ) AS last_apk_url,
                    p.updated_at,
                    (
                        SELECT s.device_name FROM sessions s
                        WHERE s.user_id = p.created_by
                        ORDER BY s.created_at DESC LIMIT 1
                    ) AS last_device_name,
                    (
                        SELECT s.apk_version FROM sessions s
                        WHERE s.user_id = p.created_by
                        ORDER BY s.created_at DESC LIMIT 1
                    ) AS last_apk_version
             FROM projects p
             JOIN users u ON u.id = p.created_by
             ORDER BY p.updated_at DESC",
        )?;

        let projects = stmt
            .query_map([], |row| {
                Ok(AdminProjectDetail {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    workspace_key: row.get(2)?,
                    workspace_dir: String::new(), // handler 层填充
                    workspace_path: row.get(3)?,
                    node_id: row.get(4)?,
                    source_type: row.get(5)?,
                    template: row.get(6)?,
                    status: row.get(7)?,
                    created_by_id: row.get(8)?,
                    created_by_account: row.get(9)?,
                    last_task_status: row.get(10)?,
                    last_apk_url: row.get(11)?,
                    updated_at: row.get(12)?,
                    last_device_name: row.get(13)?,
                    last_apk_version: row.get(14)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(projects)
    }

    /// 管理员总览：返回所有未过期的 session（活跃设备）
    pub fn list_active_sessions_admin(&self) -> Result<Vec<AdminSessionEntry>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT s.id, s.user_id,
                    COALESCE(u.email, u.phone, s.user_id) AS user_account,
                    s.device_name, s.apk_version, s.expires_at, s.created_at
             FROM sessions s
             LEFT JOIN users u ON u.id = s.user_id
             WHERE s.expires_at > ?1
             ORDER BY s.created_at DESC",
        )?;

        let sessions = stmt
            .query_map([now()], |row| {
                Ok(AdminSessionEntry {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    user_account: row.get(2)?,
                    device_name: row.get(3)?,
                    apk_version: row.get(4)?,
                    expires_at: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(sessions)
    }

    // ─── 用户头像 ─────────────────────────────────────────────────────────────

    /// 保存用户头像（data URL 格式，如 `data:image/png;base64,...`）
    pub fn save_user_avatar(&self, user_id: &str, avatar_data_url: &str) -> Result<()> {
        self.conn()?.execute(
            "UPDATE users SET avatar_data_url = ?1 WHERE id = ?2",
            params![avatar_data_url, user_id],
        )?;
        Ok(())
    }

    /// 获取用户头像 data URL，不存在时返回 None
    pub fn get_user_avatar(&self, user_id: &str) -> Result<Option<String>> {
        let conn = self.conn()?;
        let result = conn
            .query_row(
                "SELECT avatar_data_url FROM users WHERE id = ?1",
                params![user_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?;
        Ok(result.flatten())
    }
}
