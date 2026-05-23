use anyhow::{Result, anyhow};
use chrono::{Duration, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};
use uuid::Uuid;

pub struct Store {
    conn: Mutex<Connection>,
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
        if template != "android" {
            return Err(anyhow!("目前只支持 android 模板"));
        }

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
            "self" => "self",
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
             SET source_type = ?2,
                 template = ?3,
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
             ORDER BY
                    CASE WHEN p.id = 'elon-self' THEN 0 ELSE 1 END,
                    p.updated_at DESC",
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

    pub fn create_task(&self, project_id: &str, user_id: &str, message: &str) -> Result<String> {
        let id = new_id("tsk");
        let now = now();
        self.conn()?.execute(
            "INSERT INTO tasks (id, project_id, user_id, message, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'running', ?5, ?5)",
            params![id, project_id, user_id, message, now],
        )?;
        self.add_message(project_id, Some(&id), Some(user_id), "user", message)?;
        Ok(id)
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
            let project_id: Option<String> = conn
                .query_row(
                    "SELECT project_id FROM tasks WHERE id = ?1",
                    params![task_id],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(project_id) = project_id {
                drop(conn);
                self.add_message(&project_id, Some(task_id), None, "assistant", reply)?;
            }
        }

        Ok(())
    }

    pub fn add_message(
        &self,
        project_id: &str,
        task_id: Option<&str>,
        user_id: Option<&str>,
        role: &str,
        content: &str,
    ) -> Result<()> {
        self.conn()?.execute(
            "INSERT INTO messages (id, project_id, task_id, user_id, role, content, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                new_id("msg"),
                project_id,
                clean_optional(task_id),
                clean_optional(user_id),
                role,
                content,
                now()
            ],
        )?;
        Ok(())
    }

    /// 标记当前用户任务为 running（服务启动时或任务开始时调用）
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

    fn conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| anyhow!("数据库连接锁已损坏"))
    }
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS users (
          id TEXT PRIMARY KEY,
          phone TEXT UNIQUE,
          email TEXT UNIQUE,
          password_hash TEXT NOT NULL,
          nickname TEXT,
          role TEXT NOT NULL DEFAULT 'user',
          status TEXT NOT NULL DEFAULT 'active',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions (
          id TEXT PRIMARY KEY,
          user_id TEXT NOT NULL,
          token_hash TEXT NOT NULL UNIQUE,
          device_name TEXT,
          expires_at TEXT NOT NULL,
          created_at TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS projects (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          description TEXT,
          workspace_key TEXT NOT NULL UNIQUE,
          template TEXT NOT NULL DEFAULT 'android',
          source_type TEXT NOT NULL DEFAULT 'template',
          repo_url TEXT,
          branch TEXT,
          workspace_path TEXT,
          status TEXT NOT NULL DEFAULT 'active',
          created_by TEXT NOT NULL,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          FOREIGN KEY (created_by) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS project_members (
          project_id TEXT NOT NULL,
          user_id TEXT NOT NULL,
          role TEXT NOT NULL,
          created_at TEXT NOT NULL,
          PRIMARY KEY (project_id, user_id),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS tasks (
          id TEXT PRIMARY KEY,
          project_id TEXT NOT NULL,
          user_id TEXT NOT NULL,
          message TEXT NOT NULL,
          status TEXT NOT NULL,
          git_branch TEXT,
          git_commit TEXT,
          apk_url TEXT,
          error TEXT,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS project_events (
          id TEXT PRIMARY KEY,
          project_id TEXT NOT NULL,
          user_id TEXT,
          event_type TEXT NOT NULL,
          payload_json TEXT,
          created_at TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS artifacts (
          id TEXT PRIMARY KEY,
          project_id TEXT NOT NULL,
          task_id TEXT,
          file_name TEXT NOT NULL,
          file_path TEXT NOT NULL,
          sha256 TEXT,
          size_bytes INTEGER,
          created_at TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (task_id) REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS messages (
          id TEXT PRIMARY KEY,
          project_id TEXT NOT NULL,
          task_id TEXT,
          user_id TEXT,
          role TEXT NOT NULL,
          content TEXT NOT NULL,
          created_at TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (task_id) REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS ws_task_log (
          workspace_user_id TEXT PRIMARY KEY,
          message TEXT NOT NULL,
          status TEXT NOT NULL,
          started_at TEXT NOT NULL,
          finished_at TEXT
        );
        "#,
    )?;
    add_column_if_missing(
        conn,
        "projects",
        "source_type",
        "source_type TEXT NOT NULL DEFAULT 'template'",
    )?;
    add_column_if_missing(conn, "projects", "repo_url", "repo_url TEXT")?;
    add_column_if_missing(conn, "projects", "branch", "branch TEXT")?;
    add_column_if_missing(conn, "projects", "workspace_path", "workspace_path TEXT")?;
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .any(|name| name == column);
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {} ADD COLUMN {}", table, definition),
            [],
        )?;
    }
    Ok(())
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
