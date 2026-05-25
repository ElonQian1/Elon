use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};
use uuid::Uuid;

use crate::store_schema::init_schema;

pub struct Store {
    conn: Mutex<Connection>,
}

const MAX_TASK_EVENTS_PER_TASK: i64 = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAgentSessionState {
    pub native_session_id: String,
    pub chat_bootstrapped: bool,
    pub dev_bootstrapped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicUser {
    pub id: String,
    pub account: String,
    pub nickname: Option<String>,
    pub role: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminUserSummary {
    pub id: String,
    pub account: String,
    pub nickname: Option<String>,
    pub role: String,
    pub status: String,
    pub project_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub workspace_key: String,
    pub template: String,
    pub source_type: String,
    pub repo_url: Option<String>,
    pub branch: Option<String>,
    pub workspace_path: Option<String>,
    pub status: String,
    pub role: String,
    pub last_task_status: Option<String>,
    pub last_apk_url: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ProjectAccess {
    pub id: String,
    pub name: String,
    pub workspace_key: String,
    pub source_type: String,
    pub workspace_path: Option<String>,
    pub role: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub conversation_id: Option<String>,
    pub message: String,
    pub status: String,
    pub apk_url: Option<String>,
    pub error: Option<String>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        init_schema(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

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

        Ok(PublicUser {
            id,
            account,
            nickname: clean_optional(nickname).map(ToOwned::to_owned),
            role: role.to_string(),
            status: "active".into(),
        })
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

        Ok(PublicUser {
            id: row.0,
            account: row.2.or(row.1).unwrap_or(account),
            nickname: row.4,
            role: row.5,
            status: row.6,
        })
    }

    pub fn create_session(
        &self,
        user_id: &str,
        device_name: Option<&str>,
    ) -> Result<(String, String)> {
        let token = format!("tok_{}", Uuid::new_v4().simple());
        let token_hash = hash_token(&token);
        let session_id = new_id("ses");
        let created_at = now();
        let expires_at = (Utc::now() + Duration::days(30)).to_rfc3339();

        self.conn()?.execute(
            "INSERT INTO sessions (id, user_id, token_hash, device_name, expires_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session_id,
                user_id,
                token_hash,
                clean_optional(device_name),
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
        self.conn()?
            .query_row(
                "SELECT u.id, u.phone, u.email, u.nickname, u.role, u.status
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
                    })
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("登录已过期，请重新登录"))
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

    pub fn create_project(
        &self,
        user_id: &str,
        name: &str,
        description: Option<&str>,
        template: Option<&str>,
    ) -> Result<ProjectSummary> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("项目名称不能为空"));
        }

        let id = new_id("prj");
        let workspace_key = id.clone();
        let template = template
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("android");
        // 目前所有受支持的模板都按 Android 脚手架处理；未来扩展时再细分。
        let template = match template {
            "android" | "android_kotlin" | "android_compose" => "android",
            _ => return Err(anyhow!("目前只支持 android 模板")),
        };

        let now = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO projects (
                id, name, description, workspace_key, template, source_type,
                status, created_by, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, 'template', 'active', ?6, ?7, ?7)",
            params![
                id,
                name,
                clean_optional(description),
                workspace_key,
                template,
                user_id,
                now
            ],
        )?;
        tx.execute(
            "INSERT INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, 'owner', ?3)",
            params![id, user_id, now],
        )?;
        tx.execute(
            "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
             VALUES (?1, ?2, ?3, 'project_created', ?4, ?5)",
            params![
                new_id("evt"),
                id,
                user_id,
                serde_json::json!({ "name": name, "template": template }).to_string(),
                now
            ],
        )?;
        tx.commit()?;

        Ok(ProjectSummary {
            id,
            name: name.to_string(),
            description: clean_optional(description).map(ToOwned::to_owned),
            workspace_key,
            template: template.to_string(),
            source_type: "template".into(),
            repo_url: None,
            branch: None,
            workspace_path: None,
            status: "active".into(),
            role: "owner".into(),
            last_task_status: None,
            last_apk_url: None,
            updated_at: now,
        })
    }

    pub fn ensure_device_user(&self, user_id: &str) -> Result<PublicUser> {
        let id = safe_external_id(user_id, "default");
        let now = now();
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO users (
                id, phone, email, password_hash, nickname, role, status, created_at, updated_at
             )
             VALUES (?1, ?2, NULL, 'device-user', 'APK 用户', 'user', 'active', ?3, ?3)",
            params![
                id,
                format!("device-{}", safe_external_id(user_id, "default")),
                now
            ],
        )?;

        conn.query_row(
            "SELECT id, phone, email, nickname, role, status FROM users WHERE id = ?1",
            params![safe_external_id(user_id, "default")],
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
                })
            },
        )
        .map_err(Into::into)
    }

    pub fn ensure_project_for_user(
        &self,
        user_id: &str,
        project_id: &str,
        name: &str,
        description: Option<&str>,
        source_type: &str,
        template: &str,
        workspace_path: Option<&str>,
    ) -> Result<ProjectAccess> {
        let user = self.ensure_device_user(user_id)?;
        let id = safe_external_id(project_id, "project");
        let name = name.trim();
        let name = if name.is_empty() {
            "移动端项目"
        } else {
            name
        };
        let source_type = match source_type.trim() {
            "local_path" => "local_path",
            "github" => "github",
            _ => "template",
        };
        let template = match template.trim() {
            "local" => "local",
            "github" => "github",
            _ => "android",
        };
        let now = now();

        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO projects (
                id, name, description, workspace_key, template, source_type, workspace_path,
                status, created_by, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?1, ?4, ?5, ?6, 'active', ?7, ?8, ?8)",
            params![
                id,
                name,
                clean_optional(description),
                template,
                source_type,
                clean_optional(workspace_path),
                user.id,
                now
            ],
        )?;
        tx.execute(
            "UPDATE projects
             SET source_type = CASE
                     WHEN ?2 != 'template' OR source_type = 'template' THEN ?2
                     ELSE source_type
                 END,
                 template = CASE
                     WHEN ?2 != 'template' OR source_type = 'template' THEN ?3
                     ELSE template
                 END,
                 workspace_path = COALESCE(?4, workspace_path)
             WHERE id = ?1",
            params![id, source_type, template, clean_optional(workspace_path)],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, 'owner', ?3)",
            params![id, user.id, now],
        )?;
        tx.commit()?;
        drop(conn);

        self.get_project_access(&user.id, &id)
    }

    pub fn list_projects_for_user(&self, user_id: &str) -> Result<Vec<ProjectSummary>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                    p.source_type, p.repo_url, p.branch, p.workspace_path, p.status,
                    pm.role,
                    (
                        SELECT t.status FROM tasks t
                        WHERE t.project_id = p.id
                        ORDER BY t.created_at DESC
                        LIMIT 1
                    ) AS last_task_status,
                    (
                        SELECT t.apk_url FROM tasks t
                        WHERE t.project_id = p.id AND t.apk_url IS NOT NULL
                        ORDER BY t.created_at DESC
                        LIMIT 1
                    ) AS last_apk_url,
                    p.updated_at
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id
             WHERE pm.user_id = ?1 AND p.status != 'deleted'
             ORDER BY p.updated_at DESC",
        )?;

        let projects = stmt
            .query_map(params![user_id], |row| {
                Ok(ProjectSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    workspace_key: row.get(3)?,
                    template: row.get(4)?,
                    source_type: row.get(5)?,
                    repo_url: row.get(6)?,
                    branch: row.get(7)?,
                    workspace_path: row.get(8)?,
                    status: row.get(9)?,
                    role: row.get(10)?,
                    last_task_status: row.get(11)?,
                    last_apk_url: row.get(12)?,
                    updated_at: row.get(13)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(projects)
    }

    pub fn get_project_access(&self, user_id: &str, project_id: &str) -> Result<ProjectAccess> {
        self.conn()?
            .query_row(
                "SELECT p.id, p.name, p.workspace_key, p.source_type, p.workspace_path, pm.role, p.status
                 FROM projects p
                 JOIN project_members pm ON pm.project_id = p.id
                 WHERE p.id = ?1 AND pm.user_id = ?2 AND p.status != 'deleted'",
                params![project_id, user_id],
                |row| {
                    Ok(ProjectAccess {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        workspace_key: row.get(2)?,
                        source_type: row.get(3)?,
                        workspace_path: row.get(4)?,
                        role: row.get(5)?,
                        status: row.get(6)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"))
    }

    pub fn update_project_git_config(
        &self,
        user_id: &str,
        project_id: &str,
        repo_url: &str,
        branch: &str,
    ) -> Result<ProjectAccess> {
        let repo_url = repo_url.trim();
        if repo_url.is_empty() {
            return Err(anyhow!("Git 仓库地址不能为空"));
        }
        let branch = branch.trim();
        if branch.is_empty() {
            return Err(anyhow!("Git 分支不能为空"));
        }

        let now = now();
        let conn = self.conn()?;
        let changed = conn.execute(
            "UPDATE projects
             SET source_type = 'github',
                 template = 'github',
                 repo_url = ?1,
                 branch = ?2,
                 updated_at = ?3
             WHERE id = ?4
               AND EXISTS (
                 SELECT 1 FROM project_members
                 WHERE project_id = ?4
                   AND user_id = ?5
                   AND role IN ('owner', 'editor')
               )",
            params![repo_url, branch, now, project_id, user_id],
        )?;
        if changed == 0 {
            return Err(anyhow!("项目不存在，或当前用户无权配置 Git"));
        }
        drop(conn);

        self.get_project_access(user_id, project_id)
    }

    pub fn ensure_conversation(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        title: Option<&str>,
    ) -> Result<String> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let now = now();
        self.conn()?.execute(
            "INSERT INTO conversations (
                project_id, user_id, id, title, status, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?5)
             ON CONFLICT(project_id, user_id, id) DO UPDATE SET
                title = COALESCE(excluded.title, conversations.title),
                updated_at = excluded.updated_at",
            params![
                project_id,
                user_id,
                conversation_id,
                clean_optional(title),
                now
            ],
        )?;
        Ok(conversation_id)
    }

    pub fn create_task(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        message: &str,
    ) -> Result<String> {
        self.create_task_with_client_request(project_id, user_id, conversation_id, None, message)
    }

    pub fn create_task_with_client_request(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        client_request_id: Option<&str>,
        message: &str,
    ) -> Result<String> {
        let id = new_id("tsk");
        let now = now();
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let client_request_id = clean_optional(client_request_id);
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO conversations (
                project_id, user_id, id, status, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, 'active', ?4, ?4)
             ON CONFLICT(project_id, user_id, id) DO UPDATE SET updated_at = excluded.updated_at",
            params![project_id, user_id, conversation_id, now],
        )?;
        tx.execute(
            "INSERT INTO tasks (
                id, project_id, user_id, conversation_id, client_request_id, message, status, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'running', ?7, ?7)",
            params![
                id,
                project_id,
                user_id,
                conversation_id,
                client_request_id,
                message,
                now
            ],
        )?;
        tx.execute(
            "INSERT INTO messages (
                id, project_id, conversation_id, task_id, user_id, role, content, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, 'user', ?6, ?7)",
            params![
                new_id("msg"),
                project_id,
                conversation_id,
                id,
                user_id,
                message,
                now
            ],
        )?;
        tx.commit()?;
        Ok(id)
    }

    pub fn get_task_by_client_request(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        client_request_id: &str,
    ) -> Result<Option<TaskSnapshot>> {
        let Some(client_request_id) = clean_optional(Some(client_request_id)) else {
            return Ok(None);
        };
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        self.conn()?
            .query_row(
                "SELECT id, project_id, user_id, conversation_id, message, status, apk_url, error
                 FROM tasks
                 WHERE project_id = ?1
                   AND user_id = ?2
                   AND conversation_id = ?3
                   AND client_request_id = ?4
                 ORDER BY created_at DESC
                 LIMIT 1",
                params![project_id, user_id, conversation_id, client_request_id],
                |row| {
                    Ok(TaskSnapshot {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        user_id: row.get(2)?,
                        conversation_id: row.get(3)?,
                        message: row.get(4)?,
                        status: row.get(5)?,
                        apk_url: row.get(6)?,
                        error: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn set_task_running(&self, task_id: &str) -> Result<()> {
        self.conn()?.execute(
            "UPDATE tasks
             SET status = 'running', error = NULL, updated_at = ?1
             WHERE id = ?2",
            params![now(), task_id],
        )?;
        Ok(())
    }

    pub fn finish_task(
        &self,
        task_id: &str,
        status: &str,
        reply: Option<&str>,
        apk_url: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        let now = now();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE tasks
             SET status = ?1, apk_url = ?2, error = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                status,
                clean_optional(apk_url),
                clean_optional(error),
                now,
                task_id
            ],
        )?;
        conn.execute(
            "UPDATE projects
             SET updated_at = ?1
             WHERE id = (SELECT project_id FROM tasks WHERE id = ?2)",
            params![now, task_id],
        )?;

        if let Some(reply) = clean_optional(reply) {
            let task_context: Option<(String, String, Option<String>)> = conn
                .query_row(
                    "SELECT project_id, user_id, conversation_id FROM tasks WHERE id = ?1",
                    params![task_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?;
            if let Some((project_id, user_id, conversation_id)) = task_context {
                drop(conn);
                self.add_message(
                    &project_id,
                    conversation_id.as_deref(),
                    Some(task_id),
                    Some(&user_id),
                    "assistant",
                    reply,
                )?;
            }
        }

        Ok(())
    }

    pub fn record_task_event(&self, task_id: &str, event_json: &str) -> Result<()> {
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO task_events (id, task_id, event_json, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![new_id("tev"), task_id, event_json, now()],
        )?;
        tx.execute(
            "DELETE FROM task_events
             WHERE task_id = ?1
               AND rowid NOT IN (
                 SELECT rowid
                 FROM task_events
                 WHERE task_id = ?1
                 ORDER BY created_at DESC, rowid DESC
                 LIMIT ?2
               )",
            params![task_id, MAX_TASK_EVENTS_PER_TASK],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_task_events(&self, task_id: &str, limit: usize) -> Result<Vec<String>> {
        let limit = limit.clamp(1, MAX_TASK_EVENTS_PER_TASK as usize) as i64;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT event_json
             FROM (
               SELECT rowid, created_at, event_json
               FROM task_events
               WHERE task_id = ?1
               ORDER BY created_at DESC, rowid DESC
               LIMIT ?2
             )
             ORDER BY created_at ASC, rowid ASC",
        )?;
        let events = stmt
            .query_map(params![task_id, limit], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(events)
    }

    pub fn add_message(
        &self,
        project_id: &str,
        conversation_id: Option<&str>,
        task_id: Option<&str>,
        user_id: Option<&str>,
        role: &str,
        content: &str,
    ) -> Result<()> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        self.conn()?.execute(
            "INSERT INTO messages (
                id, project_id, conversation_id, task_id, user_id, role, content, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                new_id("msg"),
                project_id,
                conversation_id,
                clean_optional(task_id),
                clean_optional(user_id),
                role,
                content,
                now()
            ],
        )?;
        Ok(())
    }

    pub fn list_recent_conversation_messages(
        &self,
        project_id: &str,
        conversation_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ConversationMessage>> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let limit = limit.clamp(1, 30) as i64;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT role, content
             FROM (
                SELECT role, content, created_at, id
                FROM messages
                WHERE project_id = ?1
                  AND conversation_id = ?2
                ORDER BY created_at DESC, id DESC
                LIMIT ?3
             )
             ORDER BY created_at ASC, id ASC",
        )?;
        let messages = stmt
            .query_map(params![project_id, conversation_id, limit], |row| {
                Ok(ConversationMessage {
                    role: row.get(0)?,
                    content: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(messages)
    }

    pub fn get_native_agent_session(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        provider: &str,
        agent_id: &str,
        workspace_path: &str,
    ) -> Result<Option<String>> {
        Ok(self
            .get_native_agent_session_state(
                project_id,
                user_id,
                conversation_id,
                provider,
                agent_id,
                workspace_path,
            )?
            .map(|state| state.native_session_id))
    }

    pub fn get_native_agent_session_state(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        provider: &str,
        agent_id: &str,
        workspace_path: &str,
    ) -> Result<Option<NativeAgentSessionState>> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        self.conn()?
            .query_row(
                "SELECT native_session_id, chat_bootstrapped, dev_bootstrapped
                 FROM agent_native_sessions
                 WHERE project_id = ?1
                   AND user_id = ?2
                   AND conversation_id = ?3
                   AND provider = ?4
                   AND agent_id = ?5
                   AND workspace_path = ?6
                   AND status = 'active'
                 ORDER BY updated_at DESC
                 LIMIT 1",
                params![
                    project_id,
                    user_id,
                    conversation_id,
                    provider,
                    agent_id,
                    workspace_path
                ],
                |row| {
                    Ok(NativeAgentSessionState {
                        native_session_id: row.get(0)?,
                        chat_bootstrapped: row.get::<_, i64>(1)? != 0,
                        dev_bootstrapped: row.get::<_, i64>(2)? != 0,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn upsert_native_agent_session(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        provider: &str,
        agent_id: &str,
        workspace_path: &str,
        native_session_id: &str,
    ) -> Result<()> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let now = now();
        self.conn()?.execute(
            "INSERT INTO agent_native_sessions (
                id, project_id, user_id, conversation_id, provider, agent_id,
                workspace_path, native_session_id, status, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', ?9, ?9)
             ON CONFLICT(project_id, user_id, conversation_id, provider, agent_id, workspace_path)
             DO UPDATE SET
                native_session_id = excluded.native_session_id,
                chat_bootstrapped = CASE
                    WHEN agent_native_sessions.native_session_id = excluded.native_session_id
                    THEN agent_native_sessions.chat_bootstrapped
                    ELSE 0
                END,
                dev_bootstrapped = CASE
                    WHEN agent_native_sessions.native_session_id = excluded.native_session_id
                    THEN agent_native_sessions.dev_bootstrapped
                    ELSE 0
                END,
                status = 'active',
                updated_at = excluded.updated_at",
            params![
                new_id("ans"),
                project_id,
                user_id,
                conversation_id,
                provider,
                agent_id,
                workspace_path,
                native_session_id,
                now
            ],
        )?;
        Ok(())
    }

    /// 标记当前用户任务为 running（服务启动时或任务开始时调用）
    pub fn upsert_native_agent_session_if_no_active(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        provider: &str,
        agent_id: &str,
        workspace_path: &str,
        native_session_id: &str,
    ) -> Result<bool> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let conn = self.conn()?;
        let existing: Option<String> = conn
            .query_row(
                "SELECT native_session_id
                 FROM agent_native_sessions
                 WHERE project_id = ?1
                   AND user_id = ?2
                   AND conversation_id = ?3
                   AND provider = ?4
                   AND agent_id = ?5
                   AND workspace_path = ?6
                   AND status = 'active'
                 LIMIT 1",
                params![
                    project_id,
                    user_id,
                    conversation_id,
                    provider,
                    agent_id,
                    workspace_path
                ],
                |row| row.get(0),
            )
            .optional()?;
        if existing.is_some() {
            return Ok(false);
        }

        let now = now();
        conn.execute(
            "INSERT INTO agent_native_sessions (
                id, project_id, user_id, conversation_id, provider, agent_id,
                workspace_path, native_session_id, status, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', ?9, ?9)
             ON CONFLICT(project_id, user_id, conversation_id, provider, agent_id, workspace_path)
             DO UPDATE SET
                native_session_id = excluded.native_session_id,
                chat_bootstrapped = CASE
                    WHEN agent_native_sessions.native_session_id = excluded.native_session_id
                    THEN agent_native_sessions.chat_bootstrapped
                    ELSE 0
                END,
                dev_bootstrapped = CASE
                    WHEN agent_native_sessions.native_session_id = excluded.native_session_id
                    THEN agent_native_sessions.dev_bootstrapped
                    ELSE 0
                END,
                status = 'active',
                updated_at = excluded.updated_at",
            params![
                new_id("ans"),
                project_id,
                user_id,
                conversation_id,
                provider,
                agent_id,
                workspace_path,
                native_session_id,
                now
            ],
        )?;
        Ok(true)
    }

    pub fn deactivate_native_agent_session(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        provider: &str,
        agent_id: &str,
        workspace_path: &str,
        native_session_id: &str,
    ) -> Result<()> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        self.conn()?.execute(
            "UPDATE agent_native_sessions
             SET status = 'stale', updated_at = ?1
             WHERE project_id = ?2
               AND user_id = ?3
               AND conversation_id = ?4
               AND provider = ?5
               AND agent_id = ?6
               AND workspace_path = ?7
               AND native_session_id = ?8
               AND status = 'active'",
            params![
                now(),
                project_id,
                user_id,
                conversation_id,
                provider,
                agent_id,
                workspace_path,
                native_session_id
            ],
        )?;
        Ok(())
    }

    pub fn mark_native_agent_session_bootstrapped(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        provider: &str,
        agent_id: &str,
        workspace_path: &str,
        native_session_id: &str,
        development: bool,
    ) -> Result<()> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let column = if development {
            "dev_bootstrapped"
        } else {
            "chat_bootstrapped"
        };
        let sql = format!(
            "UPDATE agent_native_sessions
             SET {column} = 1, updated_at = ?1
             WHERE project_id = ?2
               AND user_id = ?3
               AND conversation_id = ?4
               AND provider = ?5
               AND agent_id = ?6
               AND workspace_path = ?7
               AND native_session_id = ?8
               AND status = 'active'"
        );
        self.conn()?.execute(
            &sql,
            params![
                now(),
                project_id,
                user_id,
                conversation_id,
                provider,
                agent_id,
                workspace_path,
                native_session_id
            ],
        )?;
        Ok(())
    }

    pub fn ws_task_started(&self, workspace_user_id: &str, message: &str) -> Result<()> {
        self.conn()?.execute(
            "INSERT OR REPLACE INTO ws_task_log (workspace_user_id, message, status, started_at, finished_at)
             VALUES (?1, ?2, 'running', ?3, NULL)",
            params![workspace_user_id, message, now()],
        )?;
        Ok(())
    }

    /// 标记任务完成（status: done / error）
    pub fn ws_task_finished(&self, workspace_user_id: &str, status: &str) -> Result<()> {
        self.conn()?.execute(
            "UPDATE ws_task_log SET status = ?1, finished_at = ?2 WHERE workspace_user_id = ?3",
            params![status, now(), workspace_user_id],
        )?;
        Ok(())
    }

    /// 查询该用户是否有被中断的任务，返回中断时的消息内容
    pub fn get_interrupted_ws_task(&self, workspace_user_id: &str) -> Result<Option<String>> {
        let conn = self.conn()?;
        let msg: Option<String> = conn
            .query_row(
                "SELECT message FROM ws_task_log WHERE workspace_user_id = ?1 AND status = 'interrupted'",
                params![workspace_user_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(msg)
    }

    /// 服务器启动时：将所有 running 状态的任务标记为 interrupted
    pub fn mark_interrupted_running_ws_tasks(&self) -> Result<usize> {
        let n = self.conn()?.execute(
            "UPDATE ws_task_log SET status = 'interrupted', finished_at = ?1 WHERE status = 'running'",
            params![now()],
        )?;
        Ok(n)
    }

    pub fn mark_interrupted_running_tasks(&self) -> Result<usize> {
        let n = self.conn()?.execute(
            "UPDATE tasks
             SET status = 'interrupted',
                 error = COALESCE(error, 'server restarted before task finished'),
                 updated_at = ?1
             WHERE status = 'running'",
            params![now()],
        )?;
        Ok(n)
    }

    fn conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| anyhow!("数据库连接锁已损坏"))
    }
}

fn safe_external_id(value: &str, fallback: &str) -> String {
    let safe = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect::<String>();
    if safe.is_empty() {
        fallback.into()
    } else {
        safe
    }
}

fn normalize_account(account: &str) -> Result<String> {
    let account = account.trim().to_ascii_lowercase();
    if account.len() < 3 {
        return Err(anyhow!("账号至少需要 3 个字符"));
    }
    Ok(account)
}

fn validate_password(password: &str) -> Result<()> {
    if password.chars().count() < 6 {
        return Err(anyhow!("密码至少需要 6 个字符"));
    }
    Ok(())
}

fn account_columns(account: &str) -> (Option<String>, Option<String>) {
    if account.contains('@') {
        (None, Some(account.to_string()))
    } else {
        (Some(account.to_string()), None)
    }
}

fn clean_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn new_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn hash_password(password: &str) -> String {
    let salt = Uuid::new_v4().simple().to_string();
    let digest = password_digest(&salt, password);
    format!("sha256${}${}", salt, digest)
}

fn verify_password(password: &str, stored: &str) -> bool {
    let mut parts = stored.split('$');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some("sha256"), Some(salt), Some(expected), None) => {
            password_digest(salt, password) == expected
        }
        _ => false,
    }
}

fn password_digest(salt: &str, password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> Store {
        let path =
            std::env::temp_dir().join(format!("elon_store_test_{}.db", Uuid::new_v4().simple()));
        Store::open(&path).expect("store should open")
    }

    fn temp_task(store: &Store) -> String {
        let user = store
            .create_user("events@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Task Events", None, None)
            .expect("project should be created");
        store
            .create_task(&project.id, &user.id, Some("conv"), "run task")
            .expect("task should be created")
    }

    fn event_message(raw: &str) -> String {
        serde_json::from_str::<serde_json::Value>(raw)
            .expect("event should be json")
            .get("message")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn lists_latest_task_events_in_chronological_order() {
        let store = temp_store();
        let task_id = temp_task(&store);

        for step in 0..5 {
            store
                .record_task_event(
                    &task_id,
                    &format!(r#"{{"type":"progress","message":"step {step}"}}"#),
                )
                .expect("event should be recorded");
        }

        let messages = store
            .list_task_events(&task_id, 3)
            .expect("events should list")
            .into_iter()
            .map(|raw| event_message(&raw))
            .collect::<Vec<_>>();

        assert_eq!(messages, vec!["step 2", "step 3", "step 4"]);
    }

    #[test]
    fn prunes_old_task_events_per_task() {
        let store = temp_store();
        let task_id = temp_task(&store);

        for step in 0..(MAX_TASK_EVENTS_PER_TASK + 5) {
            store
                .record_task_event(
                    &task_id,
                    &format!(r#"{{"type":"progress","message":"step {step}"}}"#),
                )
                .expect("event should be recorded");
        }

        let events = store
            .list_task_events(&task_id, MAX_TASK_EVENTS_PER_TASK as usize + 100)
            .expect("events should list");

        assert_eq!(events.len(), MAX_TASK_EVENTS_PER_TASK as usize);
        assert_eq!(event_message(events.first().unwrap()), "step 5");
        assert_eq!(event_message(events.last().unwrap()), "step 1004");
    }
}
