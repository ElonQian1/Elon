use crate::store_schema::apply_migrations;
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

mod admin_stats;
mod billing;
mod billing_alerts;
mod billing_price_rules;
#[cfg(test)]
mod billing_reservation_tests;
mod billing_reservations;
mod build_quota;
mod codex_vault;
mod common;
mod compute_metering;
mod conversations;
pub(crate) mod default_joint_projects;
mod external_app_tool_executions;
mod external_apps;
#[cfg(test)]
mod external_apps_tests;
mod friend_messages;
mod friends;
mod group_ai;
mod group_ai_flow;
mod group_ai_governance;
mod group_summary;
#[cfg(test)]
mod group_summary_tests;
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
mod project_branding;
mod project_dev_profiles;
mod project_execution_sessions;
mod project_identities;
mod project_invites;
mod project_landing_snapshots;
mod project_landing_upload_tokens;
mod project_member_audit;
mod project_member_conversations;
mod project_member_moderation;
pub(crate) mod project_releases;
mod project_roles;
mod project_runtime_permissions;
mod project_space;
mod project_storage;
mod project_workspace_health_snapshots;
mod projects;
pub(crate) mod route_c_budget;
mod social_ai_messages;
mod social_ai_pending;
mod social_ai_selected;
mod store_types;
mod system_projects;
mod task_recovery;
mod tasks;
pub(crate) mod token_usage;
#[cfg(test)]
mod token_usage_tests;
mod user_archive;
mod user_memories;
mod user_presence;
mod user_progression;
mod users;
pub use admin_stats::{
    estimate_cost_cny, AdminAccountingAuditRow, AdminDayRow, AdminFeatureRow, AdminModelRow,
    AdminPlatformSummary, AdminTrendRow, AdminUserDetail, AdminUserUsageRow, UserQuota,
};
pub use billing::{AdminBalanceRow, AdminBillingEventRow, BillingEvent, RechargeRecord};
pub use billing_alerts::BillingAlertRow;
pub use billing_price_rules::{BillingPriceRule, BillingPriceRuleUpsert, BillingPriceSnapshot};
pub use billing_reservations::{BillingReservationOutcome, BillingReservationRequest};
pub use codex_vault::CodexVaultRecord;
use common::{
    account_columns, clean_optional, hash_password, hash_token, new_id, normalize_account, now,
    safe_external_id, validate_password, verify_password,
};
pub use compute_metering::ComputeMeterEvent;
pub(crate) use external_app_tool_executions::{
    AdminExternalAppToolExecutionSummary, ExternalAppToolExecutionWrite,
};
pub use node_compute_runs::{
    NodeComputeRun, NodeComputeRunFinish, NodeComputeRunStart, NodeQualityScore,
};
pub use node_ledger::{NodeBalance, NodeCredential, NodeTransaction, SettleParams};
pub use node_payouts::CreateNodePayout;
pub use project_dev_profiles::ProjectDevProfile;
pub use project_execution_sessions::{
    ProjectExecutionSession, ProjectExecutionSessionFinish, ProjectExecutionSessionStart,
};
pub use project_roles::{
    PERMISSION_INVITE_MEMBERS, PERMISSION_MANAGE_MEMBERS, PERMISSION_MANAGE_PROJECT_SETTINGS,
    PERMISSION_MANAGE_ROLES, PERMISSION_MODERATE_MEMBERS, PERMISSION_SEND_MESSAGES,
    PERMISSION_VIEW_AUDIT_LOG, PERMISSION_VIEW_MEMBERS,
};
pub use project_space::{
    CHANNEL_PERMISSION_MANAGE, CHANNEL_PERMISSION_SEND, CHANNEL_PERMISSION_START_AI,
    CHANNEL_PERMISSION_VIEW,
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
pub use user_progression::UserProgressionLedger;

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

        let mut project = ProjectSummary {
            id,
            name: name.to_string(),
            display_name: None,
            description: description.map(ToOwned::to_owned),
            workspace_key,
            template: template.to_string(),
            source_type: "template".into(),
            repo_url: None,
            branch: None,
            workspace_path: None,
            node_id: None,
            storage_node_id: None,
            storage_repo_path: None,
            storage_repo_url: None,
            storage_worktree_path: None,
            storage_status: "none".into(),
            status: "active".into(),
            role: "owner".into(),
            member_count: 1,
            is_public: false,
            join_mode: "open".into(),
            runtime_permission: default_project_runtime_permission(),
            last_task_status: None,
            last_apk_url: None,
            icon_data_url: None,
            updated_at: now,
        };
        project_branding::apply_project_summary_branding(&mut project);

        Ok(CreateProjectResult {
            project,
            reused_existing: false,
        })
    }

    /// 注册一个指向外部本地路径的项目（如 D:\rust\active-projects\bb64a）。
    /// source_type='local_path'，workspace_path 写入项目记录。
    /// 同一代码身份优先复用现有记录（reused_existing=true）。
    pub fn register_external_project(
        &self,
        user_id: &str,
        project_id: Option<&str>,
        name: &str,
        description: Option<&str>,
        workspace_path: &str,
        node_id: Option<&str>,
        repo_url: Option<&str>,
        branch: Option<&str>,
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
        let repo_url = clean_optional(repo_url);
        let branch = clean_optional(branch);

        let now = now();
        let conn = self.conn()?;
        let identity_candidates =
            project_identities::identity_candidates(node_id, workspace_path, repo_url, branch);

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
            if let Some(existing) =
                find_owner_project_by_workspace_path(&conn, user_id, workspace_path)?
            {
                if existing.id != project_id {
                    let display = existing
                        .display_name
                        .as_deref()
                        .unwrap_or(existing.name.as_str());
                    anyhow::bail!("该本地路径已绑定到项目「{}」，请直接打开该项目", display);
                }
            }
            if let Some(existing) = project_identities::find_owner_project_by_identity(
                &conn,
                user_id,
                &identity_candidates,
            )? {
                if existing.id != project_id {
                    return Err(project_identities::identity_conflict_error(&existing));
                }
            }
            if let Some(existing) =
                project_identities::find_owner_project_by_git_remote(&conn, user_id, repo_url)?
            {
                if existing.id != project_id {
                    return Err(project_identities::identity_conflict_error(&existing));
                }
            }

            let project = update_external_project_binding(
                &conn,
                user_id,
                project_id,
                Some(name),
                clean_optional(description),
                workspace_path,
                node_id,
                repo_url,
                branch,
                &now,
                "project_bound_external",
            )?;
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }

        if let Some(project) = project_identities::find_owner_project_by_identity(
            &conn,
            user_id,
            &identity_candidates,
        )? {
            let project = update_external_project_binding(
                &conn,
                user_id,
                &project.id,
                None,
                None,
                workspace_path,
                node_id,
                repo_url,
                branch,
                &now,
                "project_reused_external_identity",
            )?;
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }
        if let Some(project) =
            project_identities::find_owner_project_by_git_remote(&conn, user_id, repo_url)?
        {
            let project = update_external_project_binding(
                &conn,
                user_id,
                &project.id,
                None,
                None,
                workspace_path,
                node_id,
                repo_url,
                branch,
                &now,
                "project_reused_external_git_remote",
            )?;
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }
        if let Some(project) = find_owner_project_by_workspace_path(&conn, user_id, workspace_path)?
        {
            let project = update_external_project_binding(
                &conn,
                user_id,
                &project.id,
                None,
                None,
                workspace_path,
                node_id,
                repo_url,
                branch,
                &now,
                "project_reused_external_path",
            )?;
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }
        if let Some(project) = find_owner_project_by_name(&conn, user_id, name)? {
            let project = update_external_project_binding(
                &conn,
                user_id,
                &project.id,
                None,
                None,
                workspace_path,
                node_id,
                repo_url,
                branch,
                &now,
                "project_reused_external_name",
            )?;
            return Ok(CreateProjectResult {
                project,
                reused_existing: true,
            });
        }

        let id = new_id("prj");
        let workspace_key = id.clone();
        let description = clean_optional(description);
        let create_result = {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "INSERT INTO projects (
                    id, name, description, workspace_key, template, source_type, repo_url, branch,
                    workspace_path, node_id,
                    status, created_by, created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'active', ?11, ?12, ?12)",
                params![
                    id,
                    name,
                    description,
                    workspace_key,
                    template,
                    source_type,
                    repo_url,
                    branch,
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
                        "repo_url": repo_url,
                        "branch": branch,
                    })
                    .to_string(),
                    now
                ],
            )?;
            project_identities::replace_project_identities(
                &tx,
                &id,
                user_id,
                node_id,
                workspace_path,
                repo_url,
                branch,
                &now,
            )?;
            if let Some(node_id) = node_id {
                pc_project_binding::upsert_project_pc_workspace_binding_tx(
                    &tx,
                    &id,
                    user_id,
                    node_id,
                    workspace_path,
                    None,
                    repo_url,
                    branch,
                    "register_external_project",
                    &now,
                )?;
            }
            tx.commit()?;
            Ok::<(), anyhow::Error>(())
        };
        if let Err(err) = create_result {
            if let Some(project) = project_identities::find_owner_project_by_identity(
                &conn,
                user_id,
                &identity_candidates,
            )? {
                let project = update_external_project_binding(
                    &conn,
                    user_id,
                    &project.id,
                    None,
                    None,
                    workspace_path,
                    node_id,
                    repo_url,
                    branch,
                    &now,
                    "project_reused_external_identity",
                )?;
                return Ok(CreateProjectResult {
                    project,
                    reused_existing: true,
                });
            }
            if let Some(project) = find_owner_project_by_name(&conn, user_id, name)? {
                let project = update_external_project_binding(
                    &conn,
                    user_id,
                    &project.id,
                    None,
                    None,
                    workspace_path,
                    node_id,
                    repo_url,
                    branch,
                    &now,
                    "project_reused_external_name",
                )?;
                return Ok(CreateProjectResult {
                    project,
                    reused_existing: true,
                });
            }
            return Err(err);
        }

        let mut project = ProjectSummary {
            id,
            name: name.to_string(),
            display_name: None,
            description: description.map(ToOwned::to_owned),
            workspace_key,
            template: template.to_string(),
            source_type: source_type.into(),
            repo_url: repo_url.map(ToOwned::to_owned),
            branch: branch.map(ToOwned::to_owned),
            workspace_path: Some(workspace_path.to_string()),
            node_id: node_id.map(ToOwned::to_owned),
            storage_node_id: None,
            storage_repo_path: None,
            storage_repo_url: None,
            storage_worktree_path: None,
            storage_status: "none".into(),
            status: "active".into(),
            role: "owner".into(),
            member_count: 1,
            is_public: false,
            join_mode: "open".into(),
            runtime_permission: default_project_runtime_permission(),
            last_task_status: None,
            last_apk_url: None,
            icon_data_url: None,
            updated_at: now,
        };
        project_branding::apply_project_summary_branding(&mut project);

        Ok(CreateProjectResult {
            project,
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

        let user = conn.query_row(
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
        )?;
        drop(conn);
        Ok(user)
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
        if id == "elon-self" {
            tx.execute(
                "UPDATE projects
                 SET is_public = 1,
                     join_mode = 'approval',
                     updated_at = ?2
                 WHERE id = ?1 AND status != 'deleted'",
                params![id, now],
            )?;
        }
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
                    p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id,
                    p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                    p.storage_worktree_path, COALESCE(p.storage_status, 'none'), p.status,
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
                    p.updated_at,
                    COALESCE(
                        (SELECT prp.mode
                           FROM project_runtime_permissions prp
                          WHERE prp.project_id = p.id),
                        'project_write'
                    ) AS runtime_permission,
                    p.display_name
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id
             WHERE pm.user_id = ?1 AND p.status != 'deleted'
             ORDER BY p.updated_at DESC",
        )?;

        let mut projects = stmt
            .query_map(params![user_id], project_summary_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        for project in &mut projects {
            apply_effective_project_summary_role(&conn, user_id, project)?;
            pc_project_binding::apply_user_pc_workspace_binding_to_summary(
                &conn, user_id, project,
            )?;
        }

        Ok(projects)
    }

    pub fn get_project_access(&self, user_id: &str, project_id: &str) -> Result<ProjectAccess> {
        let conn = self.conn()?;
        let mut access = conn
            .query_row(
                "SELECT p.id, p.name, p.workspace_key, p.template, p.source_type, p.repo_url, p.branch,
                        p.workspace_path, p.node_id,
                        p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                        p.storage_worktree_path, COALESCE(p.storage_status, 'none'), pm.role, p.status,
                        COALESCE(
                            (SELECT prp.mode
                               FROM project_runtime_permissions prp
                              WHERE prp.project_id = p.id),
                            'project_write'
                        ) AS runtime_permission
                 FROM projects p
                 JOIN project_members pm ON pm.project_id = p.id
                 WHERE p.id = ?1 AND pm.user_id = ?2 AND p.status != 'deleted'",
                params![project_id, user_id],
                |row| {
                    Ok(ProjectAccess {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        workspace_key: row.get(2)?,
                        template: row.get(3)?,
                        source_type: row.get(4)?,
                        repo_url: row.get(5)?,
                        branch: row.get(6)?,
                        workspace_path: row.get(7)?,
                        node_id: row.get(8)?,
                        storage_node_id: row.get(9)?,
                        storage_repo_path: row.get(10)?,
                        storage_repo_url: row.get(11)?,
                        storage_worktree_path: row.get(12)?,
                        storage_status: row.get(13)?,
                        role: row.get(14)?,
                        status: row.get(15)?,
                        runtime_permission: row.get(16)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"))?;
        if let Some(effective_role) =
            project_roles::project_member_effective_role_locked(&conn, project_id, user_id)?
        {
            access.role = effective_role;
        }
        pc_project_binding::apply_user_pc_workspace_binding_to_access(&conn, user_id, &mut access)?;
        drop(conn);
        if self.project_member_is_banned(project_id, user_id)? {
            anyhow::bail!("你已被该项目封禁，无法访问项目空间");
        }
        Ok(access)
    }

    pub fn get_project_space_access(
        &self,
        user_id: &str,
        project_id: &str,
    ) -> Result<ProjectAccess> {
        if self.project_member_is_banned(project_id, user_id)? {
            anyhow::bail!("你已被该项目封禁，无法访问项目空间");
        }
        if let Ok(access) = self.get_project_access(user_id, project_id) {
            return Ok(access);
        }
        self.conn()?
            .query_row(
                "SELECT p.id, p.name, p.workspace_key, p.template, p.source_type, p.repo_url, p.branch,
                        p.workspace_path, p.node_id,
                        p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                        p.storage_worktree_path, COALESCE(p.storage_status, 'none'), p.status,
                        COALESCE(
                            (SELECT prp.mode
                               FROM project_runtime_permissions prp
                              WHERE prp.project_id = p.id),
                            'project_write'
                        ) AS runtime_permission
                 FROM projects p
                 WHERE p.id = ?1
                   AND p.status != 'deleted'
                   AND p.is_public = 1
                   AND p.join_mode != 'invite'",
                params![project_id],
                |row| {
                    Ok(ProjectAccess {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        workspace_key: row.get(2)?,
                        template: row.get(3)?,
                        source_type: row.get(4)?,
                        repo_url: row.get(5)?,
                        branch: row.get(6)?,
                        workspace_path: row.get(7)?,
                        node_id: row.get(8)?,
                        storage_node_id: row.get(9)?,
                        storage_repo_path: row.get(10)?,
                        storage_repo_url: row.get(11)?,
                        storage_worktree_path: row.get(12)?,
                        storage_status: row.get(13)?,
                        status: row.get(14)?,
                        runtime_permission: row.get(15)?,
                        role: "visitor".to_string(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"))
    }

    pub fn update_project_git_metadata(
        &self,
        user_id: &str,
        project_id: &str,
        repo_url: Option<&str>,
        branch: Option<&str>,
    ) -> Result<ProjectSummary> {
        let repo_url = clean_optional(repo_url);
        let branch = clean_optional(branch);
        if repo_url.is_none() && branch.is_none() {
            let conn = self.conn()?;
            return find_project_by_id_for_user(&conn, user_id, project_id)?
                .ok_or_else(|| anyhow!("项目不存在，或当前用户无权访问"));
        }

        let now = now();
        let conn = self.conn()?;
        let changed = conn.execute(
            "UPDATE projects
             SET repo_url = COALESCE(?1, repo_url),
                 branch = COALESCE(?2, branch),
                 updated_at = ?3
             WHERE id = ?4
               AND source_type NOT IN ('agent_balloon', 'chat_memory')
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
        find_project_by_id_for_user(&conn, user_id, project_id)?
            .ok_or_else(|| anyhow!("Git 配置保存后无法读取项目"))
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
    let mut project = ProjectSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        display_name: row.get(25)?,
        description: row.get(2)?,
        workspace_key: row.get(3)?,
        template: row.get(4)?,
        source_type: row.get(5)?,
        repo_url: row.get(6)?,
        branch: row.get(7)?,
        workspace_path: row.get(8)?,
        node_id: row.get(9)?,
        storage_node_id: row.get(10)?,
        storage_repo_path: row.get(11)?,
        storage_repo_url: row.get(12)?,
        storage_worktree_path: row.get(13)?,
        storage_status: row.get(14)?,
        status: row.get(15)?,
        role: row.get(16)?,
        member_count: row.get(17)?,
        is_public: row.get::<_, i64>(18)? != 0,
        join_mode: row.get(19)?,
        runtime_permission: row.get(24)?,
        last_task_status: row.get(20)?,
        last_apk_url: row.get(21)?,
        icon_data_url: row.get(22)?,
        updated_at: row.get(23)?,
    };
    project_branding::apply_project_summary_branding(&mut project);
    Ok(project)
}

#[allow(clippy::too_many_arguments)]
fn update_external_project_binding(
    conn: &Connection,
    user_id: &str,
    project_id: &str,
    name_override: Option<&str>,
    description_override: Option<&str>,
    workspace_path: &str,
    node_id: Option<&str>,
    repo_url: Option<&str>,
    branch: Option<&str>,
    now: &str,
    event_type: &str,
) -> Result<ProjectSummary> {
    let tx = conn.unchecked_transaction()?;
    let changed = tx.execute(
        "UPDATE projects
         SET name = COALESCE(?2, name),
             description = COALESCE(?3, description),
             template = 'local',
             source_type = 'local_path',
             workspace_path = ?4,
             node_id = ?5,
             repo_url = COALESCE(?6, repo_url),
             branch = COALESCE(?7, branch),
             is_public = CASE WHEN ?1 = 'elon-self' THEN 1 ELSE is_public END,
             join_mode = CASE WHEN ?1 = 'elon-self' THEN 'approval' ELSE join_mode END,
             updated_at = ?8
         WHERE id = ?1 AND status != 'deleted'",
        params![
            project_id,
            name_override,
            description_override,
            workspace_path,
            node_id,
            repo_url,
            branch,
            now
        ],
    )?;
    if changed == 0 {
        anyhow::bail!("项目不存在，或当前用户无权访问");
    }

    tx.execute(
        "INSERT INTO project_events (id, project_id, user_id, event_type, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            new_id("evt"),
            project_id,
            user_id,
            event_type,
            serde_json::json!({
                "workspace_path": workspace_path,
                "node_id": node_id,
                "repo_url": repo_url,
                "branch": branch,
            })
            .to_string(),
            now
        ],
    )?;

    let project = find_project_by_id_for_user(&tx, user_id, project_id)?
        .ok_or_else(|| anyhow!("项目绑定后无法读取"))?;
    project_identities::replace_project_identities(
        &tx,
        project_id,
        user_id,
        project.node_id.as_deref(),
        project.workspace_path.as_deref().unwrap_or(workspace_path),
        project.repo_url.as_deref(),
        project.branch.as_deref(),
        now,
    )?;
    if let Some(node_id) = project.node_id.as_deref() {
        pc_project_binding::upsert_project_pc_workspace_binding_tx(
            &tx,
            project_id,
            user_id,
            node_id,
            project.workspace_path.as_deref().unwrap_or(workspace_path),
            None,
            project.repo_url.as_deref(),
            project.branch.as_deref(),
            event_type,
            now,
        )?;
    }
    tx.commit()?;

    find_project_by_id_for_user(conn, user_id, project_id)?
        .ok_or_else(|| anyhow!("项目绑定后无法读取"))
}

fn find_owner_project_by_name(
    conn: &Connection,
    user_id: &str,
    name: &str,
) -> Result<Option<ProjectSummary>> {
    let mut project = conn
        .query_row(
            "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                    p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id,
                    p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                    p.storage_worktree_path, COALESCE(p.storage_status, 'none'), p.status,
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
                    p.updated_at,
                    COALESCE(
                        (SELECT prp.mode
                           FROM project_runtime_permissions prp
                          WHERE prp.project_id = p.id),
                        'project_write'
                    ) AS runtime_permission,
                    p.display_name
             FROM projects p
             LEFT JOIN project_members pm ON pm.project_id = p.id AND pm.user_id = ?1
             WHERE p.created_by = ?1 AND p.name = ?2 AND p.status != 'deleted'
             ORDER BY p.updated_at DESC
             LIMIT 1",
            params![user_id, name],
            project_summary_from_row,
        )
        .optional()?;
    if let Some(project) = &mut project {
        apply_effective_project_summary_role(conn, user_id, project)?;
    }
    Ok(project)
}

fn find_owner_project_by_workspace_path(
    conn: &Connection,
    user_id: &str,
    workspace_path: &str,
) -> Result<Option<ProjectSummary>> {
    let expected = normalize_workspace_path_for_match(workspace_path);
    if expected.is_empty() {
        return Ok(None);
    }
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id,
                p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                p.storage_worktree_path, COALESCE(p.storage_status, 'none'), p.status,
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
                p.updated_at,
                COALESCE(
                    (SELECT prp.mode
                       FROM project_runtime_permissions prp
                      WHERE prp.project_id = p.id),
                    'project_write'
                ) AS runtime_permission,
                p.display_name
         FROM projects p
         LEFT JOIN project_members pm ON pm.project_id = p.id AND pm.user_id = ?1
         WHERE p.created_by = ?1
           AND p.status != 'deleted'
           AND p.source_type IN ('local_path', 'pc_managed')
           AND p.workspace_path IS NOT NULL
           AND TRIM(p.workspace_path) != ''
         ORDER BY p.updated_at DESC",
    )?;
    let mut rows = stmt.query_map(params![user_id], project_summary_from_row)?;
    while let Some(project) = rows.next() {
        let mut project = project?;
        apply_effective_project_summary_role(conn, user_id, &mut project)?;
        if project
            .workspace_path
            .as_deref()
            .map(normalize_workspace_path_for_match)
            .as_deref()
            == Some(expected.as_str())
        {
            return Ok(Some(project));
        }
    }
    Ok(None)
}

fn normalize_workspace_path_for_match(path: &str) -> String {
    project_identities::normalize_workspace_path(path)
}

fn find_project_by_id_for_user(
    conn: &Connection,
    user_id: &str,
    project_id: &str,
) -> Result<Option<ProjectSummary>> {
    let mut project = conn
        .query_row(
            "SELECT p.id, p.name, p.description, p.workspace_key, p.template,
                    p.source_type, p.repo_url, p.branch, p.workspace_path, p.node_id,
                    p.storage_node_id, p.storage_repo_path, p.storage_repo_url,
                    p.storage_worktree_path, COALESCE(p.storage_status, 'none'), p.status,
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
                    p.updated_at,
                    COALESCE(
                        (SELECT prp.mode
                           FROM project_runtime_permissions prp
                          WHERE prp.project_id = p.id),
                        'project_write'
                    ) AS runtime_permission,
                    p.display_name
             FROM projects p
             JOIN project_members pm ON pm.project_id = p.id AND pm.user_id = ?2
             WHERE p.id = ?1 AND p.status != 'deleted'
             LIMIT 1",
            params![project_id, user_id],
            project_summary_from_row,
        )
        .optional()?;
    if let Some(project) = &mut project {
        apply_effective_project_summary_role(conn, user_id, project)?;
    }
    Ok(project)
}

fn apply_effective_project_summary_role(
    conn: &Connection,
    user_id: &str,
    project: &mut ProjectSummary,
) -> Result<()> {
    if let Some(role) =
        project_roles::project_member_effective_role_locked(conn, &project.id, user_id)?
    {
        project.role = role;
    }
    Ok(())
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
                None,
                None,
            )
            .expect("external project should register");
        assert!(!first.reused_existing);
        assert_eq!(first.project.source_type, "local_path");
        assert_eq!(first.project.node_id.as_deref(), Some("node-a"));

        let same_path = store
            .register_external_project(
                &user.id,
                None,
                "Different Project Name",
                Some("same pc path"),
                r"D:\rust\active-projects\one",
                Some("node-a"),
                None,
                None,
            )
            .expect("same external path should reuse identity");
        assert!(same_path.reused_existing);
        assert_eq!(same_path.project.id, first.project.id);
        assert_eq!(
            same_path.project.workspace_path.as_deref(),
            Some(r"D:\rust\active-projects\one")
        );
        assert_eq!(same_path.project.runtime_permission, "project_write");

        let second = store
            .register_external_project(
                &user.id,
                None,
                "PC Project",
                Some("from pc"),
                r"D:\rust\active-projects\two",
                Some("node-b"),
                Some("git@github.com:owner/pc-project.git"),
                Some("main"),
            )
            .expect("same external project should update");
        assert!(second.reused_existing);
        assert_eq!(
            second.project.workspace_path.as_deref(),
            Some(r"D:\rust\active-projects\two")
        );
        assert_eq!(second.project.node_id.as_deref(), Some("node-b"));
        assert_eq!(
            second.project.repo_url.as_deref(),
            Some("git@github.com:owner/pc-project.git")
        );
        assert_eq!(second.project.branch.as_deref(), Some("main"));

        let access = store
            .get_project_access(&user.id, &second.project.id)
            .expect("project access should include node binding");
        assert_eq!(access.node_id.as_deref(), Some("node-b"));
    }

    #[test]
    fn project_landing_snapshot_is_normalized_and_readable() {
        let store = temp_store();
        let user = store
            .create_user("pc-project-landing@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .register_external_project(
                &user.id,
                None,
                "Landing Project",
                Some("from pc"),
                r"D:\rust\active-projects\landing",
                Some("node-a"),
                None,
                None,
            )
            .expect("external project should register");

        let snapshot = store
            .update_project_landing_snapshot(
                &user.id,
                &project.project.id,
                &serde_json::json!({
                    "title": "Landing Project",
                    "downloads": {
                        "windows": "https://example.com/app.exe",
                        "ios": "javascript:alert(1)"
                    }
                }),
            )
            .expect("landing snapshot should update")
            .expect("landing snapshot should have display content");
        assert_eq!(snapshot["source"]["mode"], "node_agent_snapshot");
        let downloads = snapshot["downloads"].as_array().unwrap();
        let windows = downloads
            .iter()
            .find(|download| download["platform"] == "windows")
            .unwrap();
        let ios = downloads
            .iter()
            .find(|download| download["platform"] == "ios")
            .unwrap();
        assert_eq!(windows["url"], "https://example.com/app.exe");
        assert!(ios.get("url").is_none());

        let loaded = store
            .project_landing_snapshot(&user.id, &project.project.id)
            .expect("landing snapshot should load")
            .expect("landing snapshot should exist");
        assert_eq!(loaded["title"], "Landing Project");
        assert_eq!(loaded["source"]["mode"], "node_agent_snapshot");
    }

    #[test]
    fn project_landing_upload_token_is_project_scoped() {
        let store = temp_store();
        let user = store
            .create_user(
                "pc-project-landing-token@example.com",
                "secret1",
                None,
                None,
            )
            .expect("user should be created");
        let project = store
            .register_external_project(
                &user.id,
                None,
                "Landing Token Project",
                Some("from pc"),
                r"D:\rust\active-projects\landing-token",
                Some("node-a"),
                None,
                None,
            )
            .expect("external project should register");
        let other = store
            .register_external_project(
                &user.id,
                None,
                "Other Landing Token Project",
                Some("from pc"),
                r"D:\rust\active-projects\landing-token-other",
                Some("node-a"),
                None,
                None,
            )
            .expect("other project should register");
        let token = "plt_test_project_scoped_token";

        let record = store
            .rotate_project_landing_upload_token(&project.project.id, &user.id, token)
            .expect("token should rotate");
        assert!(store
            .authenticate_project_landing_upload_token(&other.project.id, token)
            .expect("token auth should run")
            .is_none());
        assert!(store
            .authenticate_project_landing_upload_token(&project.project.id, "wrong-token")
            .expect("token auth should run")
            .is_none());

        let authed = store
            .authenticate_project_landing_upload_token(&project.project.id, token)
            .expect("token auth should run")
            .expect("token should authenticate for its project");
        assert_eq!(authed.id, record.id);
        let snapshot = store
            .update_project_landing_snapshot_with_upload_token(
                &project.project.id,
                &authed.id,
                &serde_json::json!({
                    "title": "Landing Token Project",
                    "release_manifest_url": "https://example.com/project-downloads.json",
                    "downloads": [{
                        "platform": "windows",
                        "status": "available",
                        "url": "https://example.com/app.exe"
                    }]
                }),
            )
            .expect("landing snapshot should update with upload token")
            .expect("snapshot should have display content");
        assert_eq!(
            snapshot["release_manifest_url"],
            "https://example.com/project-downloads.json"
        );
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
                None,
                None,
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
    fn register_external_project_reuses_existing_workspace_path_with_different_name() {
        let store = temp_store();
        let user = store
            .create_user("pc-project-path-owner@example.com", "secret1", None, None)
            .expect("user should be created");
        let workspace_path = r"D:\rust\active-projects\江西吉安商会";

        let first = store
            .register_external_project(
                &user.id,
                None,
                "江西吉安商会",
                None,
                workspace_path,
                Some("node-a"),
                None,
                None,
            )
            .expect("first project should register");
        let second = store
            .register_external_project(
                &user.id,
                None,
                "jxjssh",
                None,
                "D:/rust/active-projects/江西吉安商会/",
                Some("node-b"),
                None,
                None,
            )
            .expect("same workspace path should reuse existing project");

        assert!(second.reused_existing);
        assert_eq!(second.project.id, first.project.id);
        assert_eq!(second.project.name, "江西吉安商会");
        assert_eq!(second.project.node_id.as_deref(), Some("node-b"));
        assert_eq!(
            store
                .get_project_pc_workspace_binding(&user.id, &second.project.id, "node-a")
                .expect("node-a binding lookup")
                .expect("node-a binding")
                .workspace_path,
            workspace_path
        );
        assert_eq!(
            store
                .get_project_pc_workspace_binding(&user.id, &second.project.id, "node-b")
                .expect("node-b binding lookup")
                .expect("node-b binding")
                .workspace_path,
            "D:/rust/active-projects/江西吉安商会/"
        );

        let count: i64 = store
            .conn()
            .expect("db connection")
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE created_by = ?1 AND status != 'deleted'",
                params![user.id],
                |row| row.get(0),
            )
            .expect("project count");
        assert_eq!(count, 1);
    }

    #[test]
    fn register_external_project_reuses_existing_git_remote_with_different_path_and_name() {
        let store = temp_store();
        let user = store
            .create_user("pc-project-git-owner@example.com", "secret1", None, None)
            .expect("user should be created");

        let first = store
            .register_external_project(
                &user.id,
                None,
                "江西吉安商会",
                None,
                r"D:\rust\active-projects\江西吉安商会",
                Some("node-a"),
                Some("git@github.com:Owner/Jiangxi-Jian.git"),
                Some("main"),
            )
            .expect("first project should register");
        let second = store
            .register_external_project(
                &user.id,
                None,
                "本地git项目",
                None,
                r"D:\rust\active-projects\jx-ja-copy",
                Some("node-b"),
                Some("https://github.com/owner/jiangxi-jian"),
                Some("refs/heads/main"),
            )
            .expect("same git remote should reuse existing project");

        assert!(second.reused_existing);
        assert_eq!(second.project.id, first.project.id);
        assert_eq!(second.project.name, "江西吉安商会");
        assert_eq!(
            second.project.workspace_path.as_deref(),
            Some(r"D:\rust\active-projects\jx-ja-copy")
        );
        assert_eq!(
            store
                .get_project_pc_workspace_binding(&user.id, &second.project.id, "node-a")
                .expect("node-a binding lookup")
                .expect("node-a binding")
                .workspace_path,
            r"D:\rust\active-projects\江西吉安商会"
        );
        assert_eq!(
            store
                .get_project_pc_workspace_binding(&user.id, &second.project.id, "node-b")
                .expect("node-b binding lookup")
                .expect("node-b binding")
                .workspace_path,
            r"D:\rust\active-projects\jx-ja-copy"
        );
        let conn = store.conn().expect("db connection");
        let project_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE created_by = ?1 AND status != 'deleted'",
                params![user.id],
                |row| row.get(0),
            )
            .expect("project count");
        let identity_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_identities WHERE project_id = ?1",
                params![second.project.id],
                |row| row.get(0),
            )
            .expect("identity count");
        assert_eq!(project_count, 1);
        assert_eq!(identity_count, 4);
    }

    #[test]
    fn register_external_project_prefers_existing_path_over_same_name() {
        let store = temp_store();
        let user = store
            .create_user(
                "pc-project-path-priority@example.com",
                "secret1",
                None,
                None,
            )
            .expect("user should be created");
        let project_a = store
            .register_external_project(
                &user.id,
                None,
                "项目A",
                None,
                r"D:\rust\active-projects\a",
                Some("node-a"),
                None,
                None,
            )
            .expect("project A should register");
        let project_b = store
            .register_external_project(
                &user.id,
                None,
                "项目B",
                None,
                r"D:\rust\active-projects\b",
                Some("node-b"),
                None,
                None,
            )
            .expect("project B should register");

        let reused = store
            .register_external_project(
                &user.id,
                None,
                "项目A",
                None,
                r"D:/rust/active-projects/b/",
                Some("node-c"),
                None,
                None,
            )
            .expect("path match should win over name match");

        assert!(reused.reused_existing);
        assert_eq!(reused.project.id, project_b.project.id);
        assert_eq!(reused.project.name, "项目B");
        assert_ne!(reused.project.id, project_a.project.id);
        let project_count: i64 = store
            .conn()
            .expect("db connection")
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE created_by = ?1 AND status != 'deleted'",
                params![user.id],
                |row| row.get(0),
            )
            .expect("project count");
        assert_eq!(project_count, 2);
    }

    #[test]
    fn register_external_project_rejects_binding_to_path_owned_by_another_project() {
        let store = temp_store();
        let user = store
            .create_user(
                "pc-project-path-conflict@example.com",
                "secret1",
                None,
                None,
            )
            .expect("user should be created");
        let workspace_path = r"D:\rust\active-projects\江西吉安商会";
        let existing = store
            .register_external_project(
                &user.id,
                None,
                "江西吉安商会",
                None,
                workspace_path,
                Some("node-a"),
                None,
                None,
            )
            .expect("first project should register");
        let other = store
            .ensure_project_for_user(
                &user.id,
                "prj-other",
                "其他项目",
                None,
                "template",
                "android",
                None,
            )
            .expect("other project should exist");

        let err = store
            .register_external_project(
                &user.id,
                Some(&other.id),
                "其他项目",
                None,
                workspace_path,
                Some("node-b"),
                None,
                None,
            )
            .expect_err("binding duplicate workspace path should fail");

        assert!(err.to_string().contains("该本地路径已绑定到项目"));
        assert!(err.to_string().contains(&existing.project.name));
    }

    #[test]
    fn register_external_project_rejects_binding_to_git_identity_owned_by_another_project() {
        let store = temp_store();
        let user = store
            .create_user("pc-project-git-conflict@example.com", "secret1", None, None)
            .expect("user should be created");
        let existing = store
            .register_external_project(
                &user.id,
                None,
                "江西吉安商会",
                None,
                r"D:\rust\active-projects\江西吉安商会",
                Some("node-a"),
                Some("git@github.com:Owner/Jiangxi-Jian.git"),
                Some("main"),
            )
            .expect("first project should register");
        let other = store
            .ensure_project_for_user(
                &user.id,
                "prj-other-git",
                "其他项目",
                None,
                "template",
                "android",
                None,
            )
            .expect("other project should exist");

        let err = store
            .register_external_project(
                &user.id,
                Some(&other.id),
                "其他项目",
                None,
                r"D:\rust\active-projects\other",
                Some("node-b"),
                Some("https://github.com/owner/jiangxi-jian.git"),
                Some("main"),
            )
            .expect_err("binding duplicate git remote should fail");

        assert!(err.to_string().contains("该代码项目已绑定到项目"));
        assert!(err.to_string().contains(&existing.project.name));
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
                None,
                None,
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
