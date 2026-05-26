use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

use crate::store_schema::apply_migrations;

mod common;
mod friend_messages;
mod friends;
mod native_sessions;
mod projects;
mod tasks;
mod users;

use common::{
    account_columns, clean_optional, hash_password, hash_token, new_id, normalize_account, now,
    safe_external_id, validate_password, verify_password,
};

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
pub struct FriendProfile {
    pub id: String,
    pub account: String,
    pub nickname: Option<String>,
    pub phone: Option<String>,
    pub friend_since: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FriendSearchResult {
    pub user: FriendProfile,
    pub already_friend: bool,
    pub is_self: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddFriendResult {
    pub friend: FriendProfile,
    pub already_friend: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FriendChatMessage {
    pub id: String,
    pub sender_user_id: String,
    pub receiver_user_id: String,
    pub content: String,
    pub created_at: String,
    pub outgoing: bool,
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

/// 管理员后台「用户项目」总览，每行代表一个项目
#[derive(Debug, Clone, Serialize)]
pub struct AdminProjectDetail {
    pub id: String,
    pub name: String,
    pub workspace_key: String,
    /// 项目在服务器上的实际绝对路径（handler 层注入）
    pub workspace_dir: String,
    /// local_path 类型项目在 DB 里记录的自定义路径
    pub workspace_path: Option<String>,
    pub source_type: String,
    pub template: String,
    pub status: String,
    pub created_by_id: String,
    pub created_by_account: String,
    pub last_task_status: Option<String>,
    pub last_apk_url: Option<String>,
    pub updated_at: String,
    /// 创建者最近登录设备名
    pub last_device_name: Option<String>,
    /// 创建者最近登录时的 APK 版本
    pub last_apk_version: Option<String>,
}

/// 管理员后台「活跃设备」总览，每行代表一个登录 session
#[derive(Debug, Clone, Serialize)]
pub struct AdminSessionEntry {
    pub id: String,
    pub user_id: String,
    pub user_account: String,
    pub device_name: Option<String>,
    pub apk_version: Option<String>,
    pub expires_at: String,
    pub created_at: String,
}

/// 管理员后台「会话列表」，每行代表一个 conversation
#[derive(Debug, Clone, Serialize)]
pub struct AdminConversationEntry {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub user_account: String,
    pub title: Option<String>,
    pub status: String,
    pub message_count: i64,
    pub task_count: i64,
    pub last_task_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 项目商店卡片：公开展示的项目摘要（不含敏感路径信息）
#[derive(Debug, Clone, Serialize)]
pub struct PublicProjectItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub template: String,
    pub owner_account: String,
    pub member_count: i64,
    pub is_public: bool,
    pub join_mode: String, // "open" | "approval" | "invite"
    pub last_task_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 项目成员条目（商店/成员列表用）
#[derive(Debug, Clone, Serialize)]
pub struct ProjectMemberEntry {
    pub user_id: String,
    pub account: String,
    pub role: String, // "owner" | "member" | "observer"
    pub joined_at: String,
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
        apply_migrations(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
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
