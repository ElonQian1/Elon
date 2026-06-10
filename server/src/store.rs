use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

use crate::store_schema::apply_migrations;

mod admin_stats;
mod billing;
mod billing_alerts;
mod billing_price_rules;
#[cfg(test)]
mod billing_reservation_tests;
mod billing_reservations;
mod build_quota;
mod common;
mod compute_metering;
mod conversations;
mod friend_messages;
mod friends;
mod groups;
mod join_requests;
mod native_sessions;
mod node_compute_runs;
mod node_hardware;
mod node_ledger;
#[cfg(test)]
mod node_payout_tests;
mod node_payouts;
mod pc_project_binding;
mod project_execution_sessions;
mod project_member_conversations;
mod project_space;
mod project_workspace_health_snapshots;
mod projects;
mod social_ai_messages;
mod social_ai_pending;
mod social_ai_selected;
mod store_types;
mod system_projects;
mod tasks;
mod token_usage;
mod user_archive;
mod user_memories;
mod users;

pub use admin_stats::{
    estimate_cost_cny, AdminAccountingAuditRow, AdminDayRow, AdminFeatureRow, AdminModelRow,
    AdminPlatformSummary, AdminTrendRow, AdminUserDetail, AdminUserUsageRow, UserQuota,
};
pub use billing::{AdminBalanceRow, AdminBillingEventRow, BillingEvent, RechargeRecord};
pub use billing_alerts::BillingAlertRow;
pub use billing_price_rules::{BillingPriceRule, BillingPriceRuleUpsert, BillingPriceSnapshot};
pub use billing_reservations::{BillingReservationOutcome, BillingReservationRequest};
use common::{
    account_columns, clean_optional, hash_password, hash_token, new_id, normalize_account, now,
    safe_external_id, validate_password, verify_password,
};
pub use compute_metering::ComputeMeterEvent;
pub use node_compute_runs::{
    NodeComputeRun, NodeComputeRunFinish, NodeComputeRunStart, NodeQualityScore,
};
pub use node_ledger::{NodeBalance, NodeCredential, NodeTransaction, SettleParams};
pub use node_payouts::CreateNodePayout;
pub use project_execution_sessions::{
    ProjectExecutionSession, ProjectExecutionSessionFinish, ProjectExecutionSessionStart,
};
pub use project_workspace_health_snapshots::ProjectWorkspaceHealthSnapshotWrite;
pub(crate) use social_ai_messages::{
    SocialAiHistoryMessage, SOCIAL_AI_DISPLAY_NAME, SOCIAL_AI_FRIEND_ACCOUNT,
    SOCIAL_AI_FRIEND_NAME, SOCIAL_AI_FRIEND_PREVIEW, SOCIAL_AI_USER_ID,
};
pub(crate) use social_ai_pending::SocialAiPendingMention;
pub use store_types::JoinRequestRecord;
pub use store_types::*;
pub(crate) use system_projects::{
    is_system_project_name, is_system_project_source_type, system_project_key_for_source_type,
    CHAT_MEMORY_PROJECT_NAME, PHONE_CONTROL_PROJECT_NAME,
};
pub use token_usage::{
    TokenUsageAccountingResult, TokenUsageBillingCharge, TokenUsageRecord, UsageDayRow,
    UsageFeatureRow, UsageModeRow, UsageQuota, UsageStats, UsageTotals,
};
pub use user_memories::{
    UserMemory, MEMORY_SCOPE_CHAT, MEMORY_SCOPE_GLOBAL, MEMORY_SCOPE_PHONE_CONTROL,
    MEMORY_SCOPE_PROJECT,
};

pub struct Store {
    conn: Mutex<Connection>,
}

const MAX_TASK_EVENTS_PER_TASK: i64 = 1000;

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
    ) -> Result<CreateProjectResult> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("项目名称不能为空"));
        }
        if is_system_project_name(name) {
            return Err(anyhow!("该名称是系统保留项目，请更换项目名称"));
        }

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

        if let Some(project) = find_owner_project_by_name(&conn, user_id, name)? {
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }

        let id = new_id("prj");
        let workspace_key = id.clone();
        let description = clean_optional(description);
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO projects (
                id, name, description, workspace_key, template, source_type,
                status, created_by, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, 'template', 'active', ?6, ?7, ?7)",
            params![id, name, description, workspace_key, template, user_id, now],
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

        Ok(CreateProjectResult {
            project: ProjectSummary {
                id,
                name: name.to_string(),
                description: description.map(ToOwned::to_owned),
                workspace_key,
                template: template.to_string(),
                source_type: "template".into(),
                repo_url: None,
                branch: None,
                workspace_path: None,
                node_id: None,
                status: "active".into(),
                role: "owner".into(),
                member_count: 1,
                is_public: false,
                join_mode: "open".into(),
                last_task_status: None,
                last_apk_url: None,
                icon_data_url: None,
                updated_at: now,
            },
            reused_existing: false,
        })
    }

    /// 注册一个指向外部本地路径的项目（如 D:\rust\active-projects\bb64a）。
    /// source_type='local_path'，workspace_path 写入项目记录。
    /// 同名项目复用现有记录（reused_existing=true）。
    pub fn register_external_project(
        &self,
        user_id: &str,
        project_id: Option<&str>,
        name: &str,
        description: Option<&str>,
        workspace_path: &str,
        node_id: Option<&str>,
    ) -> Result<CreateProjectResult> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("项目名称不能为空"));
        }
        if is_system_project_name(name) {
            return Err(anyhow!("该名称是系统保留项目，不能绑定为外部代码项目"));
        }
        let workspace_path = workspace_path.trim();
        if workspace_path.is_empty() {
            return Err(anyhow!("workspace_path 不能为空"));
        }
        let template = "local";
        let source_type = "local_path";
        let node_id = clean_optional(node_id);

        let now = now();
        let conn = self.conn()?;

        let requested_project_id = project_id.map(str::trim).filter(|v| !v.is_empty());
        if let Some(project_id) = requested_project_id {
            let (role, source_type): (String, String) = conn
                .query_row(
                    "SELECT pm.role, p.source_type
                     FROM projects p
                     JOIN project_members pm ON pm.project_id = p.id
                     WHERE p.id = ?1 AND pm.user_id = ?2 AND p.status != 'deleted'",
                    params![project_id, user_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
                .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"))?;
            if role != "owner" {
                anyhow::bail!("只有项目 owner 才能绑定 PC 本地路径");
            }
            if is_system_project_source_type(&source_type) {
                anyhow::bail!("系统归档项目不能绑定为外部代码工作区");
            }

            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "UPDATE projects
                 SET name = ?2,
                     description = COALESCE(?3, description),
                     template = 'local',
                     source_type = 'local_path',
                     workspace_path = ?4,
                     node_id = ?5,
                     updated_at = ?6
                 WHERE id = ?1 AND status != 'deleted'",
                params![
                    project_id,
                    name,
                    clean_optional(description),
                    workspace_path,
                    node_id,
                    now
                ],
            )?;
            tx.execute(
                "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
                 VALUES (?1, ?2, ?3, 'project_bound_external', ?4, ?5)",
                params![
                    new_id("evt"),
                    project_id,
                    user_id,
                    serde_json::json!({
                        "workspace_path": workspace_path,
                        "node_id": node_id,
                    })
                    .to_string(),
                    now
                ],
            )?;
            tx.commit()?;

            let project = find_project_by_id_for_user(&conn, user_id, project_id)?
                .ok_or_else(|| anyhow!("项目绑定后无法读取"))?;
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }

        if let Some(mut project) = find_owner_project_by_name(&conn, user_id, name)? {
            conn.execute(
                "UPDATE projects
                 SET template = 'local',
                     source_type = 'local_path',
                     workspace_path = ?2,
                     node_id = ?3,
                     updated_at = ?4
                 WHERE id = ?1",
                params![&project.id, workspace_path, node_id, now],
            )?;
            project.template = "local".into();
            project.source_type = "local_path".into();
            project.workspace_path = Some(workspace_path.to_string());
            project.node_id = node_id.map(ToOwned::to_owned);
            project.updated_at = now;
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }

        let id = new_id("prj");
        let workspace_key = id.clone();
        let description = clean_optional(description);
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO projects (
                id, name, description, workspace_key, template, source_type, workspace_path, node_id,
                status, created_by, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', ?9, ?10, ?10)",
            params![
                id,
                name,
                description,
                workspace_key,
                template,
                source_type,
                workspace_path,
                node_id,
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
             VALUES (?1, ?2, ?3, 'project_registered_external', ?4, ?5)",
            params![
                new_id("evt"),
                id,
                user_id,
                serde_json::json!({
                    "name": name,
                    "workspace_path": workspace_path,
                    "node_id": node_id,
                }).to_string(),
                now
            ],
        )?;
        tx.commit()?;

        Ok(CreateProjectResult {
            project: ProjectSummary {
                id,
                name: name.to_string(),
                description: description.map(ToOwned::to_owned),
                workspace_key,
                template: template.to_string(),
                source_type: source_type.into(),
                repo_url: None,
                branch: None,
                workspace_path: Some(workspace_path.to_string()),
                node_id: node_id.map(ToOwned::to_owned),
                status: "active".into(),
                role: "owner".into(),
                member_count: 1,
                is_public: false,
                join_mode: "open".into(),
                last_task_status: None,
                last_apk_url: None,
                icon_data_url: None,
                updated_at: now,
            },
            reused_existing: false,
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
                    avatar_data_url: None,
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
                 workspace_path = CASE
                     WHEN node_id IS NOT NULL AND node_id != '' THEN workspace_path
                     ELSE COALESCE(?4, workspace_path)
                 END
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

        // 如果项目被标记为 deleted（例如被迁移脚本误删），尝试透明重定向到同名 active 项目，
        // 或将其恢复为 active，避免 APK 收到"项目不存在"错误。
        {
            let conn2 = self.conn()?;
            let is_deleted: bool = conn2
                .query_row(
                    "SELECT status = 'deleted' FROM projects WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if is_deleted {
                let active_id: Option<String> = conn2
                    .query_row(
                        "SELECT id FROM projects \
                         WHERE created_by = ?1 AND name = ?2 AND status != 'deleted' LIMIT 1",
                        params![user.id, name],
                        |row| row.get(0),
                    )
                    .optional()?;
                drop(conn2);
                if let Some(redirect_id) = active_id {
                    // 同名 active 项目已存在，直接返回它（透明重定向）
                    return self.get_project_access(&user.id, &redirect_id);
                } else {
                    // 无同名 active 项目，将本项目恢复为 active
                    self.conn()?.execute(
                        "UPDATE projects SET status = 'active' WHERE id = ?1",
                        params![id],
                    )?;
                }
            }
        }

        self.get_project_access(&user.id, &id)
    }

    pub fn list_projects_for_user(&self, user_id: &str) -> Result<Vec<ProjectSummary>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                    p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id, p.status,
                    pm.role,
                    (SELECT COUNT(*) FROM project_members pm2 WHERE pm2.project_id = p.id) AS member_count,
                    p.is_public,
                    p.join_mode,
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
                    p.icon_data_url,
                    p.updated_at
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id
             WHERE pm.user_id = ?1 AND p.status != 'deleted'
             ORDER BY p.updated_at DESC",
        )?;

        let projects = stmt
            .query_map(params![user_id], project_summary_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(projects)
    }

    pub fn get_project_access(&self, user_id: &str, project_id: &str) -> Result<ProjectAccess> {
        self.conn()?
            .query_row(
                "SELECT p.id, p.name, p.workspace_key, p.source_type, p.workspace_path, p.node_id, pm.role, p.status
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
                        node_id: row.get(5)?,
                        role: row.get(6)?,
                        status: row.get(7)?,
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

    /// 定期清理：将 running 超过指定秒数的任务标记为 failed。
    /// 用于防止 PC 节点断线但任务因异常未收到 CliDone 而永久卡住。
    pub fn mark_stale_running_tasks(&self, older_than_secs: u64) -> Result<usize> {
        use chrono::{Duration, Utc};
        let cutoff = (Utc::now() - Duration::seconds(older_than_secs as i64)).to_rfc3339();
        let n = self.conn()?.execute(
            "UPDATE tasks
             SET status = 'failed',
                 error = COALESCE(error, 'PC节点断线或任务超时自动终止'),
                 updated_at = ?1
             WHERE status = 'running'
               AND created_at < ?2",
            params![now(), cutoff],
        )?;
        Ok(n)
    }

    fn conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| anyhow!("数据库连接锁已损坏"))
    }
}

// ── 私有帮助函数 ──────────────────────────────────────────────────────────────

fn project_summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectSummary> {
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
        node_id: row.get(9)?,
        status: row.get(10)?,
        role: row.get(11)?,
        member_count: row.get(12)?,
        is_public: row.get::<_, i64>(13)? != 0,
        join_mode: row.get(14)?,
        last_task_status: row.get(15)?,
        last_apk_url: row.get(16)?,
        icon_data_url: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

fn find_owner_project_by_name(
    conn: &Connection,
    user_id: &str,
    name: &str,
) -> Result<Option<ProjectSummary>> {
    Ok(conn
        .query_row(
            "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                    p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id, p.status,
                    COALESCE(pm.role, 'owner') AS role,
                    (SELECT COUNT(*) FROM project_members pm2 WHERE pm2.project_id = p.id) AS member_count,
                    p.is_public,
                    p.join_mode,
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
                    p.icon_data_url,
                    p.updated_at
             FROM projects p
             LEFT JOIN project_members pm ON pm.project_id = p.id AND pm.user_id = ?1
             WHERE p.created_by = ?1 AND p.name = ?2 AND p.status != 'deleted'
             ORDER BY p.updated_at DESC
             LIMIT 1",
            params![user_id, name],
            project_summary_from_row,
        )
        .optional()?)
}

fn find_project_by_id_for_user(
    conn: &Connection,
    user_id: &str,
    project_id: &str,
) -> Result<Option<ProjectSummary>> {
    Ok(conn
        .query_row(
            "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                    p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id, p.status,
                    pm.role,
                    (SELECT COUNT(*) FROM project_members pm2 WHERE pm2.project_id = p.id) AS member_count,
                    p.is_public,
                    p.join_mode,
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
                    p.icon_data_url,
                    p.updated_at
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id AND pm.user_id = ?2
             WHERE p.id = ?1 AND p.status != 'deleted'
             LIMIT 1",
            params![project_id, user_id],
            project_summary_from_row,
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_store_external_project_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn register_external_project_persists_and_updates_node_binding() {
        let store = temp_store();
        let user = store
            .create_user("pc-project-owner@example.com", "secret1", None, None)
            .expect("user should be created");

        let first = store
            .register_external_project(
                &user.id,
                None,
                "PC Project",
                Some("from pc"),
                r"D:\rust\active-projects\one",
                Some("node-a"),
            )
            .expect("external project should register");
        assert!(!first.reused_existing);
        assert_eq!(first.project.source_type, "local_path");
        assert_eq!(first.project.node_id.as_deref(), Some("node-a"));

        let second = store
            .register_external_project(
                &user.id,
                None,
                "PC Project",
                Some("from pc"),
                r"D:\rust\active-projects\two",
                Some("node-b"),
            )
            .expect("same external project should update");
        assert!(second.reused_existing);
        assert_eq!(
            second.project.workspace_path.as_deref(),
            Some(r"D:\rust\active-projects\two")
        );
        assert_eq!(second.project.node_id.as_deref(), Some("node-b"));

        let access = store
            .get_project_access(&user.id, &second.project.id)
            .expect("project access should include node binding");
        assert_eq!(access.node_id.as_deref(), Some("node-b"));
    }

    #[test]
    fn register_external_project_can_bind_existing_shared_project_by_id() {
        let store = temp_store();
        let owner = store
            .create_user("shared-owner@example.com", "secret1", None, None)
            .expect("user should be created");
        store
            .ensure_project_for_user(
                &owner.id,
                "elon-self",
                "一龙项目",
                None,
                "template",
                "android",
                None,
            )
            .expect("shared project should exist");

        let bound = store
            .register_external_project(
                &owner.id,
                Some("elon-self"),
                "一龙项目",
                Some("PC local repo"),
                r"D:\rust\active-projects\elon cli",
                Some("node-owner"),
            )
            .expect("existing shared project should bind");

        assert!(bound.reused_existing);
        assert_eq!(bound.project.id, "elon-self");
        assert_eq!(
            bound.project.workspace_path.as_deref(),
            Some(r"D:\rust\active-projects\elon cli")
        );
        assert_eq!(bound.project.node_id.as_deref(), Some("node-owner"));
    }

    #[test]
    fn ensure_project_for_user_preserves_pc_bound_workspace_path() {
        let store = temp_store();
        let owner = store
            .create_user("pc-bound-owner@example.com", "secret1", None, None)
            .expect("user should be created");
        store
            .ensure_project_for_user(
                &owner.id,
                "elon-self",
                "一龙项目",
                None,
                "template",
                "android",
                None,
            )
            .expect("shared project should exist");
        store
            .register_external_project(
                &owner.id,
                Some("elon-self"),
                "一龙项目",
                Some("PC local repo"),
                r"D:\rust\active-projects\elon cli",
                Some("node-owner"),
            )
            .expect("existing shared project should bind");

        let ensured = store
            .ensure_project_for_user(
                &owner.id,
                "elon-self",
                "一龙项目",
                Some("server fallback"),
                "local_path",
                "local",
                Some("/opt/elon/data/projects/elon-self"),
            )
            .expect("ensure should keep project accessible");

        assert_eq!(
            ensured.workspace_path.as_deref(),
            Some(r"D:\rust\active-projects\elon cli")
        );
        assert_eq!(ensured.node_id.as_deref(), Some("node-owner"));
    }
}
